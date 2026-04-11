use std::collections::HashMap;
use std::future::poll_fn;
use std::str::FromStr;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::task::{Poll, Waker};
use std::time::{Duration, Instant};

use reqwest::{
    header::{HeaderMap, HeaderValue},
    Client, Method, StatusCode,
};
use serde::de::DeserializeOwned;
use serde_json::Value;
use tracing::{debug, warn};

use crate::command::CommandDefinition;
use crate::error::DiscordError;
use crate::model::{
    ApplicationCommand, Channel, CreateDmChannel, CreateMessage, GatewayBot, Guild,
    InteractionCallbackResponse, Member, Message, Role, Snowflake,
};
use crate::types::invalid_data_error;

const API_BASE: &str = "https://discord.com/api/v10";

pub struct RestClient {
    client: Client,
    token: String,
    application_id: AtomicU64,
    rate_limits: Arc<RateLimitState>,
}

pub type DiscordHttpClient = RestClient;

impl RestClient {
    pub fn new(token: impl Into<String>, application_id: u64) -> Self {
        Self {
            client: Client::new(),
            token: token.into(),
            application_id: AtomicU64::new(application_id),
            rate_limits: Arc::new(RateLimitState::default()),
        }
    }

    pub fn application_id(&self) -> u64 {
        self.application_id.load(Ordering::Relaxed)
    }

    pub fn set_application_id(&self, application_id: u64) {
        self.application_id.store(application_id, Ordering::Relaxed);
    }

    pub async fn create_message(
        &self,
        channel_id: impl Into<Snowflake>,
        body: &CreateMessage,
    ) -> Result<Message, DiscordError> {
        self.request_typed(
            Method::POST,
            &format!("/channels/{}/messages", channel_id.into()),
            Some(body),
        )
        .await
    }

    pub async fn update_message(
        &self,
        channel_id: impl Into<Snowflake>,
        message_id: impl Into<Snowflake>,
        body: &CreateMessage,
    ) -> Result<Message, DiscordError> {
        self.request_typed(
            Method::PATCH,
            &format!(
                "/channels/{}/messages/{}",
                channel_id.into(),
                message_id.into()
            ),
            Some(body),
        )
        .await
    }

    pub async fn get_message(
        &self,
        channel_id: impl Into<Snowflake>,
        message_id: impl Into<Snowflake>,
    ) -> Result<Message, DiscordError> {
        self.request_typed(
            Method::GET,
            &format!(
                "/channels/{}/messages/{}",
                channel_id.into(),
                message_id.into()
            ),
            Option::<&Value>::None,
        )
        .await
    }

    pub async fn get_channel(
        &self,
        channel_id: impl Into<Snowflake>,
    ) -> Result<Channel, DiscordError> {
        self.request_typed(
            Method::GET,
            &format!("/channels/{}", channel_id.into()),
            Option::<&Value>::None,
        )
        .await
    }

    pub async fn delete_channel(
        &self,
        channel_id: impl Into<Snowflake>,
    ) -> Result<Channel, DiscordError> {
        self.request_typed(
            Method::DELETE,
            &format!("/channels/{}", channel_id.into()),
            Option::<&Value>::None,
        )
        .await
    }

    pub async fn update_channel(
        &self,
        channel_id: impl Into<Snowflake>,
        body: &Value,
    ) -> Result<Channel, DiscordError> {
        self.request_typed(
            Method::PATCH,
            &format!("/channels/{}", channel_id.into()),
            Some(body),
        )
        .await
    }

    pub async fn get_channel_messages(
        &self,
        channel_id: impl Into<Snowflake>,
        limit: Option<u64>,
    ) -> Result<Vec<Message>, DiscordError> {
        let path = match limit {
            Some(l) => format!("/channels/{}/messages?limit={}", channel_id.into(), l),
            None => format!("/channels/{}/messages", channel_id.into()),
        };
        self.request_typed(Method::GET, &path, Option::<&Value>::None)
            .await
    }

    pub async fn bulk_delete_messages(
        &self,
        channel_id: impl Into<Snowflake>,
        message_ids: Vec<Snowflake>,
    ) -> Result<(), DiscordError> {
        let body = serde_json::json!({ "messages": message_ids.iter().map(|id| id.as_str()).collect::<Vec<_>>() });
        self.request_no_content(
            Method::POST,
            &format!("/channels/{}/messages/bulk-delete", channel_id.into()),
            Some(&body),
        )
        .await
    }

