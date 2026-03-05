use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use reqwest::{Client, Method, StatusCode};
use serde_json::Value;

use crate::types::{invalid_data_error, Error};

const API_BASE: &str = "https://discord.com/api/v10";

pub struct DiscordHttpClient {
    client: Client,
    token: String,
    application_id: AtomicU64,
}

impl DiscordHttpClient {
    pub fn new(token: impl Into<String>, application_id: u64) -> Self {
        Self {
            client: Client::new(),
            token: token.into(),
            application_id: AtomicU64::new(application_id),
        }
    }

    pub fn application_id(&self) -> u64 {
        self.application_id.load(Ordering::Relaxed)
    }

    pub fn set_application_id(&self, application_id: u64) {
        self.application_id.store(application_id, Ordering::Relaxed);
    }

    pub async fn send_message(&self, channel_id: u64, body: &Value) -> Result<Value, Error> {
        self.request(Method::POST, &format!("/channels/{channel_id}/messages"), Some(body))
            .await
    }

    pub async fn edit_message(
        &self,
        channel_id: u64,
        message_id: u64,
        body: &Value,
    ) -> Result<Value, Error> {
        self.request(
            Method::PATCH,
            &format!("/channels/{channel_id}/messages/{message_id}"),
            Some(body),
        )
        .await
    }

    pub async fn delete_message(&self, channel_id: u64, message_id: u64) -> Result<(), Error> {
        self.request(
            Method::DELETE,
            &format!("/channels/{channel_id}/messages/{message_id}"),
            None,
        )
        .await?;
        Ok(())
    }

    pub async fn create_dm_channel(&self, user_id: u64) -> Result<Value, Error> {
        let body = serde_json::json!({ "recipient_id": user_id.to_string() });
        self.request(Method::POST, "/users/@me/channels", Some(&body))
            .await
    }

    pub async fn create_interaction_response(
        &self,
        interaction_id: &str,
        interaction_token: &str,
        body: &Value,
    ) -> Result<(), Error> {
        self.request(
            Method::POST,
            &format!("/interactions/{interaction_id}/{interaction_token}/callback"),
            Some(body),
        )
        .await?;
        Ok(())
    }

    pub async fn create_followup_message(
        &self,
        interaction_token: &str,
        body: &Value,
    ) -> Result<Value, Error> {
        self.request(
            Method::POST,
            &format!(
                "/webhooks/{}/{interaction_token}",
                self.application_id()
            ),
            Some(body),
        )
        .await
    }

    pub async fn edit_followup_message(
        &self,
        interaction_token: &str,
        message_id: &str,
        body: &Value,
    ) -> Result<Value, Error> {
        self.request(
            Method::PATCH,
            &format!(
                "/webhooks/{}/{interaction_token}/messages/{message_id}",
                self.application_id()
            ),
            Some(body),
        )
        .await
    }

    pub async fn bulk_overwrite_global_commands(
        &self,
        commands: Vec<Value>,
    ) -> Result<Vec<Value>, Error> {
        let body = Value::Array(commands);
        let response = self
            .request(
                Method::PUT,
                &format!("/applications/{}/commands", self.application_id()),
                Some(&body),
            )
            .await?;

        match response {
            Value::Array(commands) => Ok(commands),
            _ => Err(invalid_data_error(
                "discord api returned unexpected payload for bulk command overwrite",
            )),
        }
    }

    pub async fn request(&self, method: Method, path: &str, body: Option<&Value>) -> Result<Value, Error> {
        let mut retried_after_429 = false;
        loop {
            let (status, response_text) = self
                .request_once(method.clone(), path, body)
                .await?;

            if status == StatusCode::TOO_MANY_REQUESTS {
                let payload = parse_body_value(response_text);
                if retried_after_429 {
                    return Err(invalid_data_error(format!(
                        "discord api rate limit exceeded after retry for {path}: {payload}"
                    )));
                }

                let retry_after = payload
                    .get("retry_after")
                    .and_then(Value::as_f64)
                    .or_else(|| payload.get("retry_after").and_then(Value::as_u64).map(|v| v as f64))
                    .unwrap_or(1.0);

                sleep_for_retry_after(retry_after);
                retried_after_429 = true;
                continue;
            }

            if !status.is_success() {
                let payload = parse_body_value(response_text);
                return Err(invalid_data_error(format!(
                    "discord api request failed ({status}) {path}: {payload}"
                )));
            }

            return Ok(parse_body_value(response_text));
        }
    }

    async fn request_once(
        &self,
        method: Method,
        path: &str,
        body: Option<&Value>,
    ) -> Result<(StatusCode, String), Error> {
        let normalized_path = if path.starts_with('/') {
            path.to_string()
        } else {
            format!("/{path}")
        };
        let url = format!("{API_BASE}{normalized_path}");

        let mut request_builder = self
            .client
            .request(method, url)
            .header("Authorization", format!("Bot {}", self.token))
            .header("Content-Type", "application/json")
            .header("User-Agent", "DiscordBot (discordrs, 0.3.0)");

        if let Some(body) = body {
            request_builder = request_builder.json(body);
        }

        let response = request_builder.send().await?;
        let status = response.status();
        let response_text = response.text().await?;

        Ok((status, response_text))
    }
}

fn parse_body_value(response_text: String) -> Value {
    if response_text.is_empty() {
        Value::Null
    } else {
        serde_json::from_str(&response_text).unwrap_or(Value::String(response_text))
    }
}

fn sleep_for_retry_after(retry_after_seconds: f64) {
    std::thread::sleep(Duration::from_secs_f64(retry_after_seconds.max(0.0)));
}
