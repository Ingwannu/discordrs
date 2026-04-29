use std::collections::HashMap;
use std::str::FromStr;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use reqwest::{
    header::{HeaderMap, HeaderValue},
    multipart::{Form, Part},
    Client, Method, StatusCode,
};
use serde::de::DeserializeOwned;
use serde_json::Value;
use tracing::{debug, warn};

use crate::command::CommandDefinition;
use crate::error::DiscordError;
use crate::model::{
    ApplicationCommand, ArchivedThreadsQuery, Ban, Channel, CreateDmChannel, CreateMessage,
    CreateTestEntitlement, Entitlement, EntitlementQuery, FollowedChannel, GatewayBot, Guild,
    GuildOnboarding, GuildScheduledEvent, GuildScheduledEventUser, GuildTemplate,
    GuildWidgetSettings, Integration, InteractionCallbackResponse, Invite,
    JoinedArchivedThreadsQuery, Member, Message, PollAnswerVoters, Role, Sku, Snowflake,
    SoundboardSound, SoundboardSoundList, StageInstance, Sticker, StickerPackList, Subscription,
    SubscriptionQuery, ThreadListResponse, ThreadMember, ThreadMemberQuery, User, Webhook,
    WelcomeScreen,
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

/// File data attached to a Discord multipart request.
///
/// The request body is sent with a `payload_json` part plus one `files[n]`
/// part per attachment.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FileAttachment {
    pub filename: String,
    pub data: Vec<u8>,
    pub content_type: Option<String>,
}

pub type FileUpload = FileAttachment;

impl FileAttachment {
    pub fn new(filename: impl Into<String>, data: impl Into<Vec<u8>>) -> Self {
        Self {
            filename: filename.into(),
            data: data.into(),
            content_type: None,
        }
    }