    pub async fn add_reaction(
        &self,
        channel_id: impl Into<Snowflake>,
        message_id: impl Into<Snowflake>,
        emoji: &str,
    ) -> Result<(), DiscordError> {
        let path = format!(
            "/channels/{}/messages/{}/reactions/{}/@me",
            channel_id.into(),
            message_id.into(),
            emoji
        );
        self.request_no_content(Method::PUT, &path, Option::<&Value>::None)
            .await
    }

    pub async fn remove_reaction(
        &self,
        channel_id: impl Into<Snowflake>,
        message_id: impl Into<Snowflake>,
        emoji: &str,
    ) -> Result<(), DiscordError> {
        let path = format!(
            "/channels/{}/messages/{}/reactions/{}/@me",
            channel_id.into(),
            message_id.into(),
            emoji
        );
        self.request_no_content(Method::DELETE, &path, Option::<&Value>::None)
            .await
    }

    pub async fn get_guild(&self, guild_id: impl Into<Snowflake>) -> Result<Guild, DiscordError> {
        self.request_typed(
            Method::GET,
            &format!("/guilds/{}", guild_id.into()),
            Option::<&Value>::None,
        )
        .await
    }

    pub async fn update_guild(
        &self,
        guild_id: impl Into<Snowflake>,
        body: &Value,
    ) -> Result<Guild, DiscordError> {
        self.request_typed(
            Method::PATCH,
            &format!("/guilds/{}", guild_id.into()),
            Some(body),
        )
        .await
    }

    pub async fn get_guild_channels(
        &self,
        guild_id: impl Into<Snowflake>,
    ) -> Result<Vec<Channel>, DiscordError> {
        self.request_typed(
            Method::GET,
            &format!("/guilds/{}/channels", guild_id.into()),
            Option::<&Value>::None,
        )
        .await
    }

    pub async fn create_guild_channel(
        &self,
        guild_id: impl Into<Snowflake>,
        body: &Value,
    ) -> Result<Channel, DiscordError> {
        self.request_typed(
            Method::POST,
            &format!("/guilds/{}/channels", guild_id.into()),
            Some(body),
        )
        .await
    }

    pub async fn get_guild_members(
        &self,
        guild_id: impl Into<Snowflake>,
        limit: Option<u64>,
    ) -> Result<Vec<Member>, DiscordError> {
        let path = match limit {
            Some(l) => format!("/guilds/{}/members?limit={}", guild_id.into(), l),
            None => format!("/guilds/{}/members", guild_id.into()),
        };
        self.request_typed(Method::GET, &path, Option::<&Value>::None)
            .await
    }

    pub async fn remove_guild_member(
        &self,
        guild_id: impl Into<Snowflake>,
        user_id: impl Into<Snowflake>,
    ) -> Result<(), DiscordError> {
        self.request_no_content(
            Method::DELETE,
            &format!("/guilds/{}/members/{}", guild_id.into(), user_id.into()),
            Option::<&Value>::None,
        )
        .await
    }

    pub async fn add_guild_member_role(
        &self,
        guild_id: impl Into<Snowflake>,
        user_id: impl Into<Snowflake>,
        role_id: impl Into<Snowflake>,
    ) -> Result<(), DiscordError> {
        self.request_no_content(
            Method::PUT,
            &format!(
                "/guilds/{}/members/{}/roles/{}",
                guild_id.into(),
                user_id.into(),
                role_id.into()
            ),
            Option::<&Value>::None,
        )
        .await
    }

    pub async fn remove_guild_member_role(
        &self,
        guild_id: impl Into<Snowflake>,
        user_id: impl Into<Snowflake>,
        role_id: impl Into<Snowflake>,
    ) -> Result<(), DiscordError> {
        self.request_no_content(
            Method::DELETE,
            &format!(
                "/guilds/{}/members/{}/roles/{}",
                guild_id.into(),
                user_id.into(),
                role_id.into()
            ),
            Option::<&Value>::None,
        )
        .await
    }

    pub async fn create_role(
        &self,
        guild_id: impl Into<Snowflake>,
        body: &Value,
    ) -> Result<Role, DiscordError> {
        self.request_typed(
            Method::POST,
            &format!("/guilds/{}/roles", guild_id.into()),
            Some(body),
        )
        .await
    }

    pub async fn update_role(
        &self,
        guild_id: impl Into<Snowflake>,
        role_id: impl Into<Snowflake>,
        body: &Value,
    ) -> Result<Role, DiscordError> {
        self.request_typed(
            Method::PATCH,
            &format!("/guilds/{}/roles/{}", guild_id.into(), role_id.into()),
            Some(body),
        )
        .await
    }

