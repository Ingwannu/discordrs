use reqwest::Method;
use serde_json::Value;

use crate::error::DiscordError;
use crate::model::{CreateMessage, Message, Snowflake, User};

use super::{FileAttachment, RestClient};

impl RestClient {
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

    pub async fn get_channel_messages(
        &self,
        channel_id: impl Into<Snowflake>,
        limit: Option<u64>,
    ) -> Result<Vec<Message>, DiscordError> {
        let path = match limit {
            Some(limit) => format!("/channels/{}/messages?limit={}", channel_id.into(), limit),
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
        if let Some(limit) = limit {
            params.push(format!("limit={limit}"));
        }
        if let Some(before) = before {
            params.push(format!("before={before}"));
        }
        if let Some(after) = after {
            params.push(format!("after={after}"));
        }
        if let Some(around) = around {
            params.push(format!("around={around}"));
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
        if let Some(limit) = limit {
            params.push(format!("limit={limit}"));
        }
        if let Some(after) = after {
            params.push(format!("after={after}"));
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
}
