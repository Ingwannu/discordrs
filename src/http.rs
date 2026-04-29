use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

use reqwest::{header::HeaderMap, Client, Method, StatusCode};
use serde::de::DeserializeOwned;
use serde_json::Value;
use tracing::{debug, warn};

mod body;
mod messages;
mod paths;
mod rate_limit;
#[cfg(test)]
mod tests;

use crate::command::CommandDefinition;
use crate::error::DiscordError;
use crate::model::{
    Application, ApplicationCommand, ApplicationRoleConnectionMetadata, ArchivedThreadsQuery,
    AutoModerationRule, Ban, BulkGuildBanRequest, BulkGuildBanResponse, Channel, CreateDmChannel,
    CreateMessage, CreateTestEntitlement, CurrentUserGuild, Entitlement, EntitlementQuery,
    FollowedChannel, GatewayBot, Guild, GuildOnboarding, GuildPreview, GuildPruneCount,
    GuildPruneResult, GuildScheduledEvent, GuildScheduledEventUser, GuildTemplate,
    GuildWidgetSettings, Integration, InteractionCallbackResponse, Invite,
    JoinedArchivedThreadsQuery, Member, Message, PollAnswerVoters, Role, Sku, Snowflake,
    SoundboardSound, SoundboardSoundList, StageInstance, Sticker, StickerPackList, Subscription,
    SubscriptionQuery, ThreadListResponse, ThreadMember, ThreadMemberQuery, User, VanityUrl,
    VoiceRegion, Webhook, WelcomeScreen,
};
use crate::types::Emoji;
use body::{
    build_multipart_form, build_sticker_form, clone_json_body, multipart_body, parse_body_value,
    serialize_body, RequestBody,
};
use paths::{
    archived_threads_query, bool_query, configured_application_id, entitlement_query,
    execute_webhook_path, followup_webhook_path, global_commands_path, guild_prune_query,
    interaction_callback_path, invite_query, joined_archived_threads_query,
    poll_answer_voters_query, rate_limit_route_key, request_uses_bot_authorization,
    subscription_query, thread_member_query, validate_token_path_segment, webhook_message_path,
};
use rate_limit::RateLimitState;
#[cfg(test)]
use rate_limit::RATE_LIMIT_BUCKET_RETENTION;

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