    pub async fn delete_role(
        &self,
        guild_id: impl Into<Snowflake>,
        role_id: impl Into<Snowflake>,
    ) -> Result<(), DiscordError> {
        self.request_no_content(
            Method::DELETE,
            &format!("/guilds/{}/roles/{}", guild_id.into(), role_id.into()),
            Option::<&Value>::None,
        )
        .await
    }

    pub async fn get_member(
        &self,
        guild_id: impl Into<Snowflake>,
        user_id: impl Into<Snowflake>,
    ) -> Result<Member, DiscordError> {
        self.request_typed(
            Method::GET,
            &format!("/guilds/{}/members/{}", guild_id.into(), user_id.into()),
            Option::<&Value>::None,
        )
        .await
    }

    pub async fn list_roles(
        &self,
        guild_id: impl Into<Snowflake>,
    ) -> Result<Vec<Role>, DiscordError> {
        self.request_typed(
            Method::GET,
            &format!("/guilds/{}/roles", guild_id.into()),
            Option::<&Value>::None,
        )
        .await
    }

    pub async fn create_webhook(
        &self,
        channel_id: impl Into<Snowflake>,
        body: &Value,
    ) -> Result<Value, DiscordError> {
        self.request(
            Method::POST,
            &format!("/channels/{}/webhooks", channel_id.into()),
            Some(body),
        )
        .await
    }

    pub async fn get_channel_webhooks(
        &self,
        channel_id: impl Into<Snowflake>,
    ) -> Result<Vec<Value>, DiscordError> {
        let response = self
            .request(
                Method::GET,
                &format!("/channels/{}/webhooks", channel_id.into()),
                Option::<&Value>::None,
            )
            .await?;
        match response {
            Value::Array(webhooks) => Ok(webhooks),
            _ => Ok(vec![]),
        }
    }

    pub async fn execute_webhook(
        &self,
        webhook_id: impl Into<Snowflake>,
        token: &str,
        body: &Value,
    ) -> Result<Value, DiscordError> {
        let path = execute_webhook_path(webhook_id.into(), token)?;
        self.request(Method::POST, &path, Some(body)).await
    }

    pub async fn create_dm_channel_typed(
        &self,
        body: &CreateDmChannel,
    ) -> Result<Channel, DiscordError> {
        self.request_typed(Method::POST, "/users/@me/channels", Some(body))
            .await
    }

    pub async fn create_interaction_response_typed(
        &self,
        interaction_id: impl Into<Snowflake>,
        interaction_token: &str,
        body: &InteractionCallbackResponse,
    ) -> Result<(), DiscordError> {
        let path = interaction_callback_path(interaction_id.into(), interaction_token)?;
        self.request_no_content(Method::POST, &path, Some(body))
            .await
    }

    pub async fn bulk_overwrite_global_commands_typed(
        &self,
        commands: &[CommandDefinition],
    ) -> Result<Vec<ApplicationCommand>, DiscordError> {
        let path = global_commands_path(self.application_id())?;
        self.request_typed(Method::PUT, &path, Some(commands)).await
    }

    pub async fn create_global_command(
        &self,
        command: &CommandDefinition,
    ) -> Result<ApplicationCommand, DiscordError> {
        let path = global_commands_path(self.application_id())?;
        self.request_typed(Method::POST, &path, Some(command)).await
    }

    pub async fn get_global_commands(&self) -> Result<Vec<ApplicationCommand>, DiscordError> {
        let path = global_commands_path(self.application_id())?;
        self.request_typed(Method::GET, &path, Option::<&Value>::None)
            .await
    }

    pub async fn get_gateway_bot(&self) -> Result<GatewayBot, DiscordError> {
        self.request_typed(Method::GET, "/gateway/bot", Option::<&Value>::None)
            .await
    }

    pub async fn bulk_overwrite_guild_commands_typed(
        &self,
        guild_id: impl Into<Snowflake>,
        commands: &[CommandDefinition],
    ) -> Result<Vec<ApplicationCommand>, DiscordError> {
        let application_id = configured_application_id(self.application_id())?;
        self.request_typed(
            Method::PUT,
            &format!(
                "/applications/{application_id}/guilds/{}/commands",
                guild_id.into()
            ),
            Some(commands),
        )
        .await
    }

    pub(crate) async fn send_message_json(
        &self,
        channel_id: impl Into<Snowflake>,
        body: &Value,
    ) -> Result<Value, DiscordError> {
        self.request(
            Method::POST,
            &format!("/channels/{}/messages", channel_id.into()),
            Some(body),
        )
        .await
    }