    pub fn with_content_type(mut self, content_type: impl Into<String>) -> Self {
        self.content_type = Some(content_type.into());
        self
    }
}

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

    pub async fn create_message_with_files(
        &self,
        channel_id: impl Into<Snowflake>,
        body: &CreateMessage,
        files: &[FileAttachment],
    ) -> Result<Message, DiscordError> {
        self.request_typed_multipart(
            Method::POST,
            &format!("/channels/{}/messages", channel_id.into()),
            body,
            files,
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

    pub async fn update_message_with_files(
        &self,
        channel_id: impl Into<Snowflake>,
        message_id: impl Into<Snowflake>,
        body: &CreateMessage,
        files: &[FileAttachment],
    ) -> Result<Message, DiscordError> {
        self.request_typed_multipart(
            Method::PATCH,
            &format!(
                "/channels/{}/messages/{}",
                channel_id.into(),
                message_id.into()
            ),
            body,
            files,
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

    pub async fn execute_webhook_with_files(
        &self,
        webhook_id: impl Into<Snowflake>,
        token: &str,
        body: &Value,
        files: &[FileAttachment],
    ) -> Result<Value, DiscordError> {
        let path = execute_webhook_path(webhook_id.into(), token)?;
        self.request_multipart(Method::POST, &path, body, files)
            .await
    }

    pub async fn get_webhook_message(
        &self,
        webhook_id: impl Into<Snowflake>,
        token: &str,
        message_id: &str,
    ) -> Result<Message, DiscordError> {
        let path = webhook_message_path(webhook_id.into(), token, message_id)?;
        self.request_typed(Method::GET, &path, Option::<&Value>::None)
            .await
    }

    pub async fn edit_webhook_message(
        &self,
        webhook_id: impl Into<Snowflake>,
        token: &str,
        message_id: &str,
        body: &CreateMessage,
    ) -> Result<Message, DiscordError> {
        let path = webhook_message_path(webhook_id.into(), token, message_id)?;
        self.request_typed(Method::PATCH, &path, Some(body)).await
    }

    pub async fn edit_webhook_message_with_files(
        &self,
        webhook_id: impl Into<Snowflake>,
        token: &str,
        message_id: &str,
        body: &CreateMessage,
        files: &[FileAttachment],
    ) -> Result<Message, DiscordError> {
        let path = webhook_message_path(webhook_id.into(), token, message_id)?;
        self.request_typed_multipart(Method::PATCH, &path, body, files)
            .await
    }

    pub async fn delete_webhook_message(
        &self,
        webhook_id: impl Into<Snowflake>,
        token: &str,
        message_id: &str,
    ) -> Result<(), DiscordError> {
        let path = webhook_message_path(webhook_id.into(), token, message_id)?;
        self.request_no_content(Method::DELETE, &path, Option::<&Value>::None)
            .await
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

    pub async fn create_interaction_response_with_files(
        &self,
        interaction_id: impl Into<Snowflake>,
        interaction_token: &str,
        body: &InteractionCallbackResponse,
        files: &[FileAttachment],
    ) -> Result<(), DiscordError> {
        let path = interaction_callback_path(interaction_id.into(), interaction_token)?;
        self.request_multipart_no_content(Method::POST, &path, body, files)
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

    pub async fn pin_message(
        &self,
        channel_id: impl Into<Snowflake>,
        message_id: impl Into<Snowflake>,
    ) -> Result<(), DiscordError> {
        self.request_no_content(
            Method::PUT,
            &format!("/channels/{}/pins/{}", channel_id.into(), message_id.into()),
            Option::<&Value>::None,
        )
        .await
    }

    pub async fn unpin_message(
        &self,
        channel_id: impl Into<Snowflake>,
        message_id: impl Into<Snowflake>,
    ) -> Result<(), DiscordError> {
        self.request_no_content(
            Method::DELETE,
            &format!("/channels/{}/pins/{}", channel_id.into(), message_id.into()),
            Option::<&Value>::None,
        )
        .await
    }

    pub async fn get_pinned_messages(
        &self,
        channel_id: impl Into<Snowflake>,
    ) -> Result<Vec<Message>, DiscordError> {
        self.request_typed(
            Method::GET,
            &format!("/channels/{}/pins", channel_id.into()),
            Option::<&Value>::None,
        )
        .await
    }

    pub async fn trigger_typing_indicator(
        &self,
        channel_id: impl Into<Snowflake>,
    ) -> Result<(), DiscordError> {
        self.request_no_content(
            Method::POST,
            &format!("/channels/{}/typing", channel_id.into()),
            Option::<&Value>::None,
        )
        .await
    }

    pub async fn crosspost_message(
        &self,
        channel_id: impl Into<Snowflake>,
        message_id: impl Into<Snowflake>,
    ) -> Result<Message, DiscordError> {
        self.request_typed(
            Method::POST,
            &format!(
                "/channels/{}/messages/{}/crosspost",
                channel_id.into(),
                message_id.into()
            ),
            Option::<&Value>::None,
        )
        .await
    }

    pub async fn get_channel_messages_paginated(
        &self,
        channel_id: impl Into<Snowflake>,
        limit: Option<u64>,
        before: Option<Snowflake>,
        after: Option<Snowflake>,
        around: Option<Snowflake>,
    ) -> Result<Vec<Message>, DiscordError> {
        let mut params = Vec::new();
        if let Some(l) = limit {
            params.push(format!("limit={l}"));
        }
        if let Some(b) = before {
            params.push(format!("before={b}"));
        }
        if let Some(a) = after {
            params.push(format!("after={a}"));
        }
        if let Some(ar) = around {
            params.push(format!("around={ar}"));
        }
        let path = if params.is_empty() {
            format!("/channels/{}/messages", channel_id.into())
        } else {
            format!(
                "/channels/{}/messages?{}",
                channel_id.into(),
                params.join("&")
            )
        };
        self.request_typed(Method::GET, &path, Option::<&Value>::None)
            .await
    }

    pub async fn get_reactions(
        &self,
        channel_id: impl Into<Snowflake>,
        message_id: impl Into<Snowflake>,
        emoji: &str,
        limit: Option<u64>,
        after: Option<Snowflake>,
    ) -> Result<Vec<User>, DiscordError> {
        let mut params = Vec::new();
        if let Some(l) = limit {
            params.push(format!("limit={l}"));
        }
        if let Some(a) = after {
            params.push(format!("after={a}"));
        }
        let query = if params.is_empty() {
            String::new()
        } else {
            format!("?{}", params.join("&"))
        };
        let path = format!(
            "/channels/{}/messages/{}/reactions/{}{}",
            channel_id.into(),
            message_id.into(),
            emoji,
            query
        );
        self.request_typed(Method::GET, &path, Option::<&Value>::None)
            .await
    }

    pub async fn remove_user_reaction(
        &self,
        channel_id: impl Into<Snowflake>,
        message_id: impl Into<Snowflake>,
        emoji: &str,
        user_id: impl Into<Snowflake>,
    ) -> Result<(), DiscordError> {
        let path = format!(
            "/channels/{}/messages/{}/reactions/{}/{}",
            channel_id.into(),
            message_id.into(),
            emoji,
            user_id.into()
        );
        self.request_no_content(Method::DELETE, &path, Option::<&Value>::None)
            .await
    }

    pub async fn remove_all_reactions(
        &self,
        channel_id: impl Into<Snowflake>,
        message_id: impl Into<Snowflake>,
    ) -> Result<(), DiscordError> {
        self.request_no_content(
            Method::DELETE,
            &format!(
                "/channels/{}/messages/{}/reactions",
                channel_id.into(),
                message_id.into()
            ),
            Option::<&Value>::None,
        )
        .await
    }

    pub async fn remove_all_reactions_for_emoji(
        &self,
        channel_id: impl Into<Snowflake>,
        message_id: impl Into<Snowflake>,
        emoji: &str,
    ) -> Result<(), DiscordError> {
        self.request_no_content(
            Method::DELETE,
            &format!(
                "/channels/{}/messages/{}/reactions/{}",
                channel_id.into(),
                message_id.into(),
                emoji
            ),
            Option::<&Value>::None,
        )
        .await
    }

    pub async fn edit_channel_permissions(
        &self,
        channel_id: impl Into<Snowflake>,
        overwrite_id: impl Into<Snowflake>,
        body: &Value,
    ) -> Result<(), DiscordError> {
        self.request_no_content(
            Method::PUT,
            &format!(
                "/channels/{}/permissions/{}",
                channel_id.into(),
                overwrite_id.into()
            ),
            Some(body),
        )
        .await
    }

    pub async fn delete_channel_permission(
        &self,
        channel_id: impl Into<Snowflake>,
        overwrite_id: impl Into<Snowflake>,
    ) -> Result<(), DiscordError> {
        self.request_no_content(
            Method::DELETE,
            &format!(
                "/channels/{}/permissions/{}",
                channel_id.into(),
                overwrite_id.into()
            ),
            Option::<&Value>::None,
        )
        .await
    }

    pub async fn create_thread_from_message(
        &self,
        channel_id: impl Into<Snowflake>,
        message_id: impl Into<Snowflake>,
        body: &Value,
    ) -> Result<Channel, DiscordError> {
        self.request_typed(
            Method::POST,
            &format!(
                "/channels/{}/messages/{}/threads",
                channel_id.into(),
                message_id.into()
            ),
            Some(body),
        )
        .await
    }

    pub async fn create_thread(
        &self,
        channel_id: impl Into<Snowflake>,
        body: &Value,
    ) -> Result<Channel, DiscordError> {
        self.request_typed(
            Method::POST,
            &format!("/channels/{}/threads", channel_id.into()),
            Some(body),
        )
        .await
    }

    pub async fn join_thread(&self, channel_id: impl Into<Snowflake>) -> Result<(), DiscordError> {
        self.request_no_content(
            Method::PUT,
            &format!("/channels/{}/thread-members/@me", channel_id.into()),
            Option::<&Value>::None,
        )
        .await
    }

    pub async fn add_thread_member(
        &self,
        channel_id: impl Into<Snowflake>,
        user_id: impl Into<Snowflake>,
    ) -> Result<(), DiscordError> {
        self.request_no_content(
            Method::PUT,
            &format!(
                "/channels/{}/thread-members/{}",
                channel_id.into(),
                user_id.into()
            ),
            Option::<&Value>::None,
        )
        .await
    }

    pub async fn leave_thread(&self, channel_id: impl Into<Snowflake>) -> Result<(), DiscordError> {
        self.request_no_content(
            Method::DELETE,
            &format!("/channels/{}/thread-members/@me", channel_id.into()),
            Option::<&Value>::None,
        )
        .await
    }

    pub async fn remove_thread_member(
        &self,
        channel_id: impl Into<Snowflake>,
        user_id: impl Into<Snowflake>,
    ) -> Result<(), DiscordError> {
        self.request_no_content(
            Method::DELETE,
            &format!(
                "/channels/{}/thread-members/{}",
                channel_id.into(),
                user_id.into()
            ),
            Option::<&Value>::None,
        )
        .await
    }

    pub async fn get_thread_member(
        &self,
        channel_id: impl Into<Snowflake>,
        user_id: impl Into<Snowflake>,
        with_member: Option<bool>,
    ) -> Result<ThreadMember, DiscordError> {
        let query = bool_query("with_member", with_member);
        self.request_typed(
            Method::GET,
            &format!(
                "/channels/{}/thread-members/{}{}",
                channel_id.into(),
                user_id.into(),
                query
            ),
            Option::<&Value>::None,
        )
        .await
    }

    pub async fn get_thread_members(
        &self,
        channel_id: impl Into<Snowflake>,
    ) -> Result<Vec<ThreadMember>, DiscordError> {
        self.list_thread_members(channel_id, &ThreadMemberQuery::default())
            .await
    }

    pub async fn list_thread_members(
        &self,
        channel_id: impl Into<Snowflake>,
        query: &ThreadMemberQuery,
    ) -> Result<Vec<ThreadMember>, DiscordError> {
        self.request_typed(
            Method::GET,
            &format!(
                "/channels/{}/thread-members{}",
                channel_id.into(),
                thread_member_query(query)
            ),
            Option::<&Value>::None,
        )
        .await
    }

    pub async fn get_public_archived_threads(
        &self,
        channel_id: impl Into<Snowflake>,
        limit: Option<u64>,
    ) -> Result<serde_json::Value, DiscordError> {
        let path = match limit {
            Some(l) => format!(
                "/channels/{}/threads/archived/public?limit={}",
                channel_id.into(),
                l
            ),
            None => format!("/channels/{}/threads/archived/public", channel_id.into()),
        };
        self.request(Method::GET, &path, Option::<&Value>::None)
            .await
    }

    pub async fn list_public_archived_threads(
        &self,
        channel_id: impl Into<Snowflake>,
        query: &ArchivedThreadsQuery,
    ) -> Result<ThreadListResponse, DiscordError> {
        self.request_typed(
            Method::GET,
            &format!(
                "/channels/{}/threads/archived/public{}",
                channel_id.into(),
                archived_threads_query(query)
            ),
            Option::<&Value>::None,
        )
        .await
    }

    pub async fn list_private_archived_threads(
        &self,
        channel_id: impl Into<Snowflake>,
        query: &ArchivedThreadsQuery,
    ) -> Result<ThreadListResponse, DiscordError> {
        self.request_typed(
            Method::GET,
            &format!(
                "/channels/{}/threads/archived/private{}",
                channel_id.into(),
                archived_threads_query(query)
            ),
            Option::<&Value>::None,
        )
        .await
    }

    pub async fn list_joined_private_archived_threads(
        &self,
        channel_id: impl Into<Snowflake>,
        query: &JoinedArchivedThreadsQuery,
    ) -> Result<ThreadListResponse, DiscordError> {
        self.request_typed(
            Method::GET,
            &format!(
                "/channels/{}/users/@me/threads/archived/private{}",
                channel_id.into(),
                joined_archived_threads_query(query)
            ),
            Option::<&Value>::None,
        )
        .await
    }

    pub async fn get_active_guild_threads(
        &self,
        guild_id: impl Into<Snowflake>,
    ) -> Result<ThreadListResponse, DiscordError> {
        self.request_typed(
            Method::GET,
            &format!("/guilds/{}/threads/active", guild_id.into()),
            Option::<&Value>::None,
        )
        .await
    }

    pub async fn get_guild_bans(
        &self,
        guild_id: impl Into<Snowflake>,
        limit: Option<u64>,
        before: Option<Snowflake>,
    ) -> Result<Vec<Ban>, DiscordError> {
        let mut params = Vec::new();
        if let Some(l) = limit {
            params.push(format!("limit={l}"));
        }
        if let Some(b) = before {
            params.push(format!("before={b}"));
        }
        let query = if params.is_empty() {
            String::new()
        } else {
            format!("?{}", params.join("&"))
        };
        self.request_typed(
            Method::GET,
            &format!("/guilds/{}/bans{}", guild_id.into(), query),
            Option::<&Value>::None,
        )
        .await
    }

    pub async fn get_guild_ban(
        &self,
        guild_id: impl Into<Snowflake>,
        user_id: impl Into<Snowflake>,
    ) -> Result<Ban, DiscordError> {
        self.request_typed(
            Method::GET,
            &format!("/guilds/{}/bans/{}", guild_id.into(), user_id.into()),
            Option::<&Value>::None,
        )
        .await
    }

    pub async fn create_guild_ban(
        &self,
        guild_id: impl Into<Snowflake>,
        user_id: impl Into<Snowflake>,
        body: &Value,
    ) -> Result<(), DiscordError> {
        self.request_no_content(
            Method::PUT,
            &format!("/guilds/{}/bans/{}", guild_id.into(), user_id.into()),
            Some(body),
        )
        .await
    }

    pub async fn remove_guild_ban(
        &self,
        guild_id: impl Into<Snowflake>,
        user_id: impl Into<Snowflake>,
    ) -> Result<(), DiscordError> {
        self.request_no_content(
            Method::DELETE,
            &format!("/guilds/{}/bans/{}", guild_id.into(), user_id.into()),
            Option::<&Value>::None,
        )
        .await
    }

    pub async fn modify_guild_member(
        &self,
        guild_id: impl Into<Snowflake>,
        user_id: impl Into<Snowflake>,
        body: &Value,
    ) -> Result<(), DiscordError> {
        self.request_no_content(
            Method::PATCH,
            &format!("/guilds/{}/members/{}", guild_id.into(), user_id.into()),
            Some(body),
        )
        .await
    }

    pub async fn modify_current_member_nick(
        &self,
        guild_id: impl Into<Snowflake>,
        nick: Option<&str>,
    ) -> Result<(), DiscordError> {
        let body = serde_json::json!({ "nick": nick });
        self.request_no_content(
            Method::PATCH,
            &format!("/guilds/{}/members/@me", guild_id.into()),
            Some(&body),
        )
        .await
    }

    pub async fn search_guild_members(
        &self,
        guild_id: impl Into<Snowflake>,
        query: &str,
        limit: Option<u64>,
    ) -> Result<Vec<Member>, DiscordError> {
        let encoded = query.replace(' ', "%20").replace('&', "%26");
        let mut params = vec![format!("query={encoded}")];
        if let Some(l) = limit {
            params.push(format!("limit={l}"));
        }
        let path = format!(
            "/guilds/{}/members/search?{}",
            guild_id.into(),
            params.join("&")
        );
        self.request_typed(Method::GET, &path, Option::<&Value>::None)
            .await
    }

    pub async fn get_guild_audit_log(
        &self,
        guild_id: impl Into<Snowflake>,
        user_id: Option<Snowflake>,
        action_type: Option<u64>,
        before: Option<Snowflake>,
        limit: Option<u64>,
    ) -> Result<serde_json::Value, DiscordError> {
        let mut params = Vec::new();
        if let Some(uid) = user_id {
            params.push(format!("user_id={uid}"));
        }
        if let Some(at) = action_type {
            params.push(format!("action_type={at}"));
        }
        if let Some(b) = before {
            params.push(format!("before={b}"));
        }
        if let Some(l) = limit {
            params.push(format!("limit={l}"));
        }
        let query = if params.is_empty() {
            String::new()
        } else {
            format!("?{}", params.join("&"))
        };
        self.request(
            Method::GET,
            &format!("/guilds/{}/audit-logs{}", guild_id.into(), query),
            Option::<&Value>::None,
        )
        .await
    }

    pub async fn modify_guild_role_positions(
        &self,
        guild_id: impl Into<Snowflake>,
        body: &Value,
    ) -> Result<Vec<Role>, DiscordError> {
        self.request_typed(
            Method::PATCH,
            &format!("/guilds/{}/roles", guild_id.into()),
            Some(body),
        )
        .await
    }

    pub async fn get_guild_emojis(
        &self,
        guild_id: impl Into<Snowflake>,
    ) -> Result<Vec<serde_json::Value>, DiscordError> {
        self.request_typed(
            Method::GET,
            &format!("/guilds/{}/emojis", guild_id.into()),
            Option::<&Value>::None,
        )
        .await
    }

    pub async fn get_guild_emoji(
        &self,
        guild_id: impl Into<Snowflake>,
        emoji_id: impl Into<Snowflake>,
    ) -> Result<serde_json::Value, DiscordError> {
        self.request(
            Method::GET,
            &format!("/guilds/{}/emojis/{}", guild_id.into(), emoji_id.into()),
            Option::<&Value>::None,
        )
        .await
    }

    pub async fn create_guild_emoji(
        &self,
        guild_id: impl Into<Snowflake>,
        body: &Value,
    ) -> Result<serde_json::Value, DiscordError> {
        self.request(
            Method::POST,
            &format!("/guilds/{}/emojis", guild_id.into()),
            Some(body),
        )
        .await
    }

    pub async fn modify_guild_emoji(
        &self,
        guild_id: impl Into<Snowflake>,
        emoji_id: impl Into<Snowflake>,
        body: &Value,
    ) -> Result<serde_json::Value, DiscordError> {
        self.request(
            Method::PATCH,
            &format!("/guilds/{}/emojis/{}", guild_id.into(), emoji_id.into()),
            Some(body),
        )
        .await
    }

    pub async fn delete_guild_emoji(
        &self,
        guild_id: impl Into<Snowflake>,
        emoji_id: impl Into<Snowflake>,
    ) -> Result<(), DiscordError> {
        self.request_no_content(
            Method::DELETE,
            &format!("/guilds/{}/emojis/{}", guild_id.into(), emoji_id.into()),
            Option::<&Value>::None,
        )
        .await
    }

    pub async fn get_application_emojis(&self) -> Result<Vec<serde_json::Value>, DiscordError> {
        let application_id = configured_application_id(self.application_id())?;
        let response = self
            .request(
                Method::GET,
                &format!("/applications/{application_id}/emojis"),
                Option::<&Value>::None,
            )
            .await?;
        Ok(response
            .get("items")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default())
    }

    pub async fn get_application_emoji(
        &self,
        emoji_id: impl Into<Snowflake>,
    ) -> Result<serde_json::Value, DiscordError> {
        let application_id = configured_application_id(self.application_id())?;
        self.request(
            Method::GET,
            &format!("/applications/{application_id}/emojis/{}", emoji_id.into()),
            Option::<&Value>::None,
        )
        .await
    }

    pub async fn create_application_emoji(
        &self,
        body: &Value,
    ) -> Result<serde_json::Value, DiscordError> {
        let application_id = configured_application_id(self.application_id())?;
        self.request(
            Method::POST,
            &format!("/applications/{application_id}/emojis"),
            Some(body),
        )
        .await
    }

    pub async fn modify_application_emoji(
        &self,
        emoji_id: impl Into<Snowflake>,
        body: &Value,
    ) -> Result<serde_json::Value, DiscordError> {
        let application_id = configured_application_id(self.application_id())?;
        self.request(
            Method::PATCH,
            &format!("/applications/{application_id}/emojis/{}", emoji_id.into()),
            Some(body),
        )
        .await
    }

    pub async fn delete_application_emoji(
        &self,
        emoji_id: impl Into<Snowflake>,
    ) -> Result<(), DiscordError> {
        let application_id = configured_application_id(self.application_id())?;
        self.request_no_content(
            Method::DELETE,
            &format!("/applications/{application_id}/emojis/{}", emoji_id.into()),
            Option::<&Value>::None,
        )
        .await
    }

    pub async fn get_skus(&self) -> Result<Vec<Sku>, DiscordError> {
        let application_id = configured_application_id(self.application_id())?;
        self.request_typed(
            Method::GET,
            &format!("/applications/{application_id}/skus"),
            Option::<&Value>::None,
        )
        .await
    }

    pub async fn get_sku_subscriptions(
        &self,
        sku_id: impl Into<Snowflake>,
        query: &SubscriptionQuery,
    ) -> Result<Vec<Subscription>, DiscordError> {
        self.request_typed(
            Method::GET,
            &format!(
                "/skus/{}/subscriptions{}",
                sku_id.into(),
                subscription_query(query)
            ),
            Option::<&Value>::None,
        )
        .await
    }

    pub async fn get_sku_subscription(
        &self,
        sku_id: impl Into<Snowflake>,
        subscription_id: impl Into<Snowflake>,
    ) -> Result<Subscription, DiscordError> {
        self.request_typed(
            Method::GET,
            &format!(
                "/skus/{}/subscriptions/{}",
                sku_id.into(),
                subscription_id.into()
            ),
            Option::<&Value>::None,
        )
        .await
    }

    pub async fn get_entitlements(
        &self,
        query: &EntitlementQuery,
    ) -> Result<Vec<Entitlement>, DiscordError> {
        let application_id = configured_application_id(self.application_id())?;
        self.request_typed(
            Method::GET,
            &format!(
                "/applications/{application_id}/entitlements{}",
                entitlement_query(query)
            ),
            Option::<&Value>::None,
        )
        .await
    }

    pub async fn get_entitlement(
        &self,
        entitlement_id: impl Into<Snowflake>,
    ) -> Result<Entitlement, DiscordError> {
        let application_id = configured_application_id(self.application_id())?;
        self.request_typed(
            Method::GET,
            &format!(
                "/applications/{application_id}/entitlements/{}",
                entitlement_id.into()
            ),
            Option::<&Value>::None,
        )
        .await
    }

    pub async fn consume_entitlement(
        &self,
        entitlement_id: impl Into<Snowflake>,
    ) -> Result<(), DiscordError> {
        let application_id = configured_application_id(self.application_id())?;
        self.request_no_content(
            Method::POST,
            &format!(
                "/applications/{application_id}/entitlements/{}/consume",
                entitlement_id.into()
            ),
            Option::<&Value>::None,
        )
        .await
    }

    pub async fn create_test_entitlement(
        &self,
        body: &CreateTestEntitlement,
    ) -> Result<Entitlement, DiscordError> {
        let application_id = configured_application_id(self.application_id())?;
        self.request_typed(
            Method::POST,
            &format!("/applications/{application_id}/entitlements"),
            Some(body),
        )
        .await
    }

    pub async fn delete_test_entitlement(
        &self,
        entitlement_id: impl Into<Snowflake>,
    ) -> Result<(), DiscordError> {
        let application_id = configured_application_id(self.application_id())?;
        self.request_no_content(
            Method::DELETE,
            &format!(
                "/applications/{application_id}/entitlements/{}",
                entitlement_id.into()
            ),
            Option::<&Value>::None,
        )
        .await
    }

    pub async fn get_sticker(
        &self,
        sticker_id: impl Into<Snowflake>,
    ) -> Result<Sticker, DiscordError> {
        self.request_typed(
            Method::GET,
            &format!("/stickers/{}", sticker_id.into()),
            Option::<&Value>::None,
        )
        .await
    }

    pub async fn list_sticker_packs(&self) -> Result<StickerPackList, DiscordError> {
        self.request_typed(Method::GET, "/sticker-packs", Option::<&Value>::None)
            .await
    }

    pub async fn get_guild_stickers(
        &self,
        guild_id: impl Into<Snowflake>,
    ) -> Result<Vec<Sticker>, DiscordError> {
        self.request_typed(
            Method::GET,
            &format!("/guilds/{}/stickers", guild_id.into()),
            Option::<&Value>::None,
        )
        .await
    }

    pub async fn get_guild_sticker(
        &self,
        guild_id: impl Into<Snowflake>,
        sticker_id: impl Into<Snowflake>,
    ) -> Result<Sticker, DiscordError> {
        self.request_typed(
            Method::GET,
            &format!("/guilds/{}/stickers/{}", guild_id.into(), sticker_id.into()),
            Option::<&Value>::None,
        )
        .await
    }

    pub async fn create_guild_sticker(
        &self,
        guild_id: impl Into<Snowflake>,
        body: &Value,
        file: FileAttachment,
    ) -> Result<Sticker, DiscordError> {
        let path = format!("/guilds/{}/stickers", guild_id.into());
        let response = self
            .request_with_headers(
                Method::POST,
                &path,
                Some(RequestBody::StickerMultipart {
                    payload_json: body.clone(),
                    file,
                }),
            )
            .await?;
        serde_json::from_value(parse_body_value(response.body)).map_err(Into::into)
    }

    pub async fn modify_guild_sticker(
        &self,
        guild_id: impl Into<Snowflake>,
        sticker_id: impl Into<Snowflake>,
        body: &Value,
    ) -> Result<Sticker, DiscordError> {
        self.request_typed(
            Method::PATCH,
            &format!("/guilds/{}/stickers/{}", guild_id.into(), sticker_id.into()),
            Some(body),
        )
        .await
    }

    pub async fn delete_guild_sticker(
        &self,
        guild_id: impl Into<Snowflake>,
        sticker_id: impl Into<Snowflake>,
    ) -> Result<(), DiscordError> {
        self.request_no_content(
            Method::DELETE,
            &format!("/guilds/{}/stickers/{}", guild_id.into(), sticker_id.into()),
            Option::<&Value>::None,
        )
        .await
    }

    pub async fn send_soundboard_sound(
        &self,
        channel_id: impl Into<Snowflake>,
        body: &Value,
    ) -> Result<(), DiscordError> {
        self.request_no_content(
            Method::POST,
            &format!("/channels/{}/send-soundboard-sound", channel_id.into()),
            Some(body),
        )
        .await
    }

    pub async fn list_default_soundboard_sounds(
        &self,
    ) -> Result<Vec<SoundboardSound>, DiscordError> {
        self.request_typed(
            Method::GET,
            "/soundboard-default-sounds",
            Option::<&Value>::None,
        )
        .await
    }

    pub async fn list_guild_soundboard_sounds(
        &self,
        guild_id: impl Into<Snowflake>,
    ) -> Result<SoundboardSoundList, DiscordError> {
        self.request_typed(
            Method::GET,
            &format!("/guilds/{}/soundboard-sounds", guild_id.into()),
            Option::<&Value>::None,
        )
        .await
    }

    pub async fn get_guild_soundboard_sound(
        &self,
        guild_id: impl Into<Snowflake>,
        sound_id: impl Into<Snowflake>,
    ) -> Result<SoundboardSound, DiscordError> {
        self.request_typed(
            Method::GET,
            &format!(
                "/guilds/{}/soundboard-sounds/{}",
                guild_id.into(),
                sound_id.into()
            ),
            Option::<&Value>::None,
        )
        .await
    }

    pub async fn create_guild_soundboard_sound(
        &self,
        guild_id: impl Into<Snowflake>,
        body: &Value,
    ) -> Result<SoundboardSound, DiscordError> {
        self.request_typed(
            Method::POST,
            &format!("/guilds/{}/soundboard-sounds", guild_id.into()),
            Some(body),
        )
        .await
    }

    pub async fn modify_guild_soundboard_sound(
        &self,
        guild_id: impl Into<Snowflake>,
        sound_id: impl Into<Snowflake>,
        body: &Value,
    ) -> Result<SoundboardSound, DiscordError> {
        self.request_typed(
            Method::PATCH,
            &format!(
                "/guilds/{}/soundboard-sounds/{}",
                guild_id.into(),
                sound_id.into()
            ),
            Some(body),
        )
        .await
    }

    pub async fn delete_guild_soundboard_sound(
        &self,
        guild_id: impl Into<Snowflake>,
        sound_id: impl Into<Snowflake>,
    ) -> Result<(), DiscordError> {
        self.request_no_content(
            Method::DELETE,
            &format!(
                "/guilds/{}/soundboard-sounds/{}",
                guild_id.into(),
                sound_id.into()
            ),
            Option::<&Value>::None,
        )
        .await
    }

    pub async fn get_guild_invites(
        &self,
        guild_id: impl Into<Snowflake>,
    ) -> Result<Vec<Invite>, DiscordError> {
        self.request_typed(
            Method::GET,
            &format!("/guilds/{}/invites", guild_id.into()),
            Option::<&Value>::None,
        )
        .await
    }

    pub async fn get_invite(&self, code: &str) -> Result<Invite, DiscordError> {
        self.request_typed(
            Method::GET,
            &format!("/invites/{code}"),
            Option::<&Value>::None,
        )
        .await
    }

    pub async fn get_invite_with_options(
        &self,
        code: &str,
        with_counts: Option<bool>,
        with_expiration: Option<bool>,
        guild_scheduled_event_id: Option<Snowflake>,
    ) -> Result<Invite, DiscordError> {
        self.request_typed(
            Method::GET,
            &format!(
                "/invites/{code}{}",
                invite_query(with_counts, with_expiration, guild_scheduled_event_id)
            ),
            Option::<&Value>::None,
        )
        .await
    }

    pub async fn delete_invite(&self, code: &str) -> Result<Invite, DiscordError> {
        self.request_typed(
            Method::DELETE,
            &format!("/invites/{code}"),
            Option::<&Value>::None,
        )
        .await
    }

    pub async fn get_guild_integrations(
        &self,
        guild_id: impl Into<Snowflake>,
    ) -> Result<Vec<Integration>, DiscordError> {
        self.request_typed(
            Method::GET,
            &format!("/guilds/{}/integrations", guild_id.into()),
            Option::<&Value>::None,
        )
        .await
    }

    pub async fn delete_guild_integration(
        &self,
        guild_id: impl Into<Snowflake>,
        integration_id: impl Into<Snowflake>,
    ) -> Result<(), DiscordError> {
        self.request_no_content(
            Method::DELETE,
            &format!(
                "/guilds/{}/integrations/{}",
                guild_id.into(),
                integration_id.into()
            ),
            Option::<&Value>::None,
        )
        .await
    }

    pub async fn get_poll_answer_voters(
        &self,
        channel_id: impl Into<Snowflake>,
        message_id: impl Into<Snowflake>,
        answer_id: u64,
        after: Option<Snowflake>,
        limit: Option<u64>,
    ) -> Result<PollAnswerVoters, DiscordError> {
        self.request_typed(
            Method::GET,
            &format!(
                "/channels/{}/polls/{}/answers/{answer_id}{}",
                channel_id.into(),
                message_id.into(),
                poll_answer_voters_query(after, limit)
            ),
            Option::<&Value>::None,
        )
        .await
    }

    pub async fn end_poll(
        &self,
        channel_id: impl Into<Snowflake>,
        message_id: impl Into<Snowflake>,
    ) -> Result<Message, DiscordError> {
        self.request_typed(
            Method::POST,
            &format!(
                "/channels/{}/polls/{}/expire",
                channel_id.into(),
                message_id.into()
            ),
            Option::<&Value>::None,
        )
        .await
    }

    pub async fn create_channel_invite(
        &self,
        channel_id: impl Into<Snowflake>,
        body: Option<&Value>,
    ) -> Result<Invite, DiscordError> {
        self.request_typed(
            Method::POST,
            &format!("/channels/{}/invites", channel_id.into()),
            body,
        )
        .await
    }

    pub async fn get_current_user(&self) -> Result<User, DiscordError> {
        self.request_typed(Method::GET, "/users/@me", Option::<&Value>::None)
            .await
    }

    pub async fn get_user(&self, user_id: impl Into<Snowflake>) -> Result<User, DiscordError> {
        self.request_typed(
            Method::GET,
            &format!("/users/{}", user_id.into()),
            Option::<&Value>::None,
        )
        .await
    }

    pub async fn get_current_user_guilds(&self) -> Result<Vec<serde_json::Value>, DiscordError> {
        self.request_typed(Method::GET, "/users/@me/guilds", Option::<&Value>::None)
            .await
    }

    pub async fn leave_guild(&self, guild_id: impl Into<Snowflake>) -> Result<(), DiscordError> {
        self.request_no_content(
            Method::DELETE,
            &format!("/users/@me/guilds/{}", guild_id.into()),
            Option::<&Value>::None,
        )
        .await
    }

    pub async fn get_guild_webhooks(
        &self,
        guild_id: impl Into<Snowflake>,
    ) -> Result<Vec<Webhook>, DiscordError> {
        self.request_typed(
            Method::GET,
            &format!("/guilds/{}/webhooks", guild_id.into()),
            Option::<&Value>::None,
        )
        .await
    }

    pub async fn get_webhook(
        &self,
        webhook_id: impl Into<Snowflake>,
    ) -> Result<Webhook, DiscordError> {
        self.request_typed(
            Method::GET,
            &format!("/webhooks/{}", webhook_id.into()),
            Option::<&Value>::None,
        )
        .await
    }

    pub async fn modify_webhook(
        &self,
        webhook_id: impl Into<Snowflake>,
        body: &Value,
    ) -> Result<Webhook, DiscordError> {
        self.request_typed(
            Method::PATCH,
            &format!("/webhooks/{}", webhook_id.into()),
            Some(body),
        )
        .await
    }

    pub async fn delete_webhook(
        &self,
        webhook_id: impl Into<Snowflake>,
    ) -> Result<(), DiscordError> {
        self.request_no_content(
            Method::DELETE,
            &format!("/webhooks/{}", webhook_id.into()),
            Option::<&Value>::None,
        )
        .await
    }

    pub async fn modify_webhook_with_token(
        &self,
        webhook_id: impl Into<Snowflake>,
        token: &str,
        body: &Value,
    ) -> Result<Webhook, DiscordError> {
        let path = format!("/webhooks/{}/{}", webhook_id.into(), token);
        self.request_typed(Method::PATCH, &path, Some(body)).await
    }

    pub async fn delete_webhook_with_token(
        &self,
        webhook_id: impl Into<Snowflake>,
        token: &str,
    ) -> Result<(), DiscordError> {
        let path = format!("/webhooks/{}/{}", webhook_id.into(), token);
        self.request_no_content(Method::DELETE, &path, Option::<&Value>::None)
            .await
    }

    pub async fn get_guild_scheduled_events(
        &self,
        guild_id: impl Into<Snowflake>,
    ) -> Result<Vec<GuildScheduledEvent>, DiscordError> {
        self.request_typed(
            Method::GET,
            &format!("/guilds/{}/scheduled-events", guild_id.into()),
            Option::<&Value>::None,
        )
        .await
    }

    pub async fn create_guild_scheduled_event(
        &self,
        guild_id: impl Into<Snowflake>,
        body: &Value,
    ) -> Result<GuildScheduledEvent, DiscordError> {
        self.request_typed(
            Method::POST,
            &format!("/guilds/{}/scheduled-events", guild_id.into()),
            Some(body),
        )
        .await
    }

    pub async fn get_guild_scheduled_event(
        &self,
        guild_id: impl Into<Snowflake>,
        event_id: impl Into<Snowflake>,
    ) -> Result<GuildScheduledEvent, DiscordError> {
        self.request_typed(
            Method::GET,
            &format!(
                "/guilds/{}/scheduled-events/{}",
                guild_id.into(),
                event_id.into()
            ),
            Option::<&Value>::None,
        )
        .await
    }

    pub async fn modify_guild_scheduled_event(
        &self,
        guild_id: impl Into<Snowflake>,
        event_id: impl Into<Snowflake>,
        body: &Value,
    ) -> Result<GuildScheduledEvent, DiscordError> {
        self.request_typed(
            Method::PATCH,
            &format!(
                "/guilds/{}/scheduled-events/{}",
                guild_id.into(),
                event_id.into()
            ),
            Some(body),
        )
        .await
    }

    pub async fn delete_guild_scheduled_event(
        &self,
        guild_id: impl Into<Snowflake>,
        event_id: impl Into<Snowflake>,
    ) -> Result<(), DiscordError> {
        self.request_no_content(
            Method::DELETE,
            &format!(
                "/guilds/{}/scheduled-events/{}",
                guild_id.into(),
                event_id.into()
            ),
            Option::<&Value>::None,
        )
        .await
    }

    pub async fn get_guild_scheduled_event_users(
        &self,
        guild_id: impl Into<Snowflake>,
        event_id: impl Into<Snowflake>,
        limit: Option<u64>,
    ) -> Result<Vec<GuildScheduledEventUser>, DiscordError> {
        let path = match limit {
            Some(l) => format!(
                "/guilds/{}/scheduled-events/{}/users?limit={}",
                guild_id.into(),
                event_id.into(),
                l
            ),
            None => format!(
                "/guilds/{}/scheduled-events/{}/users",
                guild_id.into(),
                event_id.into()
            ),
        };
        self.request_typed(Method::GET, &path, Option::<&Value>::None)
            .await
    }

    pub async fn get_auto_moderation_rules(
        &self,
        guild_id: impl Into<Snowflake>,
    ) -> Result<Vec<serde_json::Value>, DiscordError> {
        self.request_typed(
            Method::GET,
            &format!("/guilds/{}/auto-moderation/rules", guild_id.into()),
            Option::<&Value>::None,
        )
        .await
    }

    pub async fn create_auto_moderation_rule(
        &self,
        guild_id: impl Into<Snowflake>,
        body: &Value,
    ) -> Result<serde_json::Value, DiscordError> {
        self.request(
            Method::POST,
            &format!("/guilds/{}/auto-moderation/rules", guild_id.into()),
            Some(body),
        )
        .await
    }

    pub async fn modify_auto_moderation_rule(
        &self,
        guild_id: impl Into<Snowflake>,
        rule_id: impl Into<Snowflake>,
        body: &Value,
    ) -> Result<serde_json::Value, DiscordError> {
        self.request(
            Method::PATCH,
            &format!(
                "/guilds/{}/auto-moderation/rules/{}",
                guild_id.into(),
                rule_id.into()
            ),
            Some(body),
        )
        .await
    }

    pub async fn delete_auto_moderation_rule(
        &self,
        guild_id: impl Into<Snowflake>,
        rule_id: impl Into<Snowflake>,
    ) -> Result<(), DiscordError> {
        self.request_no_content(
            Method::DELETE,
            &format!(
                "/guilds/{}/auto-moderation/rules/{}",
                guild_id.into(),
                rule_id.into()
            ),
            Option::<&Value>::None,
        )
        .await
    }

    pub async fn get_global_command(
        &self,
        command_id: impl Into<Snowflake>,
    ) -> Result<ApplicationCommand, DiscordError> {
        let path = global_commands_path(self.application_id())?;
        self.request_typed(
            Method::GET,
            &format!("{}/{}", path, command_id.into()),
            Option::<&Value>::None,
        )
        .await
    }

    pub async fn edit_global_command(
        &self,
        command_id: impl Into<Snowflake>,
        body: &Value,
    ) -> Result<ApplicationCommand, DiscordError> {
        let path = global_commands_path(self.application_id())?;
        self.request_typed(
            Method::PATCH,
            &format!("{}/{}", path, command_id.into()),
            Some(body),
        )
        .await
    }

    pub async fn delete_global_command(
        &self,
        command_id: impl Into<Snowflake>,
    ) -> Result<(), DiscordError> {
        let path = global_commands_path(self.application_id())?;
        self.request_no_content(
            Method::DELETE,
            &format!("{}/{}", path, command_id.into()),
            Option::<&Value>::None,
        )
        .await
    }

    pub async fn get_guild_commands(
        &self,
        guild_id: impl Into<Snowflake>,
    ) -> Result<Vec<ApplicationCommand>, DiscordError> {
        let application_id = configured_application_id(self.application_id())?;
        self.request_typed(
            Method::GET,
            &format!(
                "/applications/{application_id}/guilds/{}/commands",
                guild_id.into()
            ),
            Option::<&Value>::None,
        )
        .await
    }

    pub async fn create_guild_command(
        &self,
        guild_id: impl Into<Snowflake>,
        command: &CommandDefinition,
    ) -> Result<ApplicationCommand, DiscordError> {
        let application_id = configured_application_id(self.application_id())?;
        self.request_typed(
            Method::POST,
            &format!(
                "/applications/{application_id}/guilds/{}/commands",
                guild_id.into()
            ),
            Some(command),
        )
        .await
    }

    pub async fn edit_guild_command(
        &self,
        guild_id: impl Into<Snowflake>,
        command_id: impl Into<Snowflake>,
        body: &Value,
    ) -> Result<ApplicationCommand, DiscordError> {
        let application_id = configured_application_id(self.application_id())?;
        self.request_typed(
            Method::PATCH,
            &format!(
                "/applications/{application_id}/guilds/{}/commands/{}",
                guild_id.into(),
                command_id.into()
            ),
            Some(body),
        )
        .await
    }

    pub async fn delete_guild_command(
        &self,
        guild_id: impl Into<Snowflake>,
        command_id: impl Into<Snowflake>,
    ) -> Result<(), DiscordError> {
        let application_id = configured_application_id(self.application_id())?;
        self.request_no_content(
            Method::DELETE,
            &format!(
                "/applications/{application_id}/guilds/{}/commands/{}",
                guild_id.into(),
                command_id.into()
            ),
            Option::<&Value>::None,
        )
        .await
    }

    pub async fn get_guild_preview(
        &self,
        guild_id: impl Into<Snowflake>,
    ) -> Result<serde_json::Value, DiscordError> {
        self.request(
            Method::GET,
            &format!("/guilds/{}/preview", guild_id.into()),
            Option::<&Value>::None,
        )
        .await
    }

    pub async fn get_guild_prune_count(
        &self,
        guild_id: impl Into<Snowflake>,
        days: Option<u64>,
        include_roles: &[Snowflake],
    ) -> Result<serde_json::Value, DiscordError> {
        let query = guild_prune_query(days, None, include_roles);
        self.request(
            Method::GET,
            &format!("/guilds/{}/prune{query}", guild_id.into()),
            Option::<&Value>::None,
        )
        .await
    }

    pub async fn begin_guild_prune(
        &self,
        guild_id: impl Into<Snowflake>,
        days: Option<u64>,
        compute_prune_count: Option<bool>,
        include_roles: &[Snowflake],
    ) -> Result<serde_json::Value, DiscordError> {
        let query = guild_prune_query(days, compute_prune_count, include_roles);
        self.request(
            Method::POST,
            &format!("/guilds/{}/prune{query}", guild_id.into()),
            Option::<&Value>::None,
        )
        .await
    }

    pub async fn get_guild_widget_settings(
        &self,
        guild_id: impl Into<Snowflake>,
    ) -> Result<GuildWidgetSettings, DiscordError> {
        self.request_typed(
            Method::GET,
            &format!("/guilds/{}/widget", guild_id.into()),
            Option::<&Value>::None,
        )
        .await
    }

    pub async fn modify_guild_widget_settings(
        &self,
        guild_id: impl Into<Snowflake>,
        body: &Value,
    ) -> Result<GuildWidgetSettings, DiscordError> {
        self.request_typed(
            Method::PATCH,
            &format!("/guilds/{}/widget", guild_id.into()),
            Some(body),
        )
        .await
    }

    pub async fn get_guild_widget(
        &self,
        guild_id: impl Into<Snowflake>,
    ) -> Result<serde_json::Value, DiscordError> {
        self.request(
            Method::GET,
            &format!("/guilds/{}/widget.json", guild_id.into()),
            Option::<&Value>::None,
        )
        .await
    }

    pub async fn follow_announcement_channel(
        &self,
        channel_id: impl Into<Snowflake>,
        webhook_channel_id: impl Into<Snowflake>,
    ) -> Result<FollowedChannel, DiscordError> {
        self.request_typed(
            Method::POST,
            &format!("/channels/{}/followers", channel_id.into()),
            Some(&serde_json::json!({ "webhook_channel_id": webhook_channel_id.into() })),
        )
        .await
    }

    pub async fn create_stage_instance(&self, body: &Value) -> Result<StageInstance, DiscordError> {
        self.request_typed(Method::POST, "/stage-instances", Some(body))
            .await
    }

    pub async fn get_stage_instance(
        &self,
        channel_id: impl Into<Snowflake>,
    ) -> Result<StageInstance, DiscordError> {
        self.request_typed(
            Method::GET,
            &format!("/stage-instances/{}", channel_id.into()),
            Option::<&Value>::None,
        )
        .await
    }

    pub async fn modify_stage_instance(
        &self,
        channel_id: impl Into<Snowflake>,
        body: &Value,
    ) -> Result<StageInstance, DiscordError> {
        self.request_typed(
            Method::PATCH,
            &format!("/stage-instances/{}", channel_id.into()),
            Some(body),
        )
        .await
    }

    pub async fn delete_stage_instance(
        &self,
        channel_id: impl Into<Snowflake>,
    ) -> Result<(), DiscordError> {
        self.request_no_content(
            Method::DELETE,
            &format!("/stage-instances/{}", channel_id.into()),
            Option::<&Value>::None,
        )
        .await
    }

    pub async fn get_guild_welcome_screen(
        &self,
        guild_id: impl Into<Snowflake>,
    ) -> Result<WelcomeScreen, DiscordError> {
        self.request_typed(
            Method::GET,
            &format!("/guilds/{}/welcome-screen", guild_id.into()),
            Option::<&Value>::None,
        )
        .await
    }

    pub async fn modify_guild_welcome_screen(
        &self,
        guild_id: impl Into<Snowflake>,
        body: &Value,
    ) -> Result<WelcomeScreen, DiscordError> {
        self.request_typed(
            Method::PATCH,
            &format!("/guilds/{}/welcome-screen", guild_id.into()),
            Some(body),
        )
        .await
    }

    pub async fn get_guild_onboarding(
        &self,
        guild_id: impl Into<Snowflake>,
    ) -> Result<GuildOnboarding, DiscordError> {
        self.request_typed(
            Method::GET,
            &format!("/guilds/{}/onboarding", guild_id.into()),
            Option::<&Value>::None,
        )
        .await
    }

    pub async fn modify_guild_onboarding(
        &self,
        guild_id: impl Into<Snowflake>,
        body: &Value,
    ) -> Result<GuildOnboarding, DiscordError> {
        self.request_typed(
            Method::PUT,
            &format!("/guilds/{}/onboarding", guild_id.into()),
            Some(body),
        )
        .await
    }

    pub async fn get_guild_templates(
        &self,
        guild_id: impl Into<Snowflake>,
    ) -> Result<Vec<GuildTemplate>, DiscordError> {
        self.request_typed(
            Method::GET,
            &format!("/guilds/{}/templates", guild_id.into()),
            Option::<&Value>::None,
        )
        .await
    }

    pub async fn create_guild_template(
        &self,
        guild_id: impl Into<Snowflake>,
        body: &Value,
    ) -> Result<GuildTemplate, DiscordError> {
        self.request_typed(
            Method::POST,
            &format!("/guilds/{}/templates", guild_id.into()),
            Some(body),
        )
        .await
    }

    pub async fn sync_guild_template(
        &self,
        guild_id: impl Into<Snowflake>,
        template_code: &str,
    ) -> Result<GuildTemplate, DiscordError> {
        validate_token_path_segment("template_code", template_code, false)?;
        self.request_typed(
            Method::PUT,
            &format!("/guilds/{}/templates/{template_code}", guild_id.into()),
            Option::<&Value>::None,
        )
        .await
    }

    pub async fn modify_guild_template(
        &self,
        guild_id: impl Into<Snowflake>,
        template_code: &str,
        body: &Value,
    ) -> Result<GuildTemplate, DiscordError> {
        validate_token_path_segment("template_code", template_code, false)?;
        self.request_typed(
            Method::PATCH,
            &format!("/guilds/{}/templates/{template_code}", guild_id.into()),
            Some(body),
        )
        .await
    }

    pub async fn delete_guild_template(
        &self,
        guild_id: impl Into<Snowflake>,
        template_code: &str,
    ) -> Result<GuildTemplate, DiscordError> {
        validate_token_path_segment("template_code", template_code, false)?;
        self.request_typed(
            Method::DELETE,
            &format!("/guilds/{}/templates/{template_code}", guild_id.into()),
            Option::<&Value>::None,
        )
        .await
    }

    pub async fn get_voice_regions(&self) -> Result<Vec<serde_json::Value>, DiscordError> {
        self.request_typed(Method::GET, "/voice/regions", Option::<&Value>::None)
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

    pub async fn create_followup_message_with_files(
        &self,
        interaction_token: &str,
        body: &CreateMessage,
        files: &[FileAttachment],
    ) -> Result<Message, DiscordError> {
        let application_id = configured_application_id(self.application_id())?;
        self.create_followup_message_with_application_id_and_files(
            &application_id,
            interaction_token,
            body,
            files,
        )
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

    pub async fn create_followup_message_with_application_id_and_files(
        &self,
        application_id: &str,
        interaction_token: &str,
        body: &CreateMessage,
        files: &[FileAttachment],
    ) -> Result<Message, DiscordError> {
        let path = followup_webhook_path(application_id, interaction_token, None)?;
        self.request_typed_multipart(Method::POST, &path, body, files)
            .await
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

    pub async fn edit_original_interaction_response_with_files(
        &self,
        interaction_token: &str,
        body: &CreateMessage,
        files: &[FileAttachment],
    ) -> Result<Message, DiscordError> {
        let application_id = configured_application_id(self.application_id())?;
        self.edit_original_interaction_response_with_application_id_and_files(
            &application_id,
            interaction_token,
            body,
            files,
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

    pub async fn edit_original_interaction_response_with_application_id_and_files(
        &self,
        application_id: &str,
        interaction_token: &str,
        body: &CreateMessage,
        files: &[FileAttachment],
    ) -> Result<Message, DiscordError> {
        let path = followup_webhook_path(application_id, interaction_token, Some("@original"))?;
        self.request_typed_multipart(Method::PATCH, &path, body, files)
            .await
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

    pub async fn edit_followup_message_with_files(
        &self,
        interaction_token: &str,
        message_id: &str,
        body: &CreateMessage,
        files: &[FileAttachment],
    ) -> Result<Message, DiscordError> {
        let application_id = configured_application_id(self.application_id())?;
        self.edit_followup_message_with_application_id_and_files(
            &application_id,
            interaction_token,
            message_id,
            body,
            files,
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

    pub async fn edit_followup_message_with_application_id_and_files(
        &self,
        application_id: &str,
        interaction_token: &str,
        message_id: &str,
        body: &CreateMessage,
        files: &[FileAttachment],
    ) -> Result<Message, DiscordError> {
        let path = followup_webhook_path(application_id, interaction_token, Some(message_id))?;
        self.request_typed_multipart(Method::PATCH, &path, body, files)
            .await
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
            .request_with_headers(
                method,
                path,
                body.map(clone_json_body).map(RequestBody::Json),
            )
            .await?;
        Ok(parse_body_value(response.body))
    }

    pub async fn request_multipart<B>(
        &self,
        method: Method,
        path: &str,
        body: &B,
        files: &[FileAttachment],
    ) -> Result<Value, DiscordError>
    where
        B: serde::Serialize + ?Sized,
    {
        let response = self
            .request_with_headers(method, path, Some(multipart_body(body, files)))
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
            .request_with_headers(
                method,
                path,
                body.map(serialize_body).map(RequestBody::Json),
            )
            .await?;
        let value = parse_body_value(response.body);
        serde_json::from_value(value).map_err(Into::into)
    }

    pub async fn request_typed_multipart<T, B>(
        &self,
        method: Method,
        path: &str,
        body: &B,
        files: &[FileAttachment],
    ) -> Result<T, DiscordError>
    where
        T: DeserializeOwned,
        B: serde::Serialize + ?Sized,
    {
        let response = self
            .request_with_headers(method, path, Some(multipart_body(body, files)))
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
        self.request_with_headers(
            method,
            path,
            body.map(serialize_body).map(RequestBody::Json),
        )
        .await?;
        Ok(())
    }

    async fn request_multipart_no_content<B>(
        &self,
        method: Method,
        path: &str,
        body: &B,
        files: &[FileAttachment],
    ) -> Result<(), DiscordError>
    where
        B: serde::Serialize + ?Sized,
    {
        self.request_with_headers(method, path, Some(multipart_body(body, files)))
            .await?;
        Ok(())
    }

    async fn request_with_headers(
        &self,
        method: Method,
        path: &str,
        body: Option<RequestBody>,
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
        body: Option<&RequestBody>,
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
            .header(
                "User-Agent",
                concat!("DiscordBot (discordrs, ", env!("CARGO_PKG_VERSION"), ")"),
            );

        if !matches!(
            body,
            Some(RequestBody::Multipart { .. } | RequestBody::StickerMultipart { .. })
        ) {
            request_builder = request_builder.header("Content-Type", "application/json");
        }

        if request_uses_bot_authorization(&normalized_path) {
            request_builder =
                request_builder.header("Authorization", format!("Bot {}", self.token));
        }

        if let Some(body) = body {
            request_builder = match body {
                RequestBody::Json(value) => request_builder.json(value),
                RequestBody::Multipart {
                    payload_json,
                    files,
                } => request_builder.multipart(build_multipart_form(payload_json, files)?),
                RequestBody::StickerMultipart { payload_json, file } => {
                    request_builder.multipart(build_sticker_form(payload_json, file)?)
                }
            };
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

enum RequestBody {
    Json(Value),
    Multipart {
        payload_json: Value,
        files: Vec<FileAttachment>,
    },
    StickerMultipart {
        payload_json: Value,
        file: FileAttachment,
    },
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

fn multipart_body<T: serde::Serialize + ?Sized>(body: &T, files: &[FileAttachment]) -> RequestBody {
    RequestBody::Multipart {
        payload_json: serialize_body(body),
        files: files.to_vec(),
    }
}

fn build_multipart_form(
    payload_json: &Value,
    files: &[FileAttachment],
) -> Result<Form, DiscordError> {
    let payload_json = serde_json::to_string(payload_json)?;
    let mut form = Form::new().text("payload_json", payload_json);

    for (index, file) in files.iter().enumerate() {
        if file.filename.trim().is_empty() {
            return Err(invalid_data_error("file filename must not be empty"));
        }

        let mut part = Part::bytes(file.data.clone()).file_name(file.filename.clone());
        if let Some(content_type) = &file.content_type {
            part = part.mime_str(content_type)?;
        }
        form = form.part(format!("files[{index}]"), part);
    }

    Ok(form)
}

fn build_sticker_form(payload_json: &Value, file: &FileAttachment) -> Result<Form, DiscordError> {
    if file.filename.trim().is_empty() {
        return Err(invalid_data_error("file filename must not be empty"));
    }

    let mut form = Form::new();
    for field in ["name", "description", "tags"] {
        if let Some(value) = payload_json.get(field).and_then(Value::as_str) {
            form = form.text(field.to_string(), value.to_string());
        }
    }

    let mut part = Part::bytes(file.data.clone()).file_name(file.filename.clone());
    if let Some(content_type) = &file.content_type {
        part = part.mime_str(content_type)?;
    }
    Ok(form.part("file", part))
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

fn webhook_message_path(
    webhook_id: Snowflake,
    token: &str,
    message_id: &str,
) -> Result<String, DiscordError> {
    validate_token_path_segment("webhook_token", token, false)?;
    validate_token_path_segment("message_id", message_id, true)?;
    Ok(format!(
        "/webhooks/{webhook_id}/{token}/messages/{message_id}"
    ))
}

fn guild_prune_query(
    days: Option<u64>,
    compute_prune_count: Option<bool>,
    include_roles: &[Snowflake],
) -> String {
    let mut params = Vec::new();
    if let Some(days) = days {
        params.push(format!("days={days}"));
    }
    if let Some(compute_prune_count) = compute_prune_count {
        params.push(format!("compute_prune_count={compute_prune_count}"));
    }
    if !include_roles.is_empty() {
        let roles = include_roles
            .iter()
            .map(Snowflake::as_str)
            .collect::<Vec<_>>()
            .join(",");
        params.push(format!("include_roles={roles}"));
    }

    if params.is_empty() {
        String::new()
    } else {
        format!("?{}", params.join("&"))
    }
}

fn query_string(params: Vec<String>) -> String {
    if params.is_empty() {
        String::new()
    } else {
        format!("?{}", params.join("&"))
    }
}

fn bool_query(name: &str, value: Option<bool>) -> String {
    query_string(
        value
            .map(|value| vec![format!("{name}={value}")])
            .unwrap_or_default(),
    )
}

fn thread_member_query(query: &ThreadMemberQuery) -> String {
    let mut params = Vec::new();
    if let Some(with_member) = query.with_member {
        params.push(format!("with_member={with_member}"));
    }
    if let Some(after) = &query.after {
        params.push(format!("after={after}"));
    }
    if let Some(limit) = query.limit {
        params.push(format!("limit={limit}"));
    }
    query_string(params)
}

fn archived_threads_query(query: &ArchivedThreadsQuery) -> String {
    let mut params = Vec::new();
    if let Some(before) = &query.before {
        params.push(format!("before={before}"));
    }
    if let Some(limit) = query.limit {
        params.push(format!("limit={limit}"));
    }
    query_string(params)
}

fn joined_archived_threads_query(query: &JoinedArchivedThreadsQuery) -> String {
    let mut params = Vec::new();
    if let Some(before) = &query.before {
        params.push(format!("before={before}"));
    }
    if let Some(limit) = query.limit {
        params.push(format!("limit={limit}"));
    }
    query_string(params)
}

fn entitlement_query(query: &EntitlementQuery) -> String {
    let mut params = Vec::new();
    if let Some(user_id) = &query.user_id {
        params.push(format!("user_id={user_id}"));
    }
    if !query.sku_ids.is_empty() {
        let sku_ids = query
            .sku_ids
            .iter()
            .map(Snowflake::as_str)
            .collect::<Vec<_>>()
            .join(",");
        params.push(format!("sku_ids={sku_ids}"));
    }
    if let Some(before) = &query.before {
        params.push(format!("before={before}"));
    }
    if let Some(after) = &query.after {
        params.push(format!("after={after}"));
    }
    if let Some(limit) = query.limit {
        params.push(format!("limit={limit}"));
    }
    if let Some(guild_id) = &query.guild_id {
        params.push(format!("guild_id={guild_id}"));
    }
    if let Some(exclude_ended) = query.exclude_ended {
        params.push(format!("exclude_ended={exclude_ended}"));
    }
    if let Some(exclude_deleted) = query.exclude_deleted {
        params.push(format!("exclude_deleted={exclude_deleted}"));
    }

    query_string(params)
}

fn subscription_query(query: &SubscriptionQuery) -> String {
    let mut params = Vec::new();
    if let Some(before) = &query.before {
        params.push(format!("before={before}"));
    }
    if let Some(after) = &query.after {
        params.push(format!("after={after}"));
    }
    if let Some(limit) = query.limit {
        params.push(format!("limit={limit}"));
    }
    if let Some(user_id) = &query.user_id {
        params.push(format!("user_id={user_id}"));
    }
    query_string(params)
}

fn invite_query(
    with_counts: Option<bool>,
    with_expiration: Option<bool>,
    guild_scheduled_event_id: Option<Snowflake>,
) -> String {
    let mut params = Vec::new();
    if let Some(with_counts) = with_counts {
        params.push(format!("with_counts={with_counts}"));
    }
    if let Some(with_expiration) = with_expiration {
        params.push(format!("with_expiration={with_expiration}"));
    }
    if let Some(guild_scheduled_event_id) = guild_scheduled_event_id {
        params.push(format!(
            "guild_scheduled_event_id={guild_scheduled_event_id}"
        ));
    }
    query_string(params)
}

fn poll_answer_voters_query(after: Option<Snowflake>, limit: Option<u64>) -> String {
    let mut params = Vec::new();
    if let Some(after) = after {
        params.push(format!("after={after}"));
    }
    if let Some(limit) = limit {
        params.push(format!("limit={limit}"));
    }
    query_string(params)
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
    tokio::time::sleep(duration).await;
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::sync::{Arc, Mutex};
    use std::time::{Duration, Instant};

    use super::{
        archived_threads_query, configured_application_id, discord_api_error,
        discord_rate_limit_error, execute_webhook_path, followup_webhook_path,
        global_commands_path, header_string, interaction_callback_path, invite_query,
        joined_archived_threads_query, parse_body_value, poll_answer_voters_query,
        rate_limit_route_key, request_uses_bot_authorization, sleep_for_retry_after,
        subscription_query, thread_member_query, validate_token_path_segment, FileAttachment,
        RateLimitState, RestClient,
    };
    use crate::command::{command_type, CommandDefinition};
    use crate::error::DiscordError;
    use crate::model::{
        ArchivedThreadsQuery, CreateMessage, CreateTestEntitlement, EntitlementQuery,
        InteractionCallbackResponse, JoinedArchivedThreadsQuery, Snowflake, SubscriptionQuery,
        ThreadMemberQuery,
    };
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

    fn sticker_payload(id: &str) -> serde_json::Value {
        json!({
            "id": id,
            "name": "sticker",
            "tags": "tag",
            "type": 2,
            "format_type": 1
        })
    }

    fn stage_payload(channel_id: &str) -> serde_json::Value {
        json!({
            "id": "9000",
            "guild_id": "200",
            "channel_id": channel_id,
            "topic": "town hall",
            "privacy_level": 2
        })
    }

    fn welcome_screen_payload() -> serde_json::Value {
        json!({
            "description": "welcome",
            "welcome_channels": [{
                "channel_id": "300",
                "description": "rules",
                "emoji_name": "wave"
            }]
        })
    }

    fn onboarding_payload() -> serde_json::Value {
        json!({
            "guild_id": "200",
            "prompts": [],
            "default_channel_ids": ["300"],
            "enabled": true,
            "mode": 1
        })
    }

    fn template_payload(code: &str) -> serde_json::Value {
        json!({
            "code": code,
            "name": "template",
            "usage_count": 0,
            "created_at": "2026-01-01T00:00:00.000000+00:00",
            "updated_at": "2026-01-01T00:00:00.000000+00:00"
        })
    }

    fn scheduled_event_payload(id: &str) -> serde_json::Value {
        json!({
            "id": id,
            "guild_id": "200",
            "channel_id": "300",
            "creator_id": "400",
            "name": "community night",
            "description": "games",
            "scheduled_start_time": "2026-05-01T00:00:00.000000+00:00",
            "scheduled_end_time": "2026-05-01T01:00:00.000000+00:00",
            "privacy_level": 2,
            "status": 1,
            "entity_type": 2,
            "entity_metadata": { "location": "Stage" },
            "user_count": 5
        })
    }

    fn sku_payload(id: &str) -> serde_json::Value {
        json!({
            "id": id,
            "type": 5,
            "application_id": "555",
            "name": "Premium",
            "slug": "premium",
            "flags": 128
        })
    }

    fn entitlement_payload(id: &str) -> serde_json::Value {
        json!({
            "id": id,
            "sku_id": "900",
            "application_id": "555",
            "user_id": "777",
            "type": 8,
            "deleted": false,
            "consumed": false,
            "starts_at": "2026-01-01T00:00:00.000000+00:00",
            "ends_at": "2026-02-01T00:00:00.000000+00:00",
            "guild_id": "200"
        })
    }

    fn subscription_payload(id: &str) -> serde_json::Value {
        json!({
            "id": id,
            "user_id": "777",
            "sku_ids": ["900"],
            "entitlement_ids": ["901"],
            "current_period_start": "2026-04-01T00:00:00.000000+00:00",
            "current_period_end": "2026-05-01T00:00:00.000000+00:00",
            "status": 0,
            "canceled_at": null
        })
    }

    fn soundboard_payload(id: &str) -> serde_json::Value {
        json!({
            "name": "quack",
            "sound_id": id,
            "volume": 1.0,
            "emoji_name": "duck",
            "guild_id": "200",
            "available": true
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
            Some(concat!(
                "DiscordBot (discordrs, ",
                env!("CARGO_PKG_VERSION"),
                ")"
            ))
        );
        assert_eq!(request.header("content-type"), Some("application/json"));
    }

    fn assert_multipart_request(
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
            Some(concat!(
                "DiscordBot (discordrs, ",
                env!("CARGO_PKG_VERSION"),
                ")"
            ))
        );
        assert!(
            request
                .header("content-type")
                .is_some_and(|value| value.starts_with("multipart/form-data; boundary=")),
            "expected multipart content-type, got {:?}",
            request.header("content-type")
        );
        assert!(request.body.contains(r#"name="payload_json""#));
        assert!(request.body.contains(r#"name="files[0]""#));
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

    fn sample_file(name: &str, data: &str) -> FileAttachment {
        FileAttachment::new(name, data.as_bytes().to_vec()).with_content_type("text/plain")
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
    fn new_coverage_query_helpers_build_expected_paths() {
        assert_eq!(
            thread_member_query(&ThreadMemberQuery {
                with_member: Some(true),
                after: Some(Snowflake::from("10")),
                limit: Some(25),
            }),
            "?with_member=true&after=10&limit=25"
        );
        assert_eq!(
            archived_threads_query(&ArchivedThreadsQuery {
                before: Some("2026-04-29T00:00:00Z".to_string()),
                limit: Some(50),
            }),
            "?before=2026-04-29T00:00:00Z&limit=50"
        );
        assert_eq!(
            joined_archived_threads_query(&JoinedArchivedThreadsQuery {
                before: Some(Snowflake::from("11")),
                limit: Some(10),
            }),
            "?before=11&limit=10"
        );
        assert_eq!(
            subscription_query(&SubscriptionQuery {
                before: Some(Snowflake::from("20")),
                after: Some(Snowflake::from("21")),
                limit: Some(100),
                user_id: Some(Snowflake::from("22")),
            }),
            "?before=20&after=21&limit=100&user_id=22"
        );
        assert_eq!(
            invite_query(Some(true), Some(false), Some(Snowflake::from("30"))),
            "?with_counts=true&with_expiration=false&guild_scheduled_event_id=30"
        );
        assert_eq!(
            poll_answer_voters_query(Some(Snowflake::from("40")), Some(15)),
            "?after=40&limit=15"
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
    async fn channel_message_file_helpers_send_multipart_payloads() {
        let responses = vec![
            PlannedResponse::json(StatusCode::OK, message_payload("701", "100", "created")),
            PlannedResponse::json(StatusCode::OK, message_payload("702", "100", "updated")),
        ];
        let (base_url, captured, server) = spawn_test_server(responses).await;
        let client = RestClient::new_with_base_url("file-token", 123, base_url);
        let body = sample_message();
        let files = vec![sample_file("hello.txt", "hello file")];

        assert_eq!(
            client
                .create_message_with_files(Snowflake::from("100"), &body, &files)
                .await
                .expect("create message with files")
                .content,
            "created"
        );
        assert_eq!(
            client
                .update_message_with_files(
                    Snowflake::from("100"),
                    Snowflake::from("701"),
                    &body,
                    &files,
                )
                .await
                .expect("update message with files")
                .content,
            "updated"
        );

        server.await.expect("server finished");
        let requests = captured.lock().expect("captured requests");
        assert_eq!(requests.len(), 2);
        assert_multipart_request(
            &requests[0],
            "POST",
            "/channels/100/messages",
            Some("Bot file-token"),
        );
        assert!(requests[0].body.contains(r#"{"content":"hello"}"#));
        assert!(requests[0].body.contains(r#"filename="hello.txt""#));
        assert!(requests[0].body.contains("Content-Type: text/plain"));
        assert!(requests[0].body.contains("hello file"));
        assert_multipart_request(
            &requests[1],
            "PATCH",
            "/channels/100/messages/701",
            Some("Bot file-token"),
        );
        assert!(requests[1].body.contains(r#"filename="hello.txt""#));
    }

    #[tokio::test]
    async fn tokenized_file_helpers_send_multipart_without_bot_authorization() {
        let responses = vec![
            PlannedResponse::json(StatusCode::OK, json!({ "id": "800" })),
            PlannedResponse::empty(StatusCode::NO_CONTENT),
            PlannedResponse::json(StatusCode::OK, message_payload("801", "500", "followup")),
            PlannedResponse::json(StatusCode::OK, message_payload("802", "500", "original")),
            PlannedResponse::json(
                StatusCode::OK,
                message_payload("803", "500", "followup-edit"),
            ),
        ];
        let (base_url, captured, server) = spawn_test_server(responses).await;
        let client = RestClient::new_with_base_url("bot-token", 123, base_url);
        let body = sample_message();
        let files = vec![sample_file("tokenized.txt", "tokenized file")];

        assert_eq!(
            client
                .execute_webhook_with_files(
                    Snowflake::from("777"),
                    "token",
                    &json!({ "content": "webhook" }),
                    &files,
                )
                .await
                .expect("execute webhook with files")["id"],
            json!("800")
        );
        client
            .create_interaction_response_with_files(
                Snowflake::from("778"),
                "token",
                &sample_interaction_response(),
                &files,
            )
            .await
            .expect("create interaction response with files");
        assert_eq!(
            client
                .create_followup_message_with_files("token", &body, &files)
                .await
                .expect("create followup with files")
                .content,
            "followup"
        );
        assert_eq!(
            client
                .edit_original_interaction_response_with_files("token", &body, &files)
                .await
                .expect("edit original with files")
                .content,
            "original"
        );
        assert_eq!(
            client
                .edit_followup_message_with_files("token", "55", &body, &files)
                .await
                .expect("edit followup with files")
                .content,
            "followup-edit"
        );

        server.await.expect("server finished");
        let requests = captured.lock().expect("captured requests");
        assert_eq!(requests.len(), 5);
        assert_multipart_request(&requests[0], "POST", "/webhooks/777/token?wait=true", None);
        assert_multipart_request(
            &requests[1],
            "POST",
            "/interactions/778/token/callback",
            None,
        );
        assert_multipart_request(&requests[2], "POST", "/webhooks/123/token", None);
        assert_multipart_request(
            &requests[3],
            "PATCH",
            "/webhooks/123/token/messages/@original",
            None,
        );
        assert_multipart_request(
            &requests[4],
            "PATCH",
            "/webhooks/123/token/messages/55",
            None,
        );
        assert!(requests[0].body.contains(r#"{"content":"webhook"}"#));
        assert!(requests[1].body.contains(r#""type":4"#));
        assert!(requests[4].body.contains("tokenized file"));
    }

    #[tokio::test]
    async fn webhook_message_crud_uses_tokenized_message_paths() {
        let responses = vec![
            PlannedResponse::json(StatusCode::OK, message_payload("900", "500", "webhook")),
            PlannedResponse::json(StatusCode::OK, message_payload("900", "500", "edited")),
            PlannedResponse::json(StatusCode::OK, message_payload("900", "500", "edited-file")),
            PlannedResponse::empty(StatusCode::NO_CONTENT),
        ];
        let (base_url, captured, server) = spawn_test_server(responses).await;
        let client = RestClient::new_with_base_url("bot-token", 123, base_url);
        let body = sample_message();
        let files = vec![sample_file("webhook.txt", "webhook file")];

        assert_eq!(
            client
                .get_webhook_message(Snowflake::from("777"), "token", "900")
                .await
                .expect("get webhook message")
                .content,
            "webhook"
        );
        assert_eq!(
            client
                .edit_webhook_message(Snowflake::from("777"), "token", "900", &body)
                .await
                .expect("edit webhook message")
                .content,
            "edited"
        );
        assert_eq!(
            client
                .edit_webhook_message_with_files(
                    Snowflake::from("777"),
                    "token",
                    "900",
                    &body,
                    &files,
                )
                .await
                .expect("edit webhook message with files")
                .content,
            "edited-file"
        );
        client
            .delete_webhook_message(Snowflake::from("777"), "token", "900")
            .await
            .expect("delete webhook message");

        server.await.expect("server finished");
        let requests = captured.lock().expect("captured requests");
        assert_eq!(requests.len(), 4);
        assert_request_basics(
            &requests[0],
            "GET",
            "/webhooks/777/token/messages/900",
            None,
        );
        assert_request_basics(
            &requests[1],
            "PATCH",
            "/webhooks/777/token/messages/900",
            None,
        );
        assert_multipart_request(
            &requests[2],
            "PATCH",
            "/webhooks/777/token/messages/900",
            None,
        );
        assert_request_basics(
            &requests[3],
            "DELETE",
            "/webhooks/777/token/messages/900",
            None,
        );
        assert!(requests[2].body.contains("webhook file"));
    }

    #[tokio::test]
    async fn sticker_stage_and_guild_admin_wrappers_hit_expected_paths() {
        let responses = vec![
            PlannedResponse::json(StatusCode::OK, json!({ "items": [{ "id": "1" }] })),
            PlannedResponse::json(StatusCode::OK, json!({ "id": "1" })),
            PlannedResponse::json(StatusCode::OK, json!({ "id": "2" })),
            PlannedResponse::json(StatusCode::OK, json!({ "id": "2", "name": "renamed" })),
            PlannedResponse::empty(StatusCode::NO_CONTENT),
            PlannedResponse::json(StatusCode::OK, sticker_payload("10")),
            PlannedResponse::json(
                StatusCode::OK,
                json!({
                    "sticker_packs": [{
                        "id": "20",
                        "name": "pack",
                        "stickers": [sticker_payload("10")]
                    }]
                }),
            ),
            PlannedResponse::json(StatusCode::OK, json!([sticker_payload("11")])),
            PlannedResponse::json(StatusCode::OK, sticker_payload("11")),
            PlannedResponse::json(StatusCode::OK, sticker_payload("12")),
            PlannedResponse::json(StatusCode::OK, sticker_payload("12")),
            PlannedResponse::empty(StatusCode::NO_CONTENT),
            PlannedResponse::json(StatusCode::OK, json!({ "pruned": 3 })),
            PlannedResponse::json(StatusCode::OK, json!({ "pruned": 2 })),
            PlannedResponse::json(
                StatusCode::OK,
                json!({ "enabled": true, "channel_id": "300" }),
            ),
            PlannedResponse::json(StatusCode::OK, json!({ "enabled": false })),
            PlannedResponse::json(StatusCode::OK, json!({ "id": "200", "name": "widget" })),
            PlannedResponse::json(
                StatusCode::OK,
                json!({ "channel_id": "100", "webhook_id": "101" }),
            ),
            PlannedResponse::json(StatusCode::OK, stage_payload("400")),
            PlannedResponse::json(StatusCode::OK, stage_payload("400")),
            PlannedResponse::json(StatusCode::OK, stage_payload("400")),
            PlannedResponse::empty(StatusCode::NO_CONTENT),
            PlannedResponse::json(StatusCode::OK, welcome_screen_payload()),
            PlannedResponse::json(StatusCode::OK, welcome_screen_payload()),
            PlannedResponse::json(StatusCode::OK, onboarding_payload()),
            PlannedResponse::json(StatusCode::OK, onboarding_payload()),
            PlannedResponse::json(StatusCode::OK, json!([template_payload("tmpl")])),
            PlannedResponse::json(StatusCode::OK, template_payload("tmpl")),
            PlannedResponse::json(StatusCode::OK, template_payload("tmpl")),
            PlannedResponse::json(StatusCode::OK, template_payload("tmpl")),
            PlannedResponse::json(StatusCode::OK, template_payload("tmpl")),
        ];
        let (base_url, captured, server) = spawn_test_server(responses).await;
        let client = RestClient::new_with_base_url("admin-token", 555, base_url);
        let body = json!({ "name": "name", "description": "desc", "tags": "tag" });

        assert_eq!(client.get_application_emojis().await.unwrap().len(), 1);
        assert_eq!(
            client
                .get_application_emoji(Snowflake::from("1"))
                .await
                .unwrap()["id"],
            json!("1")
        );
        assert_eq!(
            client.create_application_emoji(&body).await.unwrap()["id"],
            json!("2")
        );
        assert_eq!(
            client
                .modify_application_emoji(Snowflake::from("2"), &body)
                .await
                .unwrap()["name"],
            json!("renamed")
        );
        client
            .delete_application_emoji(Snowflake::from("2"))
            .await
            .unwrap();
        assert_eq!(
            client
                .get_sticker(Snowflake::from("10"))
                .await
                .unwrap()
                .name,
            "sticker"
        );
        assert_eq!(
            client
                .list_sticker_packs()
                .await
                .unwrap()
                .sticker_packs
                .len(),
            1
        );
        assert_eq!(
            client
                .get_guild_stickers(Snowflake::from("200"))
                .await
                .unwrap()
                .len(),
            1
        );
        assert_eq!(
            client
                .get_guild_sticker(Snowflake::from("200"), Snowflake::from("11"))
                .await
                .unwrap()
                .id
                .as_str(),
            "11"
        );
        assert_eq!(
            client
                .create_guild_sticker(
                    Snowflake::from("200"),
                    &body,
                    sample_file("sticker.png", "png")
                )
                .await
                .unwrap()
                .id
                .as_str(),
            "12"
        );
        assert_eq!(
            client
                .modify_guild_sticker(Snowflake::from("200"), Snowflake::from("12"), &body)
                .await
                .unwrap()
                .id
                .as_str(),
            "12"
        );
        client
            .delete_guild_sticker(Snowflake::from("200"), Snowflake::from("12"))
            .await
            .unwrap();
        assert_eq!(
            client
                .get_guild_prune_count(Snowflake::from("200"), Some(7), &[Snowflake::from("9")])
                .await
                .unwrap()["pruned"],
            json!(3)
        );
        assert_eq!(
            client
                .begin_guild_prune(Snowflake::from("200"), Some(7), Some(false), &[])
                .await
                .unwrap()["pruned"],
            json!(2)
        );
        assert!(
            client
                .get_guild_widget_settings(Snowflake::from("200"))
                .await
                .unwrap()
                .enabled
        );
        assert!(
            !client
                .modify_guild_widget_settings(Snowflake::from("200"), &json!({ "enabled": false }))
                .await
                .unwrap()
                .enabled
        );
        assert_eq!(
            client
                .get_guild_widget(Snowflake::from("200"))
                .await
                .unwrap()["name"],
            json!("widget")
        );
        assert_eq!(
            client
                .follow_announcement_channel(Snowflake::from("100"), Snowflake::from("101"))
                .await
                .unwrap()
                .webhook_id
                .as_str(),
            "101"
        );
        assert_eq!(
            client
                .create_stage_instance(&body)
                .await
                .unwrap()
                .channel_id
                .as_str(),
            "400"
        );
        assert_eq!(
            client
                .get_stage_instance(Snowflake::from("400"))
                .await
                .unwrap()
                .topic,
            "town hall"
        );
        assert_eq!(
            client
                .modify_stage_instance(Snowflake::from("400"), &body)
                .await
                .unwrap()
                .privacy_level,
            2
        );
        client
            .delete_stage_instance(Snowflake::from("400"))
            .await
            .unwrap();
        assert_eq!(
            client
                .get_guild_welcome_screen(Snowflake::from("200"))
                .await
                .unwrap()
                .welcome_channels
                .len(),
            1
        );
        assert_eq!(
            client
                .modify_guild_welcome_screen(Snowflake::from("200"), &welcome_screen_payload())
                .await
                .unwrap()
                .description
                .as_deref(),
            Some("welcome")
        );
        assert!(
            client
                .get_guild_onboarding(Snowflake::from("200"))
                .await
                .unwrap()
                .enabled
        );
        assert!(
            client
                .modify_guild_onboarding(Snowflake::from("200"), &onboarding_payload())
                .await
                .unwrap()
                .enabled
        );
        assert_eq!(
            client
                .get_guild_templates(Snowflake::from("200"))
                .await
                .unwrap()
                .len(),
            1
        );
        assert_eq!(
            client
                .create_guild_template(Snowflake::from("200"), &body)
                .await
                .unwrap()
                .code,
            "tmpl"
        );
        assert_eq!(
            client
                .sync_guild_template(Snowflake::from("200"), "tmpl")
                .await
                .unwrap()
                .code,
            "tmpl"
        );
        assert_eq!(
            client
                .modify_guild_template(Snowflake::from("200"), "tmpl", &body)
                .await
                .unwrap()
                .code,
            "tmpl"
        );
        assert_eq!(
            client
                .delete_guild_template(Snowflake::from("200"), "tmpl")
                .await
                .unwrap()
                .code,
            "tmpl"
        );

        server.await.expect("server finished");
        let requests = captured.lock().expect("captured requests");
        assert_eq!(requests.len(), 31);
        assert_request_basics(
            &requests[0],
            "GET",
            "/applications/555/emojis",
            Some("Bot admin-token"),
        );
        assert_request_basics(
            &requests[4],
            "DELETE",
            "/applications/555/emojis/2",
            Some("Bot admin-token"),
        );
        assert_request_basics(&requests[5], "GET", "/stickers/10", Some("Bot admin-token"));
        assert_request_basics(
            &requests[7],
            "GET",
            "/guilds/200/stickers",
            Some("Bot admin-token"),
        );
        assert_eq!(requests[9].method, "POST");
        assert_eq!(requests[9].path, "/guilds/200/stickers");
        assert_eq!(requests[9].header("authorization"), Some("Bot admin-token"));
        assert!(requests[9]
            .header("content-type")
            .is_some_and(|value| value.starts_with("multipart/form-data; boundary=")));
        assert!(requests[9].body.contains(r#"name="file""#));
        assert_request_basics(
            &requests[12],
            "GET",
            "/guilds/200/prune?days=7&include_roles=9",
            Some("Bot admin-token"),
        );
        assert_request_basics(
            &requests[13],
            "POST",
            "/guilds/200/prune?days=7&compute_prune_count=false",
            Some("Bot admin-token"),
        );
        assert_request_basics(
            &requests[17],
            "POST",
            "/channels/100/followers",
            Some("Bot admin-token"),
        );
        assert_request_basics(
            &requests[18],
            "POST",
            "/stage-instances",
            Some("Bot admin-token"),
        );
        assert_request_basics(
            &requests[22],
            "GET",
            "/guilds/200/welcome-screen",
            Some("Bot admin-token"),
        );
        assert_request_basics(
            &requests[25],
            "PUT",
            "/guilds/200/onboarding",
            Some("Bot admin-token"),
        );
        assert_request_basics(
            &requests[28],
            "PUT",
            "/guilds/200/templates/tmpl",
            Some("Bot admin-token"),
        );
    }

    #[tokio::test]
    async fn scheduled_event_wrappers_return_typed_models() {
        let responses = vec![
            PlannedResponse::json(StatusCode::OK, json!([scheduled_event_payload("1")])),
            PlannedResponse::json(StatusCode::OK, scheduled_event_payload("2")),
            PlannedResponse::json(StatusCode::OK, scheduled_event_payload("2")),
            PlannedResponse::json(StatusCode::OK, scheduled_event_payload("2")),
            PlannedResponse::empty(StatusCode::NO_CONTENT),
            PlannedResponse::json(
                StatusCode::OK,
                json!([{
                    "guild_scheduled_event_id": "2",
                    "user": {
                        "id": "500",
                        "username": "attendee",
                        "discriminator": "0000",
                        "bot": false
                    }
                }]),
            ),
        ];
        let (base_url, captured, server) = spawn_test_server(responses).await;
        let client = RestClient::new_with_base_url("scheduled-token", 0, base_url);
        let body = json!({ "name": "community night" });

        assert_eq!(
            client
                .get_guild_scheduled_events(Snowflake::from("200"))
                .await
                .unwrap()[0]
                .name,
            "community night"
        );
        assert_eq!(
            client
                .create_guild_scheduled_event(Snowflake::from("200"), &body)
                .await
                .unwrap()
                .entity_metadata
                .unwrap()["location"],
            json!("Stage")
        );
        assert_eq!(
            client
                .get_guild_scheduled_event(Snowflake::from("200"), Snowflake::from("2"))
                .await
                .unwrap()
                .user_count,
            Some(5)
        );
        assert_eq!(
            client
                .modify_guild_scheduled_event(Snowflake::from("200"), Snowflake::from("2"), &body)
                .await
                .unwrap()
                .status,
            1
        );
        client
            .delete_guild_scheduled_event(Snowflake::from("200"), Snowflake::from("2"))
            .await
            .unwrap();
        assert_eq!(
            client
                .get_guild_scheduled_event_users(
                    Snowflake::from("200"),
                    Snowflake::from("2"),
                    Some(50)
                )
                .await
                .unwrap()[0]
                .user
                .username,
            "attendee"
        );

        server.await.expect("server finished");
        let requests = captured.lock().expect("captured requests");
        assert_eq!(requests.len(), 6);
        assert_request_basics(
            &requests[0],
            "GET",
            "/guilds/200/scheduled-events",
            Some("Bot scheduled-token"),
        );
        assert_request_basics(
            &requests[4],
            "DELETE",
            "/guilds/200/scheduled-events/2",
            Some("Bot scheduled-token"),
        );
        assert_request_basics(
            &requests[5],
            "GET",
            "/guilds/200/scheduled-events/2/users?limit=50",
            Some("Bot scheduled-token"),
        );
    }

    #[tokio::test]
    async fn monetization_and_soundboard_wrappers_hit_expected_paths() {
        let responses = vec![
            PlannedResponse::json(StatusCode::OK, json!([sku_payload("900")])),
            PlannedResponse::json(StatusCode::OK, json!([subscription_payload("950")])),
            PlannedResponse::json(StatusCode::OK, subscription_payload("950")),
            PlannedResponse::json(StatusCode::OK, json!([entitlement_payload("901")])),
            PlannedResponse::json(StatusCode::OK, entitlement_payload("901")),
            PlannedResponse::empty(StatusCode::NO_CONTENT),
            PlannedResponse::json(StatusCode::OK, entitlement_payload("902")),
            PlannedResponse::empty(StatusCode::NO_CONTENT),
            PlannedResponse::empty(StatusCode::NO_CONTENT),
            PlannedResponse::json(StatusCode::OK, json!([soundboard_payload("1")])),
            PlannedResponse::json(
                StatusCode::OK,
                json!({ "items": [soundboard_payload("2")] }),
            ),
            PlannedResponse::json(StatusCode::OK, soundboard_payload("2")),
            PlannedResponse::json(StatusCode::OK, soundboard_payload("3")),
            PlannedResponse::json(StatusCode::OK, soundboard_payload("3")),
            PlannedResponse::empty(StatusCode::NO_CONTENT),
        ];
        let (base_url, captured, server) = spawn_test_server(responses).await;
        let client = RestClient::new_with_base_url("premium-token", 555, base_url);
        let query = EntitlementQuery {
            user_id: Some(Snowflake::from("777")),
            sku_ids: vec![Snowflake::from("900"), Snowflake::from("901")],
            limit: Some(25),
            guild_id: Some(Snowflake::from("200")),
            exclude_ended: Some(true),
            exclude_deleted: Some(false),
            ..EntitlementQuery::default()
        };
        let subscription_query = SubscriptionQuery {
            user_id: Some(Snowflake::from("777")),
            limit: Some(10),
            ..SubscriptionQuery::default()
        };
        let sound_body = json!({ "sound_id": "1", "source_guild_id": "200" });

        assert_eq!(client.get_skus().await.unwrap()[0].slug, "premium");
        assert_eq!(
            client
                .get_sku_subscriptions(Snowflake::from("900"), &subscription_query)
                .await
                .unwrap()[0]
                .id
                .as_str(),
            "950"
        );
        assert_eq!(
            client
                .get_sku_subscription(Snowflake::from("900"), Snowflake::from("950"))
                .await
                .unwrap()
                .user_id
                .as_str(),
            "777"
        );
        assert_eq!(
            client.get_entitlements(&query).await.unwrap()[0]
                .sku_id
                .as_str(),
            "900"
        );
        assert_eq!(
            client
                .get_entitlement(Snowflake::from("901"))
                .await
                .unwrap()
                .user_id
                .unwrap()
                .as_str(),
            "777"
        );
        client
            .consume_entitlement(Snowflake::from("901"))
            .await
            .unwrap();
        assert_eq!(
            client
                .create_test_entitlement(&CreateTestEntitlement {
                    sku_id: Snowflake::from("900"),
                    owner_id: Snowflake::from("200"),
                    owner_type: 1,
                })
                .await
                .unwrap()
                .id
                .as_str(),
            "902"
        );
        client
            .delete_test_entitlement(Snowflake::from("902"))
            .await
            .unwrap();
        client
            .send_soundboard_sound(Snowflake::from("300"), &sound_body)
            .await
            .unwrap();
        assert_eq!(
            client.list_default_soundboard_sounds().await.unwrap().len(),
            1
        );
        assert_eq!(
            client
                .list_guild_soundboard_sounds(Snowflake::from("200"))
                .await
                .unwrap()
                .items
                .len(),
            1
        );
        assert_eq!(
            client
                .get_guild_soundboard_sound(Snowflake::from("200"), Snowflake::from("2"))
                .await
                .unwrap()
                .name,
            "quack"
        );
        assert_eq!(
            client
                .create_guild_soundboard_sound(Snowflake::from("200"), &sound_body)
                .await
                .unwrap()
                .sound_id
                .as_str(),
            "3"
        );
        assert_eq!(
            client
                .modify_guild_soundboard_sound(
                    Snowflake::from("200"),
                    Snowflake::from("3"),
                    &json!({ "name": "quack" })
                )
                .await
                .unwrap()
                .sound_id
                .as_str(),
            "3"
        );
        client
            .delete_guild_soundboard_sound(Snowflake::from("200"), Snowflake::from("3"))
            .await
            .unwrap();

        server.await.expect("server finished");
        let requests = captured.lock().expect("captured requests");
        assert_eq!(requests.len(), 15);
        assert_request_basics(
            &requests[0],
            "GET",
            "/applications/555/skus",
            Some("Bot premium-token"),
        );
        assert_request_basics(
            &requests[1],
            "GET",
            "/skus/900/subscriptions?limit=10&user_id=777",
            Some("Bot premium-token"),
        );
        assert_request_basics(
            &requests[2],
            "GET",
            "/skus/900/subscriptions/950",
            Some("Bot premium-token"),
        );
        assert_request_basics(
            &requests[3],
            "GET",
            "/applications/555/entitlements?user_id=777&sku_ids=900,901&limit=25&guild_id=200&exclude_ended=true&exclude_deleted=false",
            Some("Bot premium-token"),
        );
        assert_request_basics(
            &requests[5],
            "POST",
            "/applications/555/entitlements/901/consume",
            Some("Bot premium-token"),
        );
        assert_request_basics(
            &requests[8],
            "POST",
            "/channels/300/send-soundboard-sound",
            Some("Bot premium-token"),
        );
        assert_request_basics(
            &requests[9],
            "GET",
            "/soundboard-default-sounds",
            Some("Bot premium-token"),
        );
        assert_request_basics(
            &requests[14],
            "DELETE",
            "/guilds/200/soundboard-sounds/3",
            Some("Bot premium-token"),
        );
    }

    #[tokio::test]
    async fn poll_thread_invite_and_integration_wrappers_hit_expected_paths() {
        let thread_list = json!({
            "threads": [{ "id": "700", "type": 11, "name": "thread" }],
            "members": [{ "id": "700", "user_id": "777", "flags": 0 }],
            "has_more": false
        });
        let responses = vec![
            PlannedResponse::empty(StatusCode::NO_CONTENT),
            PlannedResponse::empty(StatusCode::NO_CONTENT),
            PlannedResponse::json(
                StatusCode::OK,
                json!({ "id": "700", "user_id": "777", "flags": 0 }),
            ),
            PlannedResponse::json(
                StatusCode::OK,
                json!([{ "id": "700", "user_id": "777", "flags": 0 }]),
            ),
            PlannedResponse::json(StatusCode::OK, thread_list.clone()),
            PlannedResponse::json(StatusCode::OK, thread_list.clone()),
            PlannedResponse::json(StatusCode::OK, thread_list.clone()),
            PlannedResponse::json(StatusCode::OK, thread_list),
            PlannedResponse::json(StatusCode::OK, json!({ "code": "abc", "uses": 2 })),
            PlannedResponse::json(
                StatusCode::OK,
                json!([{
                    "id": "900",
                    "name": "integration",
                    "type": "discord",
                    "enabled": true,
                    "account": { "id": "acc", "name": "account" }
                }]),
            ),
            PlannedResponse::empty(StatusCode::NO_CONTENT),
            PlannedResponse::json(
                StatusCode::OK,
                json!({ "users": [{ "id": "777", "username": "voter" }] }),
            ),
            PlannedResponse::json(
                StatusCode::OK,
                json!({ "id": "800", "channel_id": "100", "content": "poll ended" }),
            ),
        ];
        let (base_url, captured, server) = spawn_test_server(responses).await;
        let client = RestClient::new_with_base_url("coverage-token", 555, base_url);

        client
            .add_thread_member(Snowflake::from("700"), Snowflake::from("777"))
            .await
            .unwrap();
        client
            .remove_thread_member(Snowflake::from("700"), Snowflake::from("777"))
            .await
            .unwrap();
        assert_eq!(
            client
                .get_thread_member(Snowflake::from("700"), Snowflake::from("777"), Some(true))
                .await
                .unwrap()
                .user_id
                .unwrap()
                .as_str(),
            "777"
        );
        assert_eq!(
            client
                .list_thread_members(
                    Snowflake::from("700"),
                    &ThreadMemberQuery {
                        with_member: Some(true),
                        after: Some(Snowflake::from("10")),
                        limit: Some(25),
                    },
                )
                .await
                .unwrap()
                .len(),
            1
        );
        assert_eq!(
            client
                .list_public_archived_threads(
                    Snowflake::from("100"),
                    &ArchivedThreadsQuery {
                        before: Some("2026-04-29T00:00:00Z".to_string()),
                        limit: Some(50),
                    },
                )
                .await
                .unwrap()
                .threads
                .len(),
            1
        );
        client
            .list_private_archived_threads(Snowflake::from("100"), &ArchivedThreadsQuery::default())
            .await
            .unwrap();
        client
            .list_joined_private_archived_threads(
                Snowflake::from("100"),
                &JoinedArchivedThreadsQuery {
                    before: Some(Snowflake::from("700")),
                    limit: Some(10),
                },
            )
            .await
            .unwrap();
        client
            .get_active_guild_threads(Snowflake::from("200"))
            .await
            .unwrap();
        assert_eq!(
            client
                .get_invite_with_options(
                    "abc",
                    Some(true),
                    Some(true),
                    Some(Snowflake::from("300"))
                )
                .await
                .unwrap()
                .uses,
            Some(2)
        );
        assert_eq!(
            client
                .get_guild_integrations(Snowflake::from("200"))
                .await
                .unwrap()[0]
                .id
                .as_str(),
            "900"
        );
        client
            .delete_guild_integration(Snowflake::from("200"), Snowflake::from("900"))
            .await
            .unwrap();
        assert_eq!(
            client
                .get_poll_answer_voters(
                    Snowflake::from("100"),
                    Snowflake::from("800"),
                    1,
                    Some(Snowflake::from("777")),
                    Some(10),
                )
                .await
                .unwrap()
                .users[0]
                .username,
            "voter"
        );
        assert_eq!(
            client
                .end_poll(Snowflake::from("100"), Snowflake::from("800"))
                .await
                .unwrap()
                .content,
            "poll ended"
        );

        server.await.expect("server finished");
        let requests = captured.lock().expect("captured requests");
        assert_eq!(requests.len(), 13);
        assert_request_basics(
            &requests[0],
            "PUT",
            "/channels/700/thread-members/777",
            Some("Bot coverage-token"),
        );
        assert_request_basics(
            &requests[3],
            "GET",
            "/channels/700/thread-members?with_member=true&after=10&limit=25",
            Some("Bot coverage-token"),
        );
        assert_request_basics(
            &requests[4],
            "GET",
            "/channels/100/threads/archived/public?before=2026-04-29T00:00:00Z&limit=50",
            Some("Bot coverage-token"),
        );
        assert_request_basics(
            &requests[6],
            "GET",
            "/channels/100/users/@me/threads/archived/private?before=700&limit=10",
            Some("Bot coverage-token"),
        );
        assert_request_basics(
            &requests[7],
            "GET",
            "/guilds/200/threads/active",
            Some("Bot coverage-token"),
        );
        assert_request_basics(
            &requests[8],
            "GET",
            "/invites/abc?with_counts=true&with_expiration=true&guild_scheduled_event_id=300",
            Some("Bot coverage-token"),
        );
        assert_request_basics(
            &requests[10],
            "DELETE",
            "/guilds/200/integrations/900",
            Some("Bot coverage-token"),
        );
        assert_request_basics(
            &requests[11],
            "GET",
            "/channels/100/polls/800/answers/1?after=777&limit=10",
            Some("Bot coverage-token"),
        );
        assert_request_basics(
            &requests[12],
            "POST",
            "/channels/100/polls/800/expire",
            Some("Bot coverage-token"),
        );
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