#[derive(Debug, serde::Deserialize)]
struct EmojiListResponse {
    #[serde(default)]
    items: Vec<Emoji>,
}

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
        self.update_channel_typed(channel_id, body).await
    }

    pub async fn update_channel_typed<B>(
        &self,
        channel_id: impl Into<Snowflake>,
        body: &B,
    ) -> Result<Channel, DiscordError>
    where
        B: serde::Serialize + ?Sized,
    {
        self.request_typed(
            Method::PATCH,
            &format!("/channels/{}", channel_id.into()),
            Some(body),
        )
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
        self.update_guild_typed(guild_id, body).await
    }

    pub async fn update_guild_typed<B>(
        &self,
        guild_id: impl Into<Snowflake>,
        body: &B,
    ) -> Result<Guild, DiscordError>
    where
        B: serde::Serialize + ?Sized,
    {
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
        self.create_guild_channel_typed(guild_id, body).await
    }

    pub async fn create_guild_channel_typed<B>(
        &self,
        guild_id: impl Into<Snowflake>,
        body: &B,
    ) -> Result<Channel, DiscordError>
    where
        B: serde::Serialize + ?Sized,
    {
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
        self.create_webhook_raw(channel_id, body).await
    }

    pub async fn create_webhook_typed<B>(
        &self,
        channel_id: impl Into<Snowflake>,
        body: &B,
    ) -> Result<Webhook, DiscordError>
    where
        B: serde::Serialize + ?Sized,
    {
        self.request_typed(
            Method::POST,
            &format!("/channels/{}/webhooks", channel_id.into()),
            Some(body),
        )
        .await
    }

    pub async fn create_webhook_raw(
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
        self.get_channel_webhooks_raw(channel_id).await
    }

    pub async fn get_channel_webhooks_typed(
        &self,
        channel_id: impl Into<Snowflake>,
    ) -> Result<Vec<Webhook>, DiscordError> {
        self.request_typed(
            Method::GET,
            &format!("/channels/{}/webhooks", channel_id.into()),
            Option::<&Value>::None,
        )
        .await
    }

    pub async fn get_channel_webhooks_raw(
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

    pub async fn get_current_application(&self) -> Result<Application, DiscordError> {
        self.request_typed(Method::GET, "/applications/@me", Option::<&Value>::None)
            .await
    }

    pub async fn edit_current_application<B>(&self, body: &B) -> Result<Application, DiscordError>
    where
        B: serde::Serialize + ?Sized,
    {
        self.request_typed(Method::PATCH, "/applications/@me", Some(body))
            .await
    }

    pub async fn get_application_role_connection_metadata_records(
        &self,
    ) -> Result<Vec<ApplicationRoleConnectionMetadata>, DiscordError> {
        let application_id = configured_application_id(self.application_id())?;
        self.request_typed(
            Method::GET,
            &format!("/applications/{application_id}/role-connections/metadata"),
            Option::<&Value>::None,
        )
        .await
    }

    pub async fn update_application_role_connection_metadata_records(
        &self,
        records: &[ApplicationRoleConnectionMetadata],
    ) -> Result<Vec<ApplicationRoleConnectionMetadata>, DiscordError> {
        let application_id = configured_application_id(self.application_id())?;
        self.request_typed(
            Method::PUT,
            &format!("/applications/{application_id}/role-connections/metadata"),
            Some(records),
        )
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

    pub async fn bulk_guild_ban(
        &self,
        guild_id: impl Into<Snowflake>,
        body: &BulkGuildBanRequest,
    ) -> Result<BulkGuildBanResponse, DiscordError> {
        self.request_typed(
            Method::POST,
            &format!("/guilds/{}/bulk-ban", guild_id.into()),
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
        self.modify_guild_member_typed(guild_id, user_id, body)
            .await
    }

    pub async fn modify_guild_member_typed<B>(
        &self,
        guild_id: impl Into<Snowflake>,
        user_id: impl Into<Snowflake>,
        body: &B,
    ) -> Result<(), DiscordError>
    where
        B: serde::Serialize + ?Sized,
    {
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

    pub async fn modify_current_member<B>(
        &self,
        guild_id: impl Into<Snowflake>,
        body: &B,
    ) -> Result<Member, DiscordError>
    where
        B: serde::Serialize + ?Sized,
    {
        self.request_typed(
            Method::PATCH,
            &format!("/guilds/{}/members/@me", guild_id.into()),
            Some(body),
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

    pub async fn get_guild_role(
        &self,
        guild_id: impl Into<Snowflake>,
        role_id: impl Into<Snowflake>,
    ) -> Result<Role, DiscordError> {
        self.request_typed(
            Method::GET,
            &format!("/guilds/{}/roles/{}", guild_id.into(), role_id.into()),
            Option::<&Value>::None,
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

    pub async fn get_guild_emojis_typed(
        &self,
        guild_id: impl Into<Snowflake>,
    ) -> Result<Vec<Emoji>, DiscordError> {
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

    pub async fn get_guild_emoji_typed(
        &self,
        guild_id: impl Into<Snowflake>,
        emoji_id: impl Into<Snowflake>,
    ) -> Result<Emoji, DiscordError> {
        self.request_typed(
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

    pub async fn create_guild_emoji_typed<B>(
        &self,
        guild_id: impl Into<Snowflake>,
        body: &B,
    ) -> Result<Emoji, DiscordError>
    where
        B: serde::Serialize + ?Sized,
    {
        self.request_typed(
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

    pub async fn modify_guild_emoji_typed<B>(
        &self,
        guild_id: impl Into<Snowflake>,
        emoji_id: impl Into<Snowflake>,
        body: &B,
    ) -> Result<Emoji, DiscordError>
    where
        B: serde::Serialize + ?Sized,
    {
        self.request_typed(
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

    pub async fn get_application_emojis_typed(&self) -> Result<Vec<Emoji>, DiscordError> {
        let application_id = configured_application_id(self.application_id())?;
        let response: EmojiListResponse = self
            .request_typed(
                Method::GET,
                &format!("/applications/{application_id}/emojis"),
                Option::<&Value>::None,
            )
            .await?;
        Ok(response.items)
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

    pub async fn get_application_emoji_typed(
        &self,
        emoji_id: impl Into<Snowflake>,
    ) -> Result<Emoji, DiscordError> {
        let application_id = configured_application_id(self.application_id())?;
        self.request_typed(
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

    pub async fn create_application_emoji_typed<B>(&self, body: &B) -> Result<Emoji, DiscordError>
    where
        B: serde::Serialize + ?Sized,
    {
        let application_id = configured_application_id(self.application_id())?;
        self.request_typed(
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

    pub async fn modify_application_emoji_typed<B>(
        &self,
        emoji_id: impl Into<Snowflake>,
        body: &B,
    ) -> Result<Emoji, DiscordError>
    where
        B: serde::Serialize + ?Sized,
    {
        let application_id = configured_application_id(self.application_id())?;
        self.request_typed(
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

    pub async fn get_current_user_guilds_typed(
        &self,
    ) -> Result<Vec<CurrentUserGuild>, DiscordError> {
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

    pub async fn get_webhook_with_token(
        &self,
        webhook_id: impl Into<Snowflake>,
        token: &str,
    ) -> Result<Webhook, DiscordError> {
        validate_token_path_segment("webhook_token", token, false)?;
        self.request_typed(
            Method::GET,
            &format!("/webhooks/{}/{}", webhook_id.into(), token),
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

    pub async fn create_guild_scheduled_event_typed<B>(
        &self,
        guild_id: impl Into<Snowflake>,
        body: &B,
    ) -> Result<GuildScheduledEvent, DiscordError>
    where
        B: serde::Serialize + ?Sized,
    {
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

    pub async fn modify_guild_scheduled_event_typed<B>(
        &self,
        guild_id: impl Into<Snowflake>,
        event_id: impl Into<Snowflake>,
        body: &B,
    ) -> Result<GuildScheduledEvent, DiscordError>
    where
        B: serde::Serialize + ?Sized,
    {
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

    pub async fn get_auto_moderation_rules_typed(
        &self,
        guild_id: impl Into<Snowflake>,
    ) -> Result<Vec<AutoModerationRule>, DiscordError> {
        self.request_typed(
            Method::GET,
            &format!("/guilds/{}/auto-moderation/rules", guild_id.into()),
            Option::<&Value>::None,
        )
        .await
    }

    pub async fn get_auto_moderation_rule(
        &self,
        guild_id: impl Into<Snowflake>,
        rule_id: impl Into<Snowflake>,
    ) -> Result<AutoModerationRule, DiscordError> {
        self.request_typed(
            Method::GET,
            &format!(
                "/guilds/{}/auto-moderation/rules/{}",
                guild_id.into(),
                rule_id.into()
            ),
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

    pub async fn create_auto_moderation_rule_typed<B>(
        &self,
        guild_id: impl Into<Snowflake>,
        body: &B,
    ) -> Result<AutoModerationRule, DiscordError>
    where
        B: serde::Serialize + ?Sized,
    {
        self.request_typed(
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

    pub async fn modify_auto_moderation_rule_typed<B>(
        &self,
        guild_id: impl Into<Snowflake>,
        rule_id: impl Into<Snowflake>,
        body: &B,
    ) -> Result<AutoModerationRule, DiscordError>
    where
        B: serde::Serialize + ?Sized,
    {
        self.request_typed(
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

    pub async fn get_guild_preview_typed(
        &self,
        guild_id: impl Into<Snowflake>,
    ) -> Result<GuildPreview, DiscordError> {
        self.request_typed(
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

    pub async fn get_guild_prune_count_typed(
        &self,
        guild_id: impl Into<Snowflake>,
        days: Option<u64>,
        include_roles: &[Snowflake],
    ) -> Result<GuildPruneCount, DiscordError> {
        let query = guild_prune_query(days, None, include_roles);
        self.request_typed(
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

    pub async fn begin_guild_prune_typed(
        &self,
        guild_id: impl Into<Snowflake>,
        days: Option<u64>,
        compute_prune_count: Option<bool>,
        include_roles: &[Snowflake],
    ) -> Result<GuildPruneResult, DiscordError> {
        let query = guild_prune_query(days, compute_prune_count, include_roles);
        self.request_typed(
            Method::POST,
            &format!("/guilds/{}/prune{query}", guild_id.into()),
            Option::<&Value>::None,
        )
        .await
    }

    pub async fn get_guild_vanity_url(
        &self,
        guild_id: impl Into<Snowflake>,
    ) -> Result<VanityUrl, DiscordError> {
        self.request_typed(
            Method::GET,
            &format!("/guilds/{}/vanity-url", guild_id.into()),
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
        self.create_stage_instance_typed(body).await
    }

    pub async fn create_stage_instance_typed<B>(
        &self,
        body: &B,
    ) -> Result<StageInstance, DiscordError>
    where
        B: serde::Serialize + ?Sized,
    {
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
        self.modify_stage_instance_typed(channel_id, body).await
    }

    pub async fn modify_stage_instance_typed<B>(
        &self,
        channel_id: impl Into<Snowflake>,
        body: &B,
    ) -> Result<StageInstance, DiscordError>
    where
        B: serde::Serialize + ?Sized,
    {
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

    pub async fn get_voice_regions_typed(&self) -> Result<Vec<VoiceRegion>, DiscordError> {
        self.request_typed(Method::GET, "/voice/regions", Option::<&Value>::None)
            .await
    }

    pub async fn get_guild_voice_regions(
        &self,
        guild_id: impl Into<Snowflake>,
    ) -> Result<Vec<VoiceRegion>, DiscordError> {
        self.request_typed(
            Method::GET,
            &format!("/guilds/{}/regions", guild_id.into()),
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

        let mut request_builder = self.client.request(method, url).header(
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

async fn sleep_for_retry_after(retry_after_seconds: f64) {
    let duration = Duration::from_secs_f64(retry_after_seconds.max(0.0));
    tokio::time::sleep(duration).await;
}