    pub(crate) async fn edit_message_json(
        &self,
        channel_id: impl Into<Snowflake>,
        message_id: impl Into<Snowflake>,
        body: &Value,
    ) -> Result<Value, DiscordError> {
        self.request(
            Method::PATCH,
            &format!(
                "/channels/{}/messages/{}",
                channel_id.into(),
                message_id.into()
            ),
            Some(body),
        )
        .await
    }

    pub async fn delete_message(
        &self,
        channel_id: impl Into<Snowflake>,
        message_id: impl Into<Snowflake>,
    ) -> Result<(), DiscordError> {
        self.request_no_content(
            Method::DELETE,
            &format!(
                "/channels/{}/messages/{}",
                channel_id.into(),
                message_id.into()
            ),
            Option::<&Value>::None,
        )
        .await
    }

    pub(crate) async fn create_interaction_response_json(
        &self,
        interaction_id: impl Into<Snowflake>,
        interaction_token: &str,
        body: &Value,
    ) -> Result<(), DiscordError> {
        let path = interaction_callback_path(interaction_id.into(), interaction_token)?;
        self.request_no_content(Method::POST, &path, Some(body))
            .await
    }

    pub(crate) async fn create_followup_message_json(
        &self,
        interaction_token: &str,
        body: &Value,
    ) -> Result<Value, DiscordError> {
        let application_id = configured_application_id(self.application_id())?;
        self.create_followup_message_json_with_application_id(
            &application_id,
            interaction_token,
            body,
        )
        .await
    }

    pub(crate) async fn create_followup_message_json_with_application_id(
        &self,
        application_id: &str,
        interaction_token: &str,
        body: &Value,
    ) -> Result<Value, DiscordError> {
        let path = followup_webhook_path(application_id, interaction_token, None)?;
        self.request(Method::POST, &path, Some(body)).await
    }

    pub async fn create_followup_message(
        &self,
        interaction_token: &str,
        body: &CreateMessage,
    ) -> Result<Message, DiscordError> {
        let application_id = configured_application_id(self.application_id())?;
        self.create_followup_message_with_application_id(&application_id, interaction_token, body)
            .await
    }

    pub async fn create_followup_message_with_application_id(
        &self,
        application_id: &str,
        interaction_token: &str,
        body: &CreateMessage,
    ) -> Result<Message, DiscordError> {
        let path = followup_webhook_path(application_id, interaction_token, None)?;
        self.request_typed(Method::POST, &path, Some(body)).await
    }

    pub async fn get_original_interaction_response(
        &self,
        interaction_token: &str,
    ) -> Result<Message, DiscordError> {
        let application_id = configured_application_id(self.application_id())?;
        self.get_original_interaction_response_with_application_id(
            &application_id,
            interaction_token,
        )
        .await
    }

    pub async fn get_original_interaction_response_with_application_id(
        &self,
        application_id: &str,
        interaction_token: &str,
    ) -> Result<Message, DiscordError> {
        let path = followup_webhook_path(application_id, interaction_token, Some("@original"))?;
        self.request_typed(Method::GET, &path, Option::<&Value>::None)
            .await
    }

    pub async fn edit_original_interaction_response(
        &self,
        interaction_token: &str,
        body: &CreateMessage,
    ) -> Result<Message, DiscordError> {
        let application_id = configured_application_id(self.application_id())?;
        self.edit_original_interaction_response_with_application_id(
            &application_id,
            interaction_token,
            body,
        )
        .await
    }

    pub async fn edit_original_interaction_response_with_application_id(
        &self,
        application_id: &str,
        interaction_token: &str,
        body: &CreateMessage,
    ) -> Result<Message, DiscordError> {
        let path = followup_webhook_path(application_id, interaction_token, Some("@original"))?;
        self.request_typed(Method::PATCH, &path, Some(body)).await
    }

    pub async fn delete_original_interaction_response(
        &self,
        interaction_token: &str,
    ) -> Result<(), DiscordError> {
        let application_id = configured_application_id(self.application_id())?;
        self.delete_original_interaction_response_with_application_id(
            &application_id,
            interaction_token,
        )
        .await
    }

    pub async fn delete_original_interaction_response_with_application_id(
        &self,
        application_id: &str,
        interaction_token: &str,
    ) -> Result<(), DiscordError> {
        let path = followup_webhook_path(application_id, interaction_token, Some("@original"))?;
        self.request_no_content(Method::DELETE, &path, Option::<&Value>::None)
            .await
    }

