use std::collections::{HashMap, HashSet};
use std::str::FromStr;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use reqwest::{header::HeaderMap, StatusCode};

use super::body::{header_string, parse_body_value};

pub(crate) const RATE_LIMIT_BUCKET_RETENTION: Duration = Duration::from_secs(60 * 60);

#[derive(Default)]
pub(crate) struct RateLimitState {
    pub(crate) route_buckets: Mutex<HashMap<String, String>>,
    pub(crate) bucket_last_seen: Mutex<HashMap<String, Instant>>,
    pub(crate) blocked_until: Mutex<HashMap<String, Instant>>,
    pub(crate) global_blocked_until: Mutex<Option<Instant>>,
}

impl RateLimitState {
    pub(crate) fn wait_duration(&self, route_key: &str) -> Option<Duration> {
        let now = Instant::now();
        self.cleanup_old_buckets(now);
        if let Some(global_until) = *self
            .global_blocked_until
            .lock()
            .expect("global rate limit mutex poisoned")
        {
            if global_until > now {
                return Some(global_until.duration_since(now));
            }
        }

        let blocked_until = self
            .blocked_until
            .lock()
            .expect("route rate limit mutex poisoned");
        let route_bucket_key = self
            .route_buckets
            .lock()
            .expect("route bucket mutex poisoned")
            .get(route_key)
            .cloned()
            .unwrap_or_else(|| route_key.to_string());

        blocked_until
            .get(&route_bucket_key)
            .copied()
            .and_then(|until| {
                if until > now {
                    Some(until.duration_since(now))
                } else {
                    None
                }
            })
    }

    pub(crate) fn observe(
        &self,
        route_key: &str,
        headers: &HeaderMap,
        status: StatusCode,
        body: &str,
    ) {
        let now = Instant::now();
        self.cleanup_old_buckets(now);
        if let Some(bucket_id) = header_string(headers.get("x-ratelimit-bucket")) {
            self.route_buckets
                .lock()
                .expect("route bucket mutex poisoned")
                .insert(route_key.to_string(), bucket_id.clone());
            self.bucket_last_seen
                .lock()
                .expect("bucket last-seen mutex poisoned")
                .insert(bucket_id.clone(), now);
        }

        if status == StatusCode::TOO_MANY_REQUESTS {
            let payload = parse_body_value(body.to_string());
            let retry_after = payload
                .get("retry_after")
                .and_then(serde_json::Value::as_f64)
                .unwrap_or(1.0);
            let blocked_until = now + Duration::from_secs_f64(retry_after.max(0.0));

            if payload
                .get("global")
                .and_then(serde_json::Value::as_bool)
                .unwrap_or(false)
            {
                *self
                    .global_blocked_until
                    .lock()
                    .expect("global rate limit mutex poisoned") = Some(blocked_until);
            } else {
                self.block_key(route_key, headers, blocked_until);
            }
            return;
        }

        let remaining = header_string(headers.get("x-ratelimit-remaining"))
            .and_then(|value| value.parse::<u64>().ok());
        let reset_after = header_string(headers.get("x-ratelimit-reset-after"))
            .and_then(|value| f64::from_str(&value).ok())
            .map(Duration::from_secs_f64);

        if remaining == Some(0) {
            if let Some(reset_after) = reset_after {
                self.block_key(route_key, headers, now + reset_after);
            }
        }
    }

    fn block_key(&self, route_key: &str, headers: &HeaderMap, blocked_until: Instant) {
        let bucket_key = header_string(headers.get("x-ratelimit-bucket"))
            .or_else(|| {
                self.route_buckets
                    .lock()
                    .expect("route bucket mutex poisoned")
                    .get(route_key)
                    .cloned()
            })
            .unwrap_or_else(|| route_key.to_string());

        self.blocked_until
            .lock()
            .expect("route rate limit mutex poisoned")
            .insert(bucket_key.clone(), blocked_until);
        self.bucket_last_seen
            .lock()
            .expect("bucket last-seen mutex poisoned")
            .insert(bucket_key, Instant::now());
    }

    pub(crate) fn cleanup_old_buckets(&self, now: Instant) {
        {
            let mut global = self
                .global_blocked_until
                .lock()
                .expect("global rate limit mutex poisoned");
            if global.is_some_and(|until| until <= now) {
                *global = None;
            }
        }

        {
            let mut blocked_until = self
                .blocked_until
                .lock()
                .expect("route rate limit mutex poisoned");
            blocked_until.retain(|_, until| *until > now);
        }

        let stale_buckets = {
            let mut bucket_last_seen = self
                .bucket_last_seen
                .lock()
                .expect("bucket last-seen mutex poisoned");
            let stale = bucket_last_seen
                .iter()
                .filter_map(|(bucket, last_seen)| {
                    (now.saturating_duration_since(*last_seen) >= RATE_LIMIT_BUCKET_RETENTION)
                        .then_some(bucket.clone())
                })
                .collect::<HashSet<_>>();
            bucket_last_seen.retain(|bucket, _| !stale.contains(bucket));
            stale
        };

        if !stale_buckets.is_empty() {
            self.route_buckets
                .lock()
                .expect("route bucket mutex poisoned")
                .retain(|_, bucket| !stale_buckets.contains(bucket));
        }
    }
}
