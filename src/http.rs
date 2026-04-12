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
    #[cfg(test)]
    base_url: String,
}

pub type DiscordHttpClient = RestClient;

impl RestClient {
    pub fn new(token: impl Into<String>, application_id: u64) -> Self {
        Self {
            client: Client::new(),
            token: token.into(),
            application_id: AtomicU64::new(application_id),
            rate_limits: Arc::new(RateLimitState::default()),
            #[cfg(test)]
            base_url: API_BASE.to_string(),
        }
    }

    #[cfg(test)]
    fn new_with_base_url(
        token: impl Into<String>,
        application_id: u64,
        base_url: impl Into<String>,
    ) -> Self {
        Self {
            client: Client::new(),
            token: token.into(),
            application_id: AtomicU64::new(application_id),
            rate_limits: Arc::new(RateLimitState::default()),
            base_url: base_url.into(),
        }
    }

    #[cfg(test)]
    fn api_base(&self) -> &str {
        &self.base_url
    }

    #[cfg(not(test))]
    fn api_base(&self) -> &str {
        API_BASE
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
        let url = format!("{}{}", self.api_base(), normalized_path);

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
    use std::collections::HashMap;
    use std::sync::{Arc, Mutex};
    use std::time::{Duration, Instant};

    use super::{
        configured_application_id, discord_api_error, discord_rate_limit_error,
        execute_webhook_path, followup_webhook_path, global_commands_path, header_string,
        interaction_callback_path, parse_body_value, rate_limit_route_key,
        request_uses_bot_authorization, sleep_for_retry_after, validate_token_path_segment,
        RateLimitState, RestClient,
    };
    use crate::command::{command_type, CommandDefinition};
    use crate::error::DiscordError;
    use crate::model::{CreateMessage, InteractionCallbackResponse, Snowflake};
    use reqwest::header::{HeaderMap, HeaderValue};
    use reqwest::{Method, StatusCode};
    use serde_json::json;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;
    use tokio::task::JoinHandle;

    #[derive(Debug)]
    struct PlannedResponse {
        status: StatusCode,
        headers: Vec<(String, String)>,
        body: String,
    }

    impl PlannedResponse {
        fn json(status: StatusCode, body: serde_json::Value) -> Self {
            Self {
                status,
                headers: vec![("Content-Type".to_string(), "application/json".to_string())],
                body: body.to_string(),
            }
        }

        fn text(status: StatusCode, body: impl Into<String>) -> Self {
            Self {
                status,
                headers: vec![("Content-Type".to_string(), "text/plain".to_string())],
                body: body.into(),
            }
        }

        fn empty(status: StatusCode) -> Self {
            Self {
                status,
                headers: Vec::new(),
                body: String::new(),
            }
        }
    }

    #[derive(Debug, Clone)]
    struct RecordedRequest {
        method: String,
        path: String,
        headers: HashMap<String, String>,
        body: String,
    }

    impl RecordedRequest {
        fn header(&self, name: &str) -> Option<&str> {
            self.headers
                .get(&name.to_ascii_lowercase())
                .map(String::as_str)
        }
    }

    fn find_bytes(haystack: &[u8], needle: &[u8]) -> Option<usize> {
        haystack
            .windows(needle.len())
            .position(|window| window == needle)
    }

    async fn read_recorded_request(stream: &mut tokio::net::TcpStream) -> RecordedRequest {
        let mut buffer = Vec::new();
        let mut header_end = None;
        let mut content_length = 0usize;

        loop {
            let mut chunk = [0u8; 2048];
            let read = stream.read(&mut chunk).await.expect("read request");
            assert!(read > 0, "client disconnected before sending request");
            buffer.extend_from_slice(&chunk[..read]);

            if header_end.is_none() {
                if let Some(index) = find_bytes(&buffer, b"\r\n\r\n") {
                    header_end = Some(index + 4);
                    let header_text = String::from_utf8_lossy(&buffer[..index]).to_string();
                    content_length = header_text
                        .split("\r\n")
                        .find_map(|line| {
                            let (name, value) = line.split_once(':')?;
                            if name.eq_ignore_ascii_case("content-length") {
                                value.trim().parse::<usize>().ok()
                            } else {
                                None
                            }
                        })
                        .unwrap_or(0);
                }
            }

            if let Some(end) = header_end {
                if buffer.len() >= end + content_length {
                    let header_text = String::from_utf8_lossy(&buffer[..end - 4]).to_string();
                    let mut lines = header_text.split("\r\n");
                    let request_line = lines.next().expect("request line");
                    let mut parts = request_line.split_whitespace();
                    let method = parts.next().expect("method").to_string();
                    let path = parts.next().expect("path").to_string();
                    let headers = lines
                        .filter_map(|line| {
                            let (name, value) = line.split_once(':')?;
                            Some((name.trim().to_ascii_lowercase(), value.trim().to_string()))
                        })
                        .collect::<HashMap<_, _>>();
                    let body =
                        String::from_utf8_lossy(&buffer[end..end + content_length]).to_string();

                    return RecordedRequest {
                        method,
                        path,
                        headers,
                        body,
                    };
                }
            }
        }
    }

    async fn write_planned_response(stream: &mut tokio::net::TcpStream, response: PlannedResponse) {
        let status_text = response.status.canonical_reason().unwrap_or("OK");
        let mut raw = format!(
            "HTTP/1.1 {} {}\r\nContent-Length: {}\r\nConnection: close\r\n",
            response.status.as_u16(),
            status_text,
            response.body.len()
        );
        for (name, value) in response.headers {
            raw.push_str(&format!("{name}: {value}\r\n"));
        }
        raw.push_str("\r\n");

        stream
            .write_all(raw.as_bytes())
            .await
            .expect("write headers");
        if !response.body.is_empty() {
            stream
                .write_all(response.body.as_bytes())
                .await
                .expect("write body");
        }
    }

    async fn spawn_test_server(
        responses: Vec<PlannedResponse>,
    ) -> (String, Arc<Mutex<Vec<RecordedRequest>>>, JoinHandle<()>) {
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind test server");
        let base_url = format!("http://{}", listener.local_addr().expect("local addr"));
        let captured = Arc::new(Mutex::new(Vec::new()));
        let captured_for_task = Arc::clone(&captured);

        let task = tokio::spawn(async move {
            for response in responses {
                let (mut stream, _) = listener.accept().await.expect("accept request");
                let request = read_recorded_request(&mut stream).await;
                captured_for_task
                    .lock()
                    .expect("capture mutex")
                    .push(request);
                write_planned_response(&mut stream, response).await;
            }
        });

        (base_url, captured, task)
    }

    fn message_payload(id: &str, channel_id: &str, content: &str) -> serde_json::Value {
        json!({
            "id": id,
            "channel_id": channel_id,
            "content": content
        })
    }

    fn channel_payload(id: &str, kind: u8, name: Option<&str>) -> serde_json::Value {
        let mut channel = json!({
            "id": id,
            "type": kind
        });
        if let Some(name) = name {
            channel["name"] = json!(name);
        }
        channel
    }

    fn guild_payload(id: &str, name: &str) -> serde_json::Value {
        json!({
            "id": id,
            "name": name
        })
    }

    fn role_payload(id: &str, name: &str) -> serde_json::Value {
        json!({
            "id": id,
            "name": name
        })
    }

    fn command_payload(id: &str, name: &str, description: &str) -> serde_json::Value {
        json!({
            "id": id,
            "type": 1,
            "name": name,
            "description": description
        })
    }

    fn gateway_payload() -> serde_json::Value {
        json!({
            "url": "wss://gateway.discord.gg",
            "shards": 1,
            "session_start_limit": {
                "total": 10,
                "remaining": 9,
                "reset_after": 1000,
                "max_concurrency": 1
            }
        })
    }

    fn assert_request_basics(
        request: &RecordedRequest,
        method: &str,
        path: &str,
        expected_authorization: Option<&str>,
    ) {
        assert_eq!(request.method, method);
        assert_eq!(request.path, path);
        assert_eq!(request.header("authorization"), expected_authorization);
        assert_eq!(
            request.header("user-agent"),
            Some("DiscordBot (discordrs, 1.0.0)")
        );
        assert_eq!(request.header("content-type"), Some("application/json"));
    }

    fn sample_command() -> CommandDefinition {
        CommandDefinition {
            kind: command_type::CHAT_INPUT,
            name: "ping".to_string(),
            description: "pong".to_string(),
            ..CommandDefinition::default()
        }
    }

    fn sample_message() -> CreateMessage {
        CreateMessage {
            content: Some("hello".to_string()),
            ..CreateMessage::default()
        }
    }

    fn sample_interaction_response() -> InteractionCallbackResponse {
        InteractionCallbackResponse {
            kind: 4,
            data: Some(json!({ "content": "ack" })),
        }
    }

    fn assert_model_error_contains(error: DiscordError, expected: &str) {
        match error {
            DiscordError::Model { message } => {
                assert!(
                    message.contains(expected),
                    "expected `{expected}` in `{message}`"
                );
            }
            other => panic!("unexpected error variant: {other:?}"),
        }
    }

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
    fn interaction_and_webhook_paths_accept_safe_segments() {
        assert_eq!(
            interaction_callback_path(Snowflake::from("123"), "safe-token").unwrap(),
            "/interactions/123/safe-token/callback"
        );
        assert_eq!(
            execute_webhook_path(Snowflake::from("456"), "safe-token").unwrap(),
            "/webhooks/456/safe-token?wait=true"
        );
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
    fn discord_api_error_uses_string_and_object_fallback_messages() {
        match discord_api_error(StatusCode::BAD_REQUEST, r#""plain string""#) {
            DiscordError::Api { message, .. } => {
                assert_eq!(message, "plain string");
            }
            other => panic!("unexpected error variant: {other:?}"),
        }

        match discord_api_error(StatusCode::BAD_REQUEST, r#"{"code":7}"#) {
            DiscordError::Api { code, message, .. } => {
                assert_eq!(code, Some(7));
                assert_eq!(message, r#"{"code":7}"#);
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
    fn rate_limit_route_key_keeps_application_and_guild_major_ids() {
        assert_eq!(
            rate_limit_route_key(&Method::POST, "/applications/123/guilds/456/commands/789"),
            "POST:applications/123/guilds/456/commands/:id"
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

    #[test]
    fn parse_helpers_handle_empty_invalid_and_string_body_shapes() {
        assert_eq!(parse_body_value(String::new()), serde_json::Value::Null);
        assert_eq!(
            parse_body_value(String::from("plain text")),
            serde_json::Value::String(String::from("plain text"))
        );
        assert_eq!(
            parse_body_value(String::from(r#"{"message":"ok"}"#)),
            serde_json::json!({ "message": "ok" })
        );

        let header = HeaderValue::from_static("bucket-1");
        assert_eq!(header_string(Some(&header)), Some(String::from("bucket-1")));
        assert_eq!(header_string(None), None);
    }

    #[test]
    fn validate_token_path_segment_handles_original_marker_and_control_characters() {
        validate_token_path_segment("message_id", "@original", true).unwrap();
        validate_token_path_segment("token", "safe-token", false).unwrap();

        let backslash = validate_token_path_segment("token", r"bad\token", false).unwrap_err();
        assert!(backslash.to_string().contains("token"));

        let query = validate_token_path_segment("token", "bad?token", false).unwrap_err();
        assert!(query.to_string().contains("token"));
    }

    #[test]
    fn authorization_and_error_helpers_cover_query_and_fallback_cases() {
        assert!(request_uses_bot_authorization(
            "/channels/123/messages?wait=true"
        ));
        assert!(!request_uses_bot_authorization(
            "/webhooks/123/token?wait=true"
        ));

        match discord_api_error(StatusCode::BAD_REQUEST, "plain body") {
            DiscordError::Api {
                status,
                code,
                message,
            } => {
                assert_eq!(status, 400);
                assert_eq!(code, None);
                assert_eq!(message, "plain body");
            }
            other => panic!("unexpected error variant: {other:?}"),
        }

        match discord_rate_limit_error("GET:channels/123", r#"{"message":"limited"}"#) {
            DiscordError::RateLimit { route, retry_after } => {
                assert_eq!(route, "GET:channels/123");
                assert_eq!(retry_after, 1.0);
            }
            other => panic!("unexpected error variant: {other:?}"),
        }

        assert_eq!(
            rate_limit_route_key(&Method::PATCH, "/channels/123/messages/456?wait=true"),
            "PATCH:channels/123/messages/:id"
        );
    }

    #[test]
    fn rate_limit_state_observe_tracks_buckets_and_global_limits() {
        let state = RateLimitState::default();
        let route_key = "POST:channels/123/messages";
        let mut headers = HeaderMap::new();
        headers.insert("x-ratelimit-bucket", HeaderValue::from_static("bucket-42"));
        headers.insert("x-ratelimit-remaining", HeaderValue::from_static("0"));
        headers.insert("x-ratelimit-reset-after", HeaderValue::from_static("0.05"));

        state.observe(route_key, &headers, StatusCode::OK, "");
        assert!(state.route_buckets.lock().unwrap().contains_key(route_key));
        assert!(state.wait_duration(route_key).is_some());

        let global_headers = HeaderMap::new();
        state.observe(
            route_key,
            &global_headers,
            StatusCode::TOO_MANY_REQUESTS,
            r#"{"retry_after":0.05,"global":true}"#,
        );
        assert!(state.wait_duration("GET:anything").is_some());
    }

    #[test]
    fn rate_limit_state_shares_bucket_blocks_across_routes_after_429() {
        let state = RateLimitState::default();
        let route_a = rate_limit_route_key(&Method::GET, "/channels/123/messages");
        let route_b = rate_limit_route_key(&Method::POST, "/channels/456/messages");

        let mut bucket_headers = HeaderMap::new();
        bucket_headers.insert(
            "x-ratelimit-bucket",
            HeaderValue::from_static("shared-bucket"),
        );

        state.observe(&route_a, &bucket_headers, StatusCode::OK, "");
        state.observe(&route_b, &bucket_headers, StatusCode::OK, "");
        state.observe(
            &route_a,
            &HeaderMap::new(),
            StatusCode::TOO_MANY_REQUESTS,
            r#"{"retry_after":0.05,"global":false}"#,
        );

        assert!(state.wait_duration(&route_a).is_some());
        assert!(state.wait_duration(&route_b).is_some());
    }

    #[test]
    fn rate_limit_state_ignores_expired_route_and_global_blocks() {
        let state = RateLimitState::default();
        state
            .blocked_until
            .lock()
            .unwrap()
            .insert("GET:channels/123/messages".to_string(), Instant::now());
        *state.global_blocked_until.lock().unwrap() = Some(Instant::now());

        assert!(state.wait_duration("GET:channels/123/messages").is_none());
        assert!(state.wait_duration("GET:anything").is_none());
    }

    #[tokio::test]
    async fn sleep_for_retry_after_waits_without_panicking() {
        let start = Instant::now();
        sleep_for_retry_after(0.01).await;
        assert!(start.elapsed() >= Duration::from_millis(5));
    }

    #[tokio::test]
    async fn client_methods_reject_missing_application_id_before_request() {
        let client = RestClient::new("token", 0);
        let command = sample_command();
        let commands = vec![command.clone()];
        let body = sample_message();

        assert_model_error_contains(
            client
                .bulk_overwrite_global_commands_typed(&commands)
                .await
                .unwrap_err(),
            "application_id must be set",
        );
        assert_model_error_contains(
            client.create_global_command(&command).await.unwrap_err(),
            "application_id must be set",
        );
        assert_model_error_contains(
            client.get_global_commands().await.unwrap_err(),
            "application_id must be set",
        );
        assert_model_error_contains(
            client
                .bulk_overwrite_guild_commands_typed(Snowflake::from("456"), &commands)
                .await
                .unwrap_err(),
            "application_id must be set",
        );
        assert_model_error_contains(
            client
                .create_followup_message_json("token", &json!({ "content": "hi" }))
                .await
                .unwrap_err(),
            "application_id must be set",
        );
        assert_model_error_contains(
            client
                .create_followup_message("token", &body)
                .await
                .unwrap_err(),
            "application_id must be set",
        );
        assert_model_error_contains(
            client
                .get_original_interaction_response("token")
                .await
                .unwrap_err(),
            "application_id must be set",
        );
        assert_model_error_contains(
            client
                .edit_original_interaction_response("token", &body)
                .await
                .unwrap_err(),
            "application_id must be set",
        );
        assert_model_error_contains(
            client
                .delete_original_interaction_response("token")
                .await
                .unwrap_err(),
            "application_id must be set",
        );
        assert_model_error_contains(
            client
                .edit_followup_message("token", "123", &body)
                .await
                .unwrap_err(),
            "application_id must be set",
        );
        assert_model_error_contains(
            client
                .delete_followup_message("token", "123")
                .await
                .unwrap_err(),
            "application_id must be set",
        );
    }

    #[tokio::test]
    async fn client_methods_reject_unsafe_tokens_before_request() {
        let client = RestClient::new("token", 123);
        let body = sample_message();
        let response = sample_interaction_response();

        assert_model_error_contains(
            client
                .execute_webhook(Snowflake::from("456"), "bad/token", &json!({}))
                .await
                .unwrap_err(),
            "webhook_token",
        );
        assert_model_error_contains(
            client
                .create_interaction_response_typed(Snowflake::from("789"), "bad/token", &response)
                .await
                .unwrap_err(),
            "interaction_token",
        );
        assert_model_error_contains(
            client
                .create_interaction_response_json(Snowflake::from("789"), "bad/token", &json!({}))
                .await
                .unwrap_err(),
            "interaction_token",
        );
        assert_model_error_contains(
            client
                .create_followup_message_with_application_id("123", "bad/token", &body)
                .await
                .unwrap_err(),
            "interaction_token",
        );
        assert_model_error_contains(
            client
                .create_followup_message_json_with_application_id(
                    "123",
                    "bad/token",
                    &json!({ "content": "hi" }),
                )
                .await
                .unwrap_err(),
            "interaction_token",
        );
        assert_model_error_contains(
            client
                .get_original_interaction_response_with_application_id("123", "bad/token")
                .await
                .unwrap_err(),
            "interaction_token",
        );
        assert_model_error_contains(
            client
                .edit_original_interaction_response_with_application_id("123", "bad/token", &body)
                .await
                .unwrap_err(),
            "interaction_token",
        );
        assert_model_error_contains(
            client
                .delete_original_interaction_response_with_application_id("123", "bad/token")
                .await
                .unwrap_err(),
            "interaction_token",
        );
    }

    #[tokio::test]
    async fn client_followup_methods_validate_application_and_message_segments() {
        let client = RestClient::new("token", 123);
        let body = sample_message();

        assert_model_error_contains(
            client
                .create_followup_message_json_with_application_id(
                    "12/3",
                    "token",
                    &json!({ "content": "hi" }),
                )
                .await
                .unwrap_err(),
            "application_id",
        );
        assert_model_error_contains(
            client
                .create_followup_message_with_application_id("12/3", "token", &body)
                .await
                .unwrap_err(),
            "application_id",
        );
        assert_model_error_contains(
            client
                .edit_followup_message_with_application_id("123", "token", "bad/id", &body)
                .await
                .unwrap_err(),
            "message_id",
        );
        assert_model_error_contains(
            client
                .delete_followup_message_with_application_id("123", "token", "bad/id")
                .await
                .unwrap_err(),
            "message_id",
        );
    }

    #[test]
    fn header_string_returns_none_for_invalid_header_bytes() {
        let invalid = HeaderValue::from_bytes(&[0xFF]).expect("invalid but allowed header bytes");
        assert_eq!(header_string(Some(&invalid)), None);
    }

    #[test]
    fn rate_limit_state_does_not_block_without_reset_after_header() {
        let state = RateLimitState::default();
        let mut headers = HeaderMap::new();
        headers.insert("x-ratelimit-remaining", HeaderValue::from_static("0"));

        state.observe("GET:channels/123/messages", &headers, StatusCode::OK, "");

        assert!(state.wait_duration("GET:channels/123/messages").is_none());
    }

    #[tokio::test]
    async fn sleep_for_retry_after_clamps_negative_values() {
        let start = Instant::now();
        sleep_for_retry_after(-1.0).await;
        assert!(start.elapsed() < Duration::from_millis(50));
    }

    #[tokio::test]
    async fn client_message_and_channel_wrappers_hit_local_server() {
        let responses = vec![
            PlannedResponse::json(StatusCode::OK, message_payload("1", "100", "hello")),
            PlannedResponse::json(StatusCode::OK, message_payload("1", "100", "updated")),
            PlannedResponse::json(StatusCode::OK, message_payload("1", "100", "updated")),
            PlannedResponse::json(StatusCode::OK, channel_payload("100", 0, Some("general"))),
            PlannedResponse::json(StatusCode::OK, channel_payload("100", 0, Some("general"))),
            PlannedResponse::json(StatusCode::OK, channel_payload("100", 0, Some("renamed"))),
            PlannedResponse::json(
                StatusCode::OK,
                json!([message_payload("1", "100", "updated")]),
            ),
            PlannedResponse::json(
                StatusCode::OK,
                json!([message_payload("2", "100", "latest")]),
            ),
            PlannedResponse::empty(StatusCode::NO_CONTENT),
            PlannedResponse::empty(StatusCode::NO_CONTENT),
            PlannedResponse::empty(StatusCode::NO_CONTENT),
            PlannedResponse::json(StatusCode::OK, json!({ "ok": true })),
            PlannedResponse::json(StatusCode::OK, json!({ "id": "903", "content": "raw" })),
            PlannedResponse::json(
                StatusCode::OK,
                json!({ "id": "903", "content": "edited raw" }),
            ),
            PlannedResponse::empty(StatusCode::NO_CONTENT),
        ];
        let (base_url, captured, server) = spawn_test_server(responses).await;
        let client = RestClient::new_with_base_url("token", 321, base_url);
        let body = sample_message();

        let created = client
            .create_message(Snowflake::from("100"), &body)
            .await
            .expect("create message");
        assert_eq!(created.content, "hello");

        let updated = client
            .update_message(Snowflake::from("100"), Snowflake::from("1"), &body)
            .await
            .expect("update message");
        assert_eq!(updated.content, "updated");

        let fetched = client
            .get_message(Snowflake::from("100"), Snowflake::from("1"))
            .await
            .expect("get message");
        assert_eq!(fetched.content, "updated");

        let channel = client
            .get_channel(Snowflake::from("100"))
            .await
            .expect("get channel");
        assert_eq!(channel.name.as_deref(), Some("general"));

        let deleted_channel = client
            .delete_channel(Snowflake::from("100"))
            .await
            .expect("delete channel");
        assert_eq!(deleted_channel.id.as_str(), "100");

        let renamed = client
            .update_channel(Snowflake::from("100"), &json!({ "name": "renamed" }))
            .await
            .expect("update channel");
        assert_eq!(renamed.name.as_deref(), Some("renamed"));

        let limited_messages = client
            .get_channel_messages(Snowflake::from("100"), Some(2))
            .await
            .expect("channel messages with limit");
        assert_eq!(limited_messages.len(), 1);

        let all_messages = client
            .get_channel_messages(Snowflake::from("100"), None)
            .await
            .expect("channel messages without limit");
        assert_eq!(all_messages[0].content, "latest");

        client
            .bulk_delete_messages(
                Snowflake::from("100"),
                vec![Snowflake::from("1"), Snowflake::from("2")],
            )
            .await
            .expect("bulk delete");
        client
            .add_reaction(Snowflake::from("100"), Snowflake::from("1"), "spark")
            .await
            .expect("add reaction");
        client
            .remove_reaction(Snowflake::from("100"), Snowflake::from("1"), "spark")
            .await
            .expect("remove reaction");

        let raw = client
            .request(
                Method::GET,
                "channels/100/custom",
                Option::<&serde_json::Value>::None,
            )
            .await
            .expect("request with normalized path");
        assert_eq!(raw["ok"], json!(true));

        let sent = client
            .send_message_json(Snowflake::from("100"), &json!({ "content": "raw" }))
            .await
            .expect("send raw message");
        assert_eq!(sent["id"], json!("903"));

        let edited = client
            .edit_message_json(
                Snowflake::from("100"),
                Snowflake::from("903"),
                &json!({ "content": "edited raw" }),
            )
            .await
            .expect("edit raw message");
        assert_eq!(edited["content"], json!("edited raw"));

        client
            .delete_message(Snowflake::from("100"), Snowflake::from("903"))
            .await
            .expect("delete raw message");

        server.await.expect("server finished");
        let requests = captured.lock().expect("captured requests");
        let auth = Some("Bot token");

        assert_request_basics(&requests[0], "POST", "/channels/100/messages", auth);
        assert_eq!(
            serde_json::from_str::<serde_json::Value>(&requests[0].body).unwrap()["content"],
            json!("hello")
        );
        assert_request_basics(&requests[1], "PATCH", "/channels/100/messages/1", auth);
        assert_request_basics(&requests[2], "GET", "/channels/100/messages/1", auth);
        assert_request_basics(&requests[3], "GET", "/channels/100", auth);
        assert_request_basics(&requests[4], "DELETE", "/channels/100", auth);
        assert_request_basics(&requests[5], "PATCH", "/channels/100", auth);
        assert_request_basics(&requests[6], "GET", "/channels/100/messages?limit=2", auth);
        assert_request_basics(&requests[7], "GET", "/channels/100/messages", auth);
        assert_request_basics(
            &requests[8],
            "POST",
            "/channels/100/messages/bulk-delete",
            auth,
        );
        assert_eq!(
            serde_json::from_str::<serde_json::Value>(&requests[8].body).unwrap(),
            json!({ "messages": ["1", "2"] })
        );
        assert_request_basics(
            &requests[9],
            "PUT",
            "/channels/100/messages/1/reactions/spark/@me",
            auth,
        );
        assert_request_basics(
            &requests[10],
            "DELETE",
            "/channels/100/messages/1/reactions/spark/@me",
            auth,
        );
        assert_request_basics(&requests[11], "GET", "/channels/100/custom", auth);
        assert_request_basics(&requests[12], "POST", "/channels/100/messages", auth);
        assert_request_basics(&requests[13], "PATCH", "/channels/100/messages/903", auth);
        assert_request_basics(&requests[14], "DELETE", "/channels/100/messages/903", auth);
    }

    #[tokio::test]
    async fn client_guild_and_command_wrappers_hit_local_server() {
        let responses = vec![
            PlannedResponse::json(StatusCode::OK, guild_payload("200", "guild")),
            PlannedResponse::json(StatusCode::OK, guild_payload("200", "guild-updated")),
            PlannedResponse::json(
                StatusCode::OK,
                json!([channel_payload("201", 0, Some("rules"))]),
            ),
            PlannedResponse::json(StatusCode::OK, channel_payload("202", 0, Some("new"))),
            PlannedResponse::json(StatusCode::OK, json!([{}])),
            PlannedResponse::json(StatusCode::OK, json!([{}])),
            PlannedResponse::empty(StatusCode::NO_CONTENT),
            PlannedResponse::empty(StatusCode::NO_CONTENT),
            PlannedResponse::empty(StatusCode::NO_CONTENT),
            PlannedResponse::json(StatusCode::OK, role_payload("300", "admin")),
            PlannedResponse::json(StatusCode::OK, role_payload("300", "mod")),
            PlannedResponse::empty(StatusCode::NO_CONTENT),
            PlannedResponse::json(StatusCode::OK, json!({})),
            PlannedResponse::json(StatusCode::OK, json!([role_payload("300", "mod")])),
            PlannedResponse::json(
                StatusCode::OK,
                json!([command_payload("401", "ping", "pong")]),
            ),
            PlannedResponse::json(StatusCode::OK, command_payload("402", "pong", "reply")),
            PlannedResponse::json(
                StatusCode::OK,
                json!([command_payload("401", "ping", "pong")]),
            ),
            PlannedResponse::json(StatusCode::OK, gateway_payload()),
            PlannedResponse::json(
                StatusCode::OK,
                json!([command_payload("403", "guild", "only")]),
            ),
        ];
        let (base_url, captured, server) = spawn_test_server(responses).await;
        let client = RestClient::new_with_base_url("guild-token", 0, base_url);
        client.set_application_id(555);
        let command = sample_command();

        let guild = client
            .get_guild(Snowflake::from("200"))
            .await
            .expect("get guild");
        assert_eq!(guild.name, "guild");

        let updated = client
            .update_guild(Snowflake::from("200"), &json!({ "name": "guild-updated" }))
            .await
            .expect("update guild");
        assert_eq!(updated.name, "guild-updated");

        let channels = client
            .get_guild_channels(Snowflake::from("200"))
            .await
            .expect("get guild channels");
        assert_eq!(channels.len(), 1);

        let created_channel = client
            .create_guild_channel(Snowflake::from("200"), &json!({ "name": "new" }))
            .await
            .expect("create guild channel");
        assert_eq!(created_channel.id.as_str(), "202");

        assert_eq!(
            client
                .get_guild_members(Snowflake::from("200"), Some(3))
                .await
                .expect("members with limit")
                .len(),
            1
        );
        assert_eq!(
            client
                .get_guild_members(Snowflake::from("200"), None)
                .await
                .expect("members without limit")
                .len(),
            1
        );
        client
            .remove_guild_member(Snowflake::from("200"), Snowflake::from("201"))
            .await
            .expect("remove guild member");
        client
            .add_guild_member_role(
                Snowflake::from("200"),
                Snowflake::from("201"),
                Snowflake::from("300"),
            )
            .await
            .expect("add guild member role");
        client
            .remove_guild_member_role(
                Snowflake::from("200"),
                Snowflake::from("201"),
                Snowflake::from("300"),
            )
            .await
            .expect("remove guild member role");

        let created_role = client
            .create_role(Snowflake::from("200"), &json!({ "name": "admin" }))
            .await
            .expect("create role");
        assert_eq!(created_role.name, "admin");

        let updated_role = client
            .update_role(
                Snowflake::from("200"),
                Snowflake::from("300"),
                &json!({ "name": "mod" }),
            )
            .await
            .expect("update role");
        assert_eq!(updated_role.name, "mod");

        client
            .delete_role(Snowflake::from("200"), Snowflake::from("300"))
            .await
            .expect("delete role");

        let member = client
            .get_member(Snowflake::from("200"), Snowflake::from("201"))
            .await
            .expect("get member");
        assert!(member.roles.is_empty());

        let roles = client
            .list_roles(Snowflake::from("200"))
            .await
            .expect("list roles");
        assert_eq!(roles[0].name, "mod");

        let overwritten = client
            .bulk_overwrite_global_commands_typed(std::slice::from_ref(&command))
            .await
            .expect("bulk overwrite global commands");
        assert_eq!(overwritten[0].name, "ping");

        let created_command = client
            .create_global_command(&CommandDefinition {
                name: "pong".to_string(),
                description: "reply".to_string(),
                ..command.clone()
            })
            .await
            .expect("create global command");
        assert_eq!(created_command.name, "pong");

        let global_commands = client
            .get_global_commands()
            .await
            .expect("get global commands");
        assert_eq!(global_commands.len(), 1);

        let gateway = client.get_gateway_bot().await.expect("get gateway bot");
        assert_eq!(gateway.shards, 1);

        let guild_commands = client
            .bulk_overwrite_guild_commands_typed(Snowflake::from("200"), &[command])
            .await
            .expect("bulk overwrite guild commands");
        assert_eq!(guild_commands[0].name, "guild");

        server.await.expect("server finished");
        let requests = captured.lock().expect("captured requests");
        let auth = Some("Bot guild-token");

        assert_request_basics(&requests[0], "GET", "/guilds/200", auth);
        assert_request_basics(&requests[1], "PATCH", "/guilds/200", auth);
        assert_request_basics(&requests[2], "GET", "/guilds/200/channels", auth);
        assert_request_basics(&requests[3], "POST", "/guilds/200/channels", auth);
        assert_request_basics(&requests[4], "GET", "/guilds/200/members?limit=3", auth);
        assert_request_basics(&requests[5], "GET", "/guilds/200/members", auth);
        assert_request_basics(&requests[6], "DELETE", "/guilds/200/members/201", auth);
        assert_request_basics(
            &requests[7],
            "PUT",
            "/guilds/200/members/201/roles/300",
            auth,
        );
        assert_request_basics(
            &requests[8],
            "DELETE",
            "/guilds/200/members/201/roles/300",
            auth,
        );
        assert_request_basics(&requests[9], "POST", "/guilds/200/roles", auth);
        assert_request_basics(&requests[10], "PATCH", "/guilds/200/roles/300", auth);
        assert_request_basics(&requests[11], "DELETE", "/guilds/200/roles/300", auth);
        assert_request_basics(&requests[12], "GET", "/guilds/200/members/201", auth);
        assert_request_basics(&requests[13], "GET", "/guilds/200/roles", auth);
        assert_request_basics(&requests[14], "PUT", "/applications/555/commands", auth);
        assert_request_basics(&requests[15], "POST", "/applications/555/commands", auth);
        assert_request_basics(&requests[16], "GET", "/applications/555/commands", auth);
        assert_request_basics(&requests[17], "GET", "/gateway/bot", auth);
        assert_request_basics(
            &requests[18],
            "PUT",
            "/applications/555/guilds/200/commands",
            auth,
        );
    }

    #[tokio::test]
    async fn client_webhook_and_followup_wrappers_hit_local_server() {
        let responses = vec![
            PlannedResponse::json(StatusCode::OK, json!({ "id": "900" })),
            PlannedResponse::json(StatusCode::OK, json!([{ "id": "900" }])),
            PlannedResponse::json(StatusCode::OK, json!({ "unexpected": true })),
            PlannedResponse::json(
                StatusCode::TOO_MANY_REQUESTS,
                json!({ "retry_after": 0.0, "global": false }),
            ),
            PlannedResponse::json(StatusCode::OK, json!({ "id": "901" })),
            PlannedResponse::json(StatusCode::OK, channel_payload("500", 1, None)),
            PlannedResponse::empty(StatusCode::NO_CONTENT),
            PlannedResponse::empty(StatusCode::NO_CONTENT),
            PlannedResponse::json(StatusCode::OK, json!({ "id": "902" })),
            PlannedResponse::json(StatusCode::OK, json!({ "id": "903" })),
            PlannedResponse::json(StatusCode::OK, message_payload("904", "500", "followup")),
            PlannedResponse::json(StatusCode::OK, message_payload("905", "500", "followup")),
            PlannedResponse::json(StatusCode::OK, message_payload("906", "500", "original")),
            PlannedResponse::json(StatusCode::OK, message_payload("907", "500", "original")),
            PlannedResponse::json(StatusCode::OK, message_payload("908", "500", "edited")),
            PlannedResponse::json(StatusCode::OK, message_payload("909", "500", "edited")),
            PlannedResponse::empty(StatusCode::NO_CONTENT),
            PlannedResponse::empty(StatusCode::NO_CONTENT),
            PlannedResponse::json(
                StatusCode::OK,
                message_payload("910", "500", "followup-edit"),
            ),
            PlannedResponse::json(
                StatusCode::OK,
                message_payload("911", "500", "followup-edit"),
            ),
            PlannedResponse::empty(StatusCode::NO_CONTENT),
            PlannedResponse::empty(StatusCode::NO_CONTENT),
        ];
        let (base_url, captured, server) = spawn_test_server(responses).await;
        let client = RestClient::new_with_base_url("hook-token", 123, base_url);
        let body = sample_message();

        let webhook = client
            .create_webhook(Snowflake::from("500"), &json!({ "name": "hook" }))
            .await
            .expect("create webhook");
        assert_eq!(webhook["id"], json!("900"));

        let webhooks = client
            .get_channel_webhooks(Snowflake::from("500"))
            .await
            .expect("get channel webhooks array");
        assert_eq!(webhooks.len(), 1);

        let fallback = client
            .get_channel_webhooks(Snowflake::from("500"))
            .await
            .expect("get channel webhooks fallback");
        assert!(fallback.is_empty());

        let executed = client
            .execute_webhook(
                Snowflake::from("777"),
                "token",
                &json!({ "content": "hook" }),
            )
            .await
            .expect("execute webhook with retry");
        assert_eq!(executed["id"], json!("901"));

        let dm = client
            .create_dm_channel_typed(&crate::model::CreateDmChannel {
                recipient_id: Snowflake::from("42"),
            })
            .await
            .expect("create dm channel");
        assert_eq!(dm.id.as_str(), "500");

        client
            .create_interaction_response_typed(
                Snowflake::from("777"),
                "token",
                &sample_interaction_response(),
            )
            .await
            .expect("create interaction response typed");
        client
            .create_interaction_response_json(
                Snowflake::from("778"),
                "token",
                &json!({ "type": 4, "data": { "content": "json" } }),
            )
            .await
            .expect("create interaction response json");

        assert_eq!(
            client
                .create_followup_message_json_with_application_id(
                    "123",
                    "token",
                    &json!({ "content": "json" }),
                )
                .await
                .expect("explicit followup json")["id"],
            json!("902")
        );
        assert_eq!(
            client
                .create_followup_message_json("token", &json!({ "content": "implicit" }))
                .await
                .expect("implicit followup json")["id"],
            json!("903")
        );
        assert_eq!(
            client
                .create_followup_message_with_application_id("123", "token", &body)
                .await
                .expect("explicit followup message")
                .content,
            "followup"
        );
        assert_eq!(
            client
                .create_followup_message("token", &body)
                .await
                .expect("implicit followup message")
                .content,
            "followup"
        );
        assert_eq!(
            client
                .get_original_interaction_response_with_application_id("123", "token")
                .await
                .expect("explicit original get")
                .content,
            "original"
        );
        assert_eq!(
            client
                .get_original_interaction_response("token")
                .await
                .expect("implicit original get")
                .content,
            "original"
        );
        assert_eq!(
            client
                .edit_original_interaction_response_with_application_id("123", "token", &body)
                .await
                .expect("explicit original edit")
                .content,
            "edited"
        );
        assert_eq!(
            client
                .edit_original_interaction_response("token", &body)
                .await
                .expect("implicit original edit")
                .content,
            "edited"
        );
        client
            .delete_original_interaction_response_with_application_id("123", "token")
            .await
            .expect("explicit original delete");
        client
            .delete_original_interaction_response("token")
            .await
            .expect("implicit original delete");

        assert_eq!(
            client
                .edit_followup_message_with_application_id("123", "token", "55", &body)
                .await
                .expect("explicit followup edit")
                .content,
            "followup-edit"
        );
        assert_eq!(
            client
                .edit_followup_message("token", "55", &body)
                .await
                .expect("implicit followup edit")
                .content,
            "followup-edit"
        );
        client
            .delete_followup_message_with_application_id("123", "token", "55")
            .await
            .expect("explicit followup delete");
        client
            .delete_followup_message("token", "55")
            .await
            .expect("implicit followup delete");

        server.await.expect("server finished");
        let requests = captured.lock().expect("captured requests");
        let bot_auth = Some("Bot hook-token");

        assert_request_basics(&requests[0], "POST", "/channels/500/webhooks", bot_auth);
        assert_request_basics(&requests[1], "GET", "/channels/500/webhooks", bot_auth);
        assert_request_basics(&requests[2], "GET", "/channels/500/webhooks", bot_auth);
        assert_request_basics(&requests[3], "POST", "/webhooks/777/token?wait=true", None);
        assert_request_basics(&requests[4], "POST", "/webhooks/777/token?wait=true", None);
        assert_request_basics(&requests[5], "POST", "/users/@me/channels", bot_auth);
        assert_request_basics(
            &requests[6],
            "POST",
            "/interactions/777/token/callback",
            None,
        );
        assert_request_basics(
            &requests[7],
            "POST",
            "/interactions/778/token/callback",
            None,
        );
        assert_request_basics(&requests[8], "POST", "/webhooks/123/token", None);
        assert_request_basics(&requests[9], "POST", "/webhooks/123/token", None);
        assert_request_basics(&requests[10], "POST", "/webhooks/123/token", None);
        assert_request_basics(&requests[11], "POST", "/webhooks/123/token", None);
        assert_request_basics(
            &requests[12],
            "GET",
            "/webhooks/123/token/messages/@original",
            None,
        );
        assert_request_basics(
            &requests[13],
            "GET",
            "/webhooks/123/token/messages/@original",
            None,
        );
        assert_request_basics(
            &requests[14],
            "PATCH",
            "/webhooks/123/token/messages/@original",
            None,
        );
        assert_request_basics(
            &requests[15],
            "PATCH",
            "/webhooks/123/token/messages/@original",
            None,
        );
        assert_request_basics(
            &requests[16],
            "DELETE",
            "/webhooks/123/token/messages/@original",
            None,
        );
        assert_request_basics(
            &requests[17],
            "DELETE",
            "/webhooks/123/token/messages/@original",
            None,
        );
        assert_request_basics(
            &requests[18],
            "PATCH",
            "/webhooks/123/token/messages/55",
            None,
        );
        assert_request_basics(
            &requests[19],
            "PATCH",
            "/webhooks/123/token/messages/55",
            None,
        );
        assert_request_basics(
            &requests[20],
            "DELETE",
            "/webhooks/123/token/messages/55",
            None,
        );
        assert_request_basics(
            &requests[21],
            "DELETE",
            "/webhooks/123/token/messages/55",
            None,
        );
    }

    #[tokio::test]
    async fn request_surfaces_api_and_rate_limit_errors_from_local_server() {
        let responses = vec![
            PlannedResponse::text(
                StatusCode::BAD_REQUEST,
                r#"{"code":50035,"message":"bad payload"}"#,
            ),
            PlannedResponse::json(
                StatusCode::TOO_MANY_REQUESTS,
                json!({ "retry_after": 0.0, "global": false }),
            ),
            PlannedResponse::json(
                StatusCode::TOO_MANY_REQUESTS,
                json!({ "retry_after": 0.0, "global": false }),
            ),
        ];
        let (base_url, captured, server) = spawn_test_server(responses).await;
        let client = RestClient::new_with_base_url("err-token", 123, base_url);

        match client
            .request(
                Method::POST,
                "/channels/9/messages",
                Some(&json!({ "content": "boom" })),
            )
            .await
            .unwrap_err()
        {
            DiscordError::Api {
                status,
                code,
                message,
            } => {
                assert_eq!(status, 400);
                assert_eq!(code, Some(50035));
                assert_eq!(message, "bad payload");
            }
            other => panic!("unexpected api error: {other:?}"),
        }

        match client
            .execute_webhook(Snowflake::from("9"), "token", &json!({ "content": "boom" }))
            .await
            .unwrap_err()
        {
            DiscordError::RateLimit { route, retry_after } => {
                assert_eq!(route, "POST:webhooks/9/token");
                assert_eq!(retry_after, 0.0);
            }
            other => panic!("unexpected rate limit error: {other:?}"),
        }

        server.await.expect("server finished");
        let requests = captured.lock().expect("captured requests");
        assert_eq!(requests.len(), 3);
        assert_request_basics(
            &requests[0],
            "POST",
            "/channels/9/messages",
            Some("Bot err-token"),
        );
        assert_request_basics(&requests[1], "POST", "/webhooks/9/token?wait=true", None);
        assert_request_basics(&requests[2], "POST", "/webhooks/9/token?wait=true", None);
    }
}