    pub async fn edit_followup_message(
        &self,
        interaction_token: &str,
        message_id: &str,
        body: &CreateMessage,
    ) -> Result<Message, DiscordError> {
        let application_id = configured_application_id(self.application_id())?;
        self.edit_followup_message_with_application_id(
            &application_id,
            interaction_token,
            message_id,
            body,
        )
        .await
    }

    pub async fn edit_followup_message_with_application_id(
        &self,
        application_id: &str,
        interaction_token: &str,
        message_id: &str,
        body: &CreateMessage,
    ) -> Result<Message, DiscordError> {
        let path = followup_webhook_path(application_id, interaction_token, Some(message_id))?;
        self.request_typed(Method::PATCH, &path, Some(body)).await
    }

    pub async fn delete_followup_message(
        &self,
        interaction_token: &str,
        message_id: &str,
    ) -> Result<(), DiscordError> {
        let application_id = configured_application_id(self.application_id())?;
        self.delete_followup_message_with_application_id(
            &application_id,
            interaction_token,
            message_id,
        )
        .await
    }

    pub async fn delete_followup_message_with_application_id(
        &self,
        application_id: &str,
        interaction_token: &str,
        message_id: &str,
    ) -> Result<(), DiscordError> {
        let path = followup_webhook_path(application_id, interaction_token, Some(message_id))?;
        self.request_no_content(Method::DELETE, &path, Option::<&Value>::None)
            .await
    }

    pub async fn request(
        &self,
        method: Method,
        path: &str,
        body: Option<&Value>,
    ) -> Result<Value, DiscordError> {
        let response = self
            .request_with_headers(method, path, body.map(clone_json_body))
            .await?;
        Ok(parse_body_value(response.body))
    }

    pub async fn request_typed<T, B>(
        &self,
        method: Method,
        path: &str,
        body: Option<&B>,
    ) -> Result<T, DiscordError>
    where
        T: DeserializeOwned,
        B: serde::Serialize + ?Sized,
    {
        let response = self
            .request_with_headers(method, path, body.map(serialize_body))
            .await?;
        let value = parse_body_value(response.body);
        serde_json::from_value(value).map_err(Into::into)
    }

    async fn request_no_content<B>(
        &self,
        method: Method,
        path: &str,
        body: Option<&B>,
    ) -> Result<(), DiscordError>
    where
        B: serde::Serialize + ?Sized,
    {
        self.request_with_headers(method, path, body.map(serialize_body))
            .await?;
        Ok(())
    }

    async fn request_with_headers(
        &self,
        method: Method,
        path: &str,
        body: Option<Value>,
    ) -> Result<RawResponse, DiscordError> {
        let route_key = rate_limit_route_key(&method, path);
        while let Some(wait_duration) = self.rate_limits.wait_duration(&route_key) {
            debug!(
                "waiting for rate limit on {route_key} for {:?}",
                wait_duration
            );
            sleep_for_retry_after(wait_duration.as_secs_f64()).await;
        }

        let response = self
            .request_once(method.clone(), path, body.as_ref())
            .await?;
        self.rate_limits.observe(
            &route_key,
            &response.headers,
            response.status,
            &response.body,
        );

        if response.status == StatusCode::TOO_MANY_REQUESTS {
            warn!("received rate limit for {route_key}, retrying once");
            let payload = parse_body_value(response.body.clone());
            let retry_after = payload
                .get("retry_after")
                .and_then(Value::as_f64)
                .unwrap_or(1.0);
            sleep_for_retry_after(retry_after).await;

            let retried = self.request_once(method, path, body.as_ref()).await?;
            self.rate_limits
                .observe(&route_key, &retried.headers, retried.status, &retried.body);
            if retried.status == StatusCode::TOO_MANY_REQUESTS {
                return Err(discord_rate_limit_error(&route_key, &retried.body));
            }
            if !retried.status.is_success() {
                return Err(discord_api_error(retried.status, &retried.body));
            }
            return Ok(retried);
        }

        if !response.status.is_success() {
            return Err(discord_api_error(response.status, &response.body));
        }

        Ok(response)
    }

    async fn request_once(
        &self,
        method: Method,
        path: &str,
        body: Option<&Value>,
    ) -> Result<RawResponse, DiscordError> {
        let normalized_path = if path.starts_with('/') {
            path.to_string()
        } else {
            format!("/{path}")
        };
        let url = format!("{API_BASE}{normalized_path}");

        let mut request_builder = self
            .client
            .request(method, url)
            .header("Content-Type", "application/json")
            .header("User-Agent", "DiscordBot (discordrs, 1.0.0)");

        if request_uses_bot_authorization(&normalized_path) {
            request_builder =
                request_builder.header("Authorization", format!("Bot {}", self.token));
        }

        if let Some(body) = body {
            request_builder = request_builder.json(body);
        }

        let response = request_builder.send().await?;
        let status = response.status();
        let headers = response.headers().clone();
        let response_text = response.text().await?;

        Ok(RawResponse {
            status,
            headers,
            body: response_text,
        })
    }
}

struct RawResponse {
    status: StatusCode,
    headers: HeaderMap,
    body: String,
}

#[derive(Default)]
struct RateLimitState {
    route_buckets: Mutex<HashMap<String, String>>,
    blocked_until: Mutex<HashMap<String, Instant>>,
    global_blocked_until: Mutex<Option<Instant>>,
}

impl RateLimitState {
    fn wait_duration(&self, route_key: &str) -> Option<Duration> {
        let now = Instant::now();
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

    fn observe(&self, route_key: &str, headers: &HeaderMap, status: StatusCode, body: &str) {
        if let Some(bucket_id) = header_string(headers.get("x-ratelimit-bucket")) {
            self.route_buckets
                .lock()
                .expect("route bucket mutex poisoned")
                .insert(route_key.to_string(), bucket_id.clone());
        }

        if status == StatusCode::TOO_MANY_REQUESTS {
            let payload = parse_body_value(body.to_string());
            let retry_after = payload
                .get("retry_after")
                .and_then(Value::as_f64)
                .unwrap_or(1.0);
            let blocked_until = Instant::now() + Duration::from_secs_f64(retry_after.max(0.0));

            if payload
                .get("global")
                .and_then(Value::as_bool)
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
                self.block_key(route_key, headers, Instant::now() + reset_after);
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
            .insert(bucket_key, blocked_until);
    }
}

fn serialize_body<T: serde::Serialize + ?Sized>(body: &T) -> Value {
    serde_json::to_value(body).expect("failed to serialize request body")
}

fn clone_json_body(body: &Value) -> Value {
    body.clone()
}

fn parse_body_value(response_text: String) -> Value {
    if response_text.is_empty() {
        Value::Null
    } else {
        serde_json::from_str(&response_text).unwrap_or(Value::String(response_text))
    }
}

fn configured_application_id(application_id: u64) -> Result<String, DiscordError> {
    if application_id == 0 {
        return Err(invalid_data_error(
            "application_id must be set before follow-up webhook calls; use set_application_id() or create_followup_message_with_application_id()",
        ));
    }

    Ok(application_id.to_string())
}

fn validate_token_path_segment(
    label: &str,
    value: &str,
    allow_original_marker: bool,
) -> Result<(), DiscordError> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(invalid_data_error(format!("{label} must not be empty")));
    }

    if allow_original_marker && trimmed == "@original" {
        return Ok(());
    }

    if trimmed.contains('/')
        || trimmed.contains('\\')
        || trimmed.contains('?')
        || trimmed.contains('#')
    {
        return Err(invalid_data_error(format!(
            "{label} must not contain path separators or URL control characters"
        )));
    }

    Ok(())
}

fn global_commands_path(application_id: u64) -> Result<String, DiscordError> {
    let application_id = configured_application_id(application_id)?;
    Ok(format!("/applications/{application_id}/commands"))
}

fn interaction_callback_path(
    interaction_id: Snowflake,
    interaction_token: &str,
) -> Result<String, DiscordError> {
    validate_token_path_segment("interaction_token", interaction_token, false)?;
    Ok(format!(
        "/interactions/{interaction_id}/{interaction_token}/callback"
    ))
}

fn execute_webhook_path(webhook_id: Snowflake, token: &str) -> Result<String, DiscordError> {
    validate_token_path_segment("webhook_token", token, false)?;
    Ok(format!("/webhooks/{webhook_id}/{token}?wait=true"))
}

fn followup_webhook_path(
    application_id: &str,
    interaction_token: &str,
    message_id: Option<&str>,
) -> Result<String, DiscordError> {
    let application_id = application_id.trim();
    if application_id.is_empty() || application_id == "0" {
        return Err(invalid_data_error(
            "application_id must be set before follow-up webhook calls",
        ));
    }
    validate_token_path_segment("application_id", application_id, false)?;
    validate_token_path_segment("interaction_token", interaction_token, false)?;

    let path = match message_id {
        Some(message_id) => {
            validate_token_path_segment("message_id", message_id, true)?;
            format!("/webhooks/{application_id}/{interaction_token}/messages/{message_id}")
        }
        None => format!("/webhooks/{application_id}/{interaction_token}"),
    };

    Ok(path)
}

fn request_uses_bot_authorization(path: &str) -> bool {
    let normalized_path = path
        .split('?')
        .next()
        .unwrap_or(path)
        .trim_start_matches('/');
    !(normalized_path.starts_with("webhooks/") || normalized_path.starts_with("interactions/"))
}

fn discord_api_error(status: StatusCode, body: &str) -> DiscordError {
    let payload = parse_body_value(body.to_string());
    let code = payload.get("code").and_then(Value::as_u64);
    let message = payload
        .get("message")
        .and_then(Value::as_str)
        .map(str::to_string)
        .or_else(|| payload.as_str().map(str::to_string))
        .unwrap_or_else(|| payload.to_string());
    DiscordError::api(status.as_u16(), code, message)
}

fn discord_rate_limit_error(route: &str, body: &str) -> DiscordError {
    let payload = parse_body_value(body.to_string());
    let retry_after = payload
        .get("retry_after")
        .and_then(Value::as_f64)
        .unwrap_or(1.0);
    DiscordError::rate_limit(route.to_string(), retry_after)
}

fn is_major_parameter_segment(segments: &[&str], index: usize) -> bool {
    matches!(
        segments.get(index.saturating_sub(1)).copied(),
        Some("applications" | "channels" | "guilds" | "webhooks")
    ) || (index >= 2 && matches!(segments.get(index - 2).copied(), Some("webhooks")))
}

fn rate_limit_route_key(method: &Method, path: &str) -> String {
    let segments = path
        .split('?')
        .next()
        .unwrap_or(path)
        .split('/')
        .filter(|segment| !segment.is_empty())
        .collect::<Vec<_>>();
    let normalized = segments
        .iter()
        .enumerate()
        .map(|(index, segment)| {
            if segment.chars().all(|ch| ch.is_ascii_digit())
                && !is_major_parameter_segment(&segments, index)
            {
                ":id".to_string()
            } else {
                (*segment).to_string()
            }
        })
        .collect::<Vec<_>>()
        .join("/");

    format!("{method}:{normalized}")
}

fn header_string(value: Option<&HeaderValue>) -> Option<String> {
    value
        .and_then(|value| value.to_str().ok())
        .map(str::to_string)
}

async fn sleep_for_retry_after(retry_after_seconds: f64) {
    let duration = Duration::from_secs_f64(retry_after_seconds.max(0.0));
    let finished = Arc::new(AtomicBool::new(false));
    let waker = Arc::new(Mutex::new(None::<Waker>));

    let finished_for_thread = Arc::clone(&finished);
    let waker_for_thread = Arc::clone(&waker);
    std::thread::spawn(move || {
        std::thread::sleep(duration);
        finished_for_thread.store(true, Ordering::Release);
        if let Ok(mut slot) = waker_for_thread.lock() {
            if let Some(waker) = slot.take() {
                waker.wake();
            }
        }
    });

    poll_fn(move |cx| {
        if finished.load(Ordering::Acquire) {
            return Poll::Ready(());
        }

        if let Ok(mut slot) = waker.lock() {
            *slot = Some(cx.waker().clone());
        }

        if finished.load(Ordering::Acquire) {
            Poll::Ready(())
        } else {
            Poll::Pending
        }
    })
    .await;
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant};

    use super::{
        configured_application_id, discord_api_error, discord_rate_limit_error,
        execute_webhook_path, followup_webhook_path, global_commands_path,
        interaction_callback_path, rate_limit_route_key, request_uses_bot_authorization,
        RateLimitState,
    };
    use crate::error::DiscordError;
    use crate::model::Snowflake;
    use reqwest::{Method, StatusCode};

    #[test]
    fn configured_application_id_rejects_zero() {
        let error = configured_application_id(0).unwrap_err();
        assert!(error.to_string().contains("application_id must be set"));
    }

    #[test]
    fn global_commands_path_rejects_zero_application_id() {
        let error = global_commands_path(0).unwrap_err();
        assert!(error.to_string().contains("application_id must be set"));
    }

    #[test]
    fn global_commands_path_uses_configured_application_id() {
        assert_eq!(
            global_commands_path(123).unwrap(),
            "/applications/123/commands"
        );
    }

    #[test]
    fn followup_webhook_path_uses_explicit_application_id() {
        let path = followup_webhook_path("123", "token", None).unwrap();
        assert_eq!(path, "/webhooks/123/token");
    }

    #[test]
    fn edit_followup_webhook_path_includes_message_id() {
        let path = followup_webhook_path("123", "token", Some("456")).unwrap();
        assert_eq!(path, "/webhooks/123/token/messages/456");
    }

    #[test]
    fn original_interaction_response_path_uses_original_message_marker() {
        let path = followup_webhook_path("123", "token", Some("@original")).unwrap();
        assert_eq!(path, "/webhooks/123/token/messages/@original");
    }

    #[test]
    fn followup_webhook_path_rejects_zero_application_id() {
        let error = followup_webhook_path("0", "token", None).unwrap_err();
        assert!(error.to_string().contains("application_id must be set"));
    }

    #[test]
    fn followup_webhook_path_rejects_empty_or_unsafe_segments() {
        let token_error = followup_webhook_path("123", "", None).unwrap_err();
        assert!(token_error.to_string().contains("interaction_token"));

        let token_separator_error = followup_webhook_path("123", "token/part", None).unwrap_err();
        assert!(token_separator_error
            .to_string()
            .contains("interaction_token"));

        let application_id_error = followup_webhook_path("12/3", "token", None).unwrap_err();
        assert!(application_id_error.to_string().contains("application_id"));

        let message_error = followup_webhook_path("123", "token", Some("bad/id")).unwrap_err();
        assert!(message_error.to_string().contains("message_id"));
    }

    #[test]
    fn interaction_callback_path_rejects_unsafe_tokens() {
        let error = interaction_callback_path(Snowflake::from("123"), "bad/token").unwrap_err();
        assert!(error.to_string().contains("interaction_token"));
    }

    #[test]
    fn execute_webhook_path_rejects_unsafe_tokens() {
        let error = execute_webhook_path(Snowflake::from("123"), "bad/token").unwrap_err();
        assert!(error.to_string().contains("webhook_token"));
    }

    #[test]
    fn request_uses_bot_authorization_skips_tokenized_callback_paths() {
        assert!(request_uses_bot_authorization("/channels/123/messages"));
        assert!(!request_uses_bot_authorization("/webhooks/123/token"));
        assert!(!request_uses_bot_authorization(
            "/interactions/123/token/callback"
        ));
        assert!(!request_uses_bot_authorization(
            "/webhooks/123/token/messages/@original"
        ));
    }

    #[test]
    fn discord_api_error_preserves_status_and_code() {
        let error = discord_api_error(
            StatusCode::BAD_REQUEST,
            r#"{"code":50035,"message":"Invalid Form Body"}"#,
        );

        match error {
            DiscordError::Api {
                status,
                code,
                message,
            } => {
                assert_eq!(status, 400);
                assert_eq!(code, Some(50035));
                assert_eq!(message, "Invalid Form Body");
            }
            other => panic!("unexpected error variant: {other:?}"),
        }
    }

    #[test]
    fn discord_rate_limit_error_preserves_route_and_retry_after() {
        let error = discord_rate_limit_error(
            "POST:webhooks/123/token",
            r#"{"message":"You are being rate limited.","retry_after":2.5,"global":false}"#,
        );

        match error {
            DiscordError::RateLimit { route, retry_after } => {
                assert_eq!(route, "POST:webhooks/123/token");
                assert_eq!(retry_after, 2.5);
            }
            other => panic!("unexpected error variant: {other:?}"),
        }
    }

    #[test]
    fn rate_limit_route_key_preserves_major_parameters() {
        assert_eq!(
            rate_limit_route_key(&Method::GET, "/channels/123/messages/456"),
            "GET:channels/123/messages/:id"
        );
        assert_eq!(
            rate_limit_route_key(&Method::GET, "/guilds/789/members/456"),
            "GET:guilds/789/members/:id"
        );
        assert_eq!(
            rate_limit_route_key(&Method::GET, "/webhooks/111/222/messages/333"),
            "GET:webhooks/111/222/messages/:id"
        );
    }

    #[test]
    fn rate_limit_state_reports_wait_duration() {
        let state = RateLimitState::default();
        state.blocked_until.lock().unwrap().insert(
            "GET:channels/:id".to_string(),
            Instant::now() + Duration::from_secs(1),
        );

        assert!(state.wait_duration("GET:channels/:id").is_some());
    }

    #[test]
    fn rate_limit_state_keeps_major_parameters_distinct() {
        let state = RateLimitState::default();
        let blocked_route = rate_limit_route_key(&Method::GET, "/channels/123/messages/456");
        let other_route = rate_limit_route_key(&Method::GET, "/channels/999/messages/456");

        state.blocked_until.lock().unwrap().insert(
            blocked_route.clone(),
            Instant::now() + Duration::from_secs(1),
        );

        assert!(state.wait_duration(&blocked_route).is_some());
        assert!(state.wait_duration(&other_route).is_none());
    }
}
