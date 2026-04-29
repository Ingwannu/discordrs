use std::collections::HashMap;
use std::fmt::{Display, Formatter};
use std::str::FromStr;

use serde::de::{self, Visitor};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::parsers::V2ModalSubmission;
use crate::types::Emoji;

#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Default)]
pub struct Snowflake {
    raw: String,
    numeric: Option<u64>,
}

impl Snowflake {
    /// Discord epoch: January 1, 2015 00:00:00 UTC in milliseconds.
    const DISCORD_EPOCH: u64 = 1_420_070_400_000;

    pub fn new(value: impl Into<String>) -> Self {
        let raw = value.into();
        let numeric = raw.parse().ok();
        Self { raw, numeric }
    }

    pub fn as_str(&self) -> &str {
        &self.raw
    }

    pub fn as_u64(&self) -> Option<u64> {
        self.numeric
    }

    pub fn to_u64(&self) -> Option<u64> {
        self.as_u64()
    }

    /// Extracts the creation timestamp from this Snowflake as Unix milliseconds.
    ///
    /// Discord encodes the creation timestamp in the top 42 bits of every Snowflake ID.
    /// Returns `None` if the inner value is not a valid u64.
    pub fn timestamp(&self) -> Option<u64> {
        let raw = self.numeric?;
        // (raw >> 22) gives milliseconds since Discord epoch
        Some((raw >> 22) + Self::DISCORD_EPOCH)
    }
}

impl Display for Snowflake {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.raw)
    }
}

impl From<u64> for Snowflake {
    fn from(value: u64) -> Self {
        Self {
            raw: value.to_string(),
            numeric: Some(value),
        }
    }
}

impl From<&str> for Snowflake {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

impl From<String> for Snowflake {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}

impl FromStr for Snowflake {
    type Err = std::convert::Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self::from(s))
    }
}

impl Serialize for Snowflake {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.raw)
    }
}

impl<'de> Deserialize<'de> for Snowflake {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct SnowflakeVisitor;

        impl<'de> Visitor<'de> for SnowflakeVisitor {
            type Value = Snowflake;

            fn expecting(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("a Discord snowflake encoded as a string or integer")
            }

            fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(Snowflake::from(value))
            }

            fn visit_i64<E>(self, value: i64) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                if value < 0 {
                    return Err(E::custom("snowflake cannot be negative"));
                }
                Ok(Snowflake::from(value as u64))
            }

            fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(Snowflake::from(value))
            }

            fn visit_string<E>(self, value: String) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(Snowflake::from(value))
            }
        }

        deserializer.deserialize_any(SnowflakeVisitor)
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct PermissionsBitField(pub u64);

impl PermissionsBitField {
    pub fn bits(self) -> u64 {
        self.0
    }

    pub fn contains(self, permission: u64) -> bool {
        self.0 & permission == permission
    }

    pub fn insert(&mut self, permission: u64) {
        self.0 |= permission;
    }

    pub fn remove(&mut self, permission: u64) {
        self.0 &= !permission;
    }
}

impl Serialize for PermissionsBitField {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.0.to_string())
    }
}

impl<'de> Deserialize<'de> for PermissionsBitField {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct PermissionsVisitor;

        impl<'de> Visitor<'de> for PermissionsVisitor {
            type Value = PermissionsBitField;

            fn expecting(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("a Discord permission bitfield encoded as a string or integer")
            }

            fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(PermissionsBitField(value))
            }

            fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                value
                    .parse()
                    .map(PermissionsBitField)
                    .map_err(|error| E::custom(format!("invalid permission bitfield: {error}")))
            }

            fn visit_string<E>(self, value: String) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                self.visit_str(&value)
            }
        }

        deserializer.deserialize_any(PermissionsVisitor)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct User {
    pub id: Snowflake,
    pub username: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub global_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub discriminator: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub avatar: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bot: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mfa_enabled: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub banner: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub accent_color: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub verified: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub flags: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub premium_type: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub public_flags: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub avatar_decoration_data: Option<serde_json::Value>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct RoleTags {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bot_id: Option<Snowflake>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub integration_id: Option<Snowflake>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub premium_subscriber: Option<bool>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct Role {
    pub id: Snowflake,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hoist: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub managed: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mentionable: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub permissions: Option<PermissionsBitField>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub position: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unicode_emoji: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<RoleTags>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub flags: Option<u64>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct Attachment {
    pub id: Snowflake,
    pub filename: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub height: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub width: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ephemeral: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_secs: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub waveform: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub flags: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
#[non_exhaustive]
pub enum ChannelType {
    Text = 0,
    Dm = 1,
    Voice = 2,
    GroupDm = 3,
    Category = 4,
    News = 5,
    StageVoice = 13,
    GuildDirectory = 14,
    GuildForum = 15,
    GuildMedia = 16,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct PermissionOverwrite {
    pub id: Snowflake,
    #[serde(rename = "type")]
    pub kind: u8,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allow: Option<PermissionsBitField>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deny: Option<PermissionsBitField>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct Channel {
    pub id: Snowflake,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub guild_id: Option<Snowflake>,
    #[serde(rename = "type")]
    pub kind: u8,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub topic: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_id: Option<Snowflake>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nsfw: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bitrate: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_limit: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rate_limit_per_user: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_message_id: Option<Snowflake>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub owner_id: Option<Snowflake>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub application_id: Option<Snowflake>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub position: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub permission_overwrites: Option<Vec<PermissionOverwrite>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rtc_region: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub video_quality_mode: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_auto_archive_duration: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub flags: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_pin_timestamp: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thread_metadata: Option<ThreadMetadata>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message_count: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub member_count: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_message_sent: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub available_tags: Option<Vec<ForumTag>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub applied_tags: Option<Vec<Snowflake>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_reaction_emoji: Option<DefaultReaction>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_thread_rate_limit_per_user: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_sort_order: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_forum_layout: Option<u64>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct DefaultReaction {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub emoji_id: Option<Snowflake>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub emoji_name: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct ForumTag {
    pub id: Snowflake,
    pub name: String,
    #[serde(default)]
    pub moderated: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub emoji_id: Option<Snowflake>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub emoji_name: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct Member {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user: Option<User>,
    #[serde(default)]
    pub roles: Vec<Snowflake>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nick: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub joined_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub permissions: Option<PermissionsBitField>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deaf: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mute: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub avatar: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub premium_since: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pending: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub communication_disabled_until: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub flags: Option<u64>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct VoiceState {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub guild_id: Option<Snowflake>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub channel_id: Option<Snowflake>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_id: Option<Snowflake>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub member: Option<Member>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
    #[serde(default)]
    pub deaf: bool,
    #[serde(default)]
    pub mute: bool,
    #[serde(default)]
    pub self_deaf: bool,
    #[serde(default)]
    pub self_mute: bool,
    #[serde(default)]
    pub suppress: bool,
    #[serde(default)]
    pub self_stream: bool,
    #[serde(default)]
    pub self_video: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_to_speak_timestamp: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct VoiceServerUpdate {
    pub guild_id: Snowflake,
    pub token: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub endpoint: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct Guild {
    pub id: Snowflake,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub owner_id: Option<Snowflake>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unavailable: Option<bool>,
    #[serde(default)]
    pub roles: Vec<Role>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub splash: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub banner: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default)]
    pub features: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub premium_tier: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub member_count: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_presences: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_members: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vanity_url_code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system_channel_id: Option<Snowflake>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rules_channel_id: Option<Snowflake>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub preferred_locale: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub afk_channel_id: Option<Snowflake>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub afk_timeout: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub verification_level: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_message_notifications: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub explicit_content_filter: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mfa_level: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub application_id: Option<Snowflake>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system_channel_flags: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub public_updates_channel_id: Option<Snowflake>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nsfw_level: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub premium_subscription_count: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub premium_progress_bar_enabled: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub safety_alerts_channel_id: Option<Snowflake>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub approximate_member_count: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub approximate_presence_count: Option<u64>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct CurrentUserGuild {
    pub id: Snowflake,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
    #[serde(default)]
    pub owner: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub permissions: Option<String>,
    #[serde(default)]
    pub features: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct GuildPreview {
    pub id: Snowflake,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub splash: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub discovery_splash: Option<String>,
    #[serde(default)]
    pub emojis: Vec<Emoji>,
    #[serde(default)]
    pub features: Vec<String>,
    pub approximate_member_count: u64,
    pub approximate_presence_count: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default)]
    pub stickers: Vec<Sticker>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct VanityUrl {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uses: Option<u64>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct GuildPruneCount {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pruned: Option<u64>,
}

pub type GuildPruneResult = GuildPruneCount;

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct BulkGuildBanRequest {
    pub user_ids: Vec<Snowflake>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delete_message_seconds: Option<u64>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct BulkGuildBanResponse {
    #[serde(default)]
    pub banned_users: Vec<Snowflake>,
    #[serde(default)]
    pub failed_users: Vec<Snowflake>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct VoiceRegion {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub optimal: bool,
    #[serde(default)]
    pub deprecated: bool,
    #[serde(default)]
    pub custom: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct AutoModerationRule {
    pub id: Snowflake,
    pub guild_id: Snowflake,
    pub name: String,
    pub creator_id: Snowflake,
    pub event_type: u8,
    pub trigger_type: u8,
    #[serde(default)]
    pub trigger_metadata: AutoModerationTriggerMetadata,
    #[serde(default)]
    pub actions: Vec<AutoModerationAction>,
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub exempt_roles: Vec<Snowflake>,
    #[serde(default)]
    pub exempt_channels: Vec<Snowflake>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct AutoModerationTriggerMetadata {
    #[serde(default)]
    pub keyword_filter: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub regex_patterns: Option<Vec<String>>,
    #[serde(default)]
    pub presets: Vec<u8>,
    #[serde(default)]
    pub allow_list: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mention_total_limit: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mention_raid_protection_enabled: Option<bool>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct AutoModerationAction {
    #[serde(rename = "type")]
    pub kind: u8,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<AutoModerationActionMetadata>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct AutoModerationActionMetadata {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub channel_id: Option<Snowflake>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_seconds: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub custom_message: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct Message {
    pub id: Snowflake,
    pub channel_id: Snowflake,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub guild_id: Option<Snowflake>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub author: Option<User>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub member: Option<Member>,
    #[serde(default)]
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timestamp: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub edited_timestamp: Option<String>,
    #[serde(default)]
    pub mentions: Vec<User>,
    #[serde(default)]
    pub attachments: Vec<Attachment>,
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub kind: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pinned: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tts: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub flags: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub webhook_id: Option<Snowflake>,
    #[serde(default)]
    pub embeds: Vec<Embed>,
    #[serde(default)]
    pub reactions: Vec<Reaction>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mention_everyone: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mention_channels: Option<Vec<ChannelMention>>,
    #[serde(default)]
    pub sticker_items: Vec<StickerItem>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub position: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message_reference: Option<MessageReference>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub components: Option<Vec<serde_json::Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub poll: Option<Poll>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct ApplicationCommandOptionChoice {
    pub name: String,
    #[serde(default)]
    pub value: serde_json::Value,
}

impl ApplicationCommandOptionChoice {
    pub fn new(name: impl Into<String>, value: impl Serialize) -> Self {
        Self {
            name: name.into(),
            value: serde_json::to_value(value)
                .expect("application command option choice should serialize"),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct PollMedia {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub emoji: Option<Emoji>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct PollAnswer {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub answer_id: Option<u64>,
    pub poll_media: PollMedia,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct PollAnswerCount {
    pub id: u64,
    pub count: u64,
    pub me_voted: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct PollResults {
    pub is_finalized: bool,
    #[serde(default)]
    pub answer_counts: Vec<PollAnswerCount>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct Poll {
    pub question: PollMedia,
    #[serde(default)]
    pub answers: Vec<PollAnswer>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expiry: Option<String>,
    #[serde(default)]
    pub allow_multiselect: bool,
    #[serde(default)]
    pub layout_type: u8,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub results: Option<PollResults>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct PollAnswerVoters {
    #[serde(default)]
    pub users: Vec<User>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct CreatePoll {
    pub question: PollMedia,
    #[serde(default)]
    pub answers: Vec<PollAnswer>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allow_multiselect: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub layout_type: Option<u8>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct ChannelMention {
    pub id: Snowflake,
    pub guild_id: Snowflake,
    #[serde(rename = "type")]
    pub kind: u8,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct MessageReference {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message_id: Option<Snowflake>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub channel_id: Option<Snowflake>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub guild_id: Option<Snowflake>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fail_if_not_exists: Option<bool>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct Ban {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user: Option<User>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct Invite {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub guild: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub channel: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub inviter: Option<User>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uses: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_uses: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_age: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temporary: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct Webhook {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<Snowflake>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kind: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub guild_id: Option<Snowflake>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub channel_id: Option<Snowflake>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub avatar: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub token: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub application_id: Option<Snowflake>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct AuditLogEntry {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<Snowflake>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_id: Option<Snowflake>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_id: Option<Snowflake>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub action_type: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub changes: Option<Vec<serde_json::Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct ThreadMember {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<Snowflake>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_id: Option<Snowflake>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub join_timestamp: Option<String>,
    #[serde(default)]
    pub flags: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub member: Option<Member>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct ThreadListResponse {
    #[serde(default)]
    pub threads: Vec<Channel>,
    #[serde(default)]
    pub members: Vec<ThreadMember>,
    #[serde(default)]
    pub has_more: bool,
}

#[derive(Clone, Debug, Default)]
pub struct ThreadMemberQuery {
    pub with_member: Option<bool>,
    pub after: Option<Snowflake>,
    pub limit: Option<u64>,
}

#[derive(Clone, Debug, Default)]
pub struct ArchivedThreadsQuery {
    pub before: Option<String>,
    pub limit: Option<u64>,
}

#[derive(Clone, Debug, Default)]
pub struct JoinedArchivedThreadsQuery {
    pub before: Option<Snowflake>,
    pub limit: Option<u64>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct ApplicationCommandOption {
    #[serde(rename = "type")]
    pub kind: u8,
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub required: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub autocomplete: Option<bool>,
    #[serde(default)]
    pub options: Vec<ApplicationCommandOption>,
    #[serde(default)]
    pub choices: Vec<ApplicationCommandOptionChoice>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_value: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_value: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_length: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_length: Option<u16>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ApplicationIntegrationType(pub u8);

impl ApplicationIntegrationType {
    pub const GUILD_INSTALL: Self = Self(0);
    pub const USER_INSTALL: Self = Self(1);
}

impl Default for ApplicationIntegrationType {
    fn default() -> Self {
        Self::GUILD_INSTALL
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct InteractionContextType(pub u8);

impl InteractionContextType {
    pub const GUILD: Self = Self(0);
    pub const BOT_DM: Self = Self(1);
    pub const PRIVATE_CHANNEL: Self = Self(2);
}

impl Default for InteractionContextType {
    fn default() -> Self {
        Self::GUILD
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ApplicationCommandHandlerType(pub u8);

impl ApplicationCommandHandlerType {
    pub const APP_HANDLER: Self = Self(1);
    pub const DISCORD_LAUNCH_ACTIVITY: Self = Self(2);
}

impl Default for ApplicationCommandHandlerType {
    fn default() -> Self {
        Self::APP_HANDLER
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct ApplicationCommand {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<Snowflake>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub application_id: Option<Snowflake>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub guild_id: Option<Snowflake>,
    #[serde(rename = "type")]
    pub kind: u8,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name_localizations: Option<HashMap<String, String>>,
    #[serde(default)]
    pub description: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description_localizations: Option<HashMap<String, String>>,
    #[serde(default)]
    pub options: Vec<ApplicationCommandOption>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_member_permissions: Option<PermissionsBitField>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dm_permission: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub integration_types: Option<Vec<ApplicationIntegrationType>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub contexts: Option<Vec<InteractionContextType>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub handler: Option<ApplicationCommandHandlerType>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<Snowflake>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nsfw: Option<bool>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct Application {
    pub id: Snowflake,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
    pub description: String,
    #[serde(default)]
    pub rpc_origins: Vec<String>,
    #[serde(default)]
    pub bot_public: bool,
    #[serde(default)]
    pub bot_require_code_grant: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub terms_of_service_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub privacy_policy_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub owner: Option<User>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub verify_key: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub team: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub guild_id: Option<Snowflake>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub primary_sku_id: Option<Snowflake>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub slug: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cover_image: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub flags: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub approximate_guild_count: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub approximate_user_install_count: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub interactions_endpoint_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role_connections_verification_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub custom_install_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub install_params: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub integration_types_config: Option<HashMap<String, serde_json::Value>>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct ApplicationRoleConnectionMetadata {
    #[serde(rename = "type")]
    pub kind: u8,
    pub key: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name_localizations: Option<HashMap<String, String>>,
    pub description: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description_localizations: Option<HashMap<String, String>>,
}

impl ApplicationCommand {
    /// Returns the command ID when Discord has assigned one.
    pub fn id_opt(&self) -> Option<&Snowflake> {
        self.id.as_ref()
    }

    /// Returns the creation timestamp once Discord has assigned an ID.
    pub fn created_at(&self) -> Option<u64> {
        self.id_opt().and_then(Snowflake::timestamp)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct InteractionContextData {
    pub id: Snowflake,
    pub application_id: Snowflake,
    pub token: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub guild_id: Option<Snowflake>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub channel_id: Option<Snowflake>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user: Option<User>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub member: Option<Member>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub app_permissions: Option<PermissionsBitField>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub locale: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub guild_locale: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entitlements: Option<Vec<Entitlement>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<InteractionContextType>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub authorizing_integration_owners: Option<HashMap<String, Snowflake>>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct CommandInteractionOption {
    #[serde(rename = "type")]
    pub kind: u8,
    pub name: String,
    #[serde(default)]
    pub options: Vec<CommandInteractionOption>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub focused: Option<bool>,
}

impl CommandInteractionOption {
    pub fn is_focused(&self) -> bool {
        self.focused.unwrap_or(false)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct CommandInteractionData {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<Snowflake>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub kind: Option<u8>,
    #[serde(default)]
    pub options: Vec<CommandInteractionOption>,
    #[serde(default)]
    pub resolved: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_id: Option<Snowflake>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct ComponentInteractionData {
    pub custom_id: String,
    pub component_type: u8,
    #[serde(default)]
    pub values: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct ChatInputCommandInteraction {
    pub context: InteractionContextData,
    pub data: CommandInteractionData,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct UserContextMenuInteraction {
    pub context: InteractionContextData,
    pub data: CommandInteractionData,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct MessageContextMenuInteraction {
    pub context: InteractionContextData,
    pub data: CommandInteractionData,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct AutocompleteInteraction {
    pub context: InteractionContextData,
    pub data: CommandInteractionData,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct ComponentInteraction {
    pub context: InteractionContextData,
    pub data: ComponentInteractionData,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ModalSubmitInteraction {
    pub context: InteractionContextData,
    pub submission: V2ModalSubmission,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct PingInteraction {
    pub context: InteractionContextData,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[non_exhaustive]
pub enum Interaction {
    Ping(PingInteraction),
    ChatInputCommand(ChatInputCommandInteraction),
    UserContextMenu(UserContextMenuInteraction),
    MessageContextMenu(MessageContextMenuInteraction),
    Autocomplete(AutocompleteInteraction),
    Component(ComponentInteraction),
    ModalSubmit(ModalSubmitInteraction),
    Unknown {
        context: InteractionContextData,
        kind: u8,
        raw_data: serde_json::Value,
    },
}

impl Interaction {
    pub fn context(&self) -> &InteractionContextData {
        match self {
            Interaction::Ping(interaction) => &interaction.context,
            Interaction::ChatInputCommand(interaction) => &interaction.context,
            Interaction::UserContextMenu(interaction) => &interaction.context,
            Interaction::MessageContextMenu(interaction) => &interaction.context,
            Interaction::Autocomplete(interaction) => &interaction.context,
            Interaction::Component(interaction) => &interaction.context,
            Interaction::ModalSubmit(interaction) => &interaction.context,
            Interaction::Unknown { context, .. } => context,
        }
    }

    pub fn id(&self) -> &Snowflake {
        &self.context().id
    }

    pub fn token(&self) -> &str {
        &self.context().token
    }

    pub fn application_id(&self) -> &Snowflake {
        &self.context().application_id
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct InteractionCallbackResponse {
    #[serde(rename = "type")]
    pub kind: u8,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct CreateMessage {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub components: Option<Vec<serde_json::Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub flags: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub embeds: Option<Vec<serde_json::Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message_reference: Option<MessageReference>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sticker_ids: Option<Vec<Snowflake>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nonce: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub poll: Option<CreatePoll>,
    #[serde(default, skip_serializing_if = "is_false")]
    pub enforce_nonce: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct CreateDmChannel {
    pub recipient_id: Snowflake,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct SessionStartLimit {
    pub total: u32,
    pub remaining: u32,
    pub reset_after: u64,
    pub max_concurrency: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct GatewayBot {
    pub url: String,
    pub shards: u32,
    pub session_start_limit: SessionStartLimit,
}

fn is_false(v: &bool) -> bool {
    !v
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct EmbedField {
    pub name: String,
    pub value: String,
    #[serde(default, skip_serializing_if = "is_false")]
    pub inline: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct EmbedAuthor {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon_url: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct EmbedFooter {
    pub text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon_url: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct EmbedMedia {
    pub url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub proxy_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub height: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub width: Option<u64>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct Embed {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timestamp: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub footer: Option<EmbedFooter>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image: Option<EmbedMedia>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thumbnail: Option<EmbedMedia>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub video: Option<EmbedMedia>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub author: Option<EmbedAuthor>,
    #[serde(default)]
    pub fields: Vec<EmbedField>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct Reaction {
    #[serde(default)]
    pub count: u64,
    #[serde(default)]
    pub me: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub emoji: Option<Emoji>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct StickerItem {
    pub id: Snowflake,
    pub name: String,
    #[serde(rename = "format_type", skip_serializing_if = "Option::is_none")]
    pub kind: Option<u8>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct Sticker {
    pub id: Snowflake,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pack_id: Option<Snowflake>,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default)]
    pub tags: String,
    #[serde(rename = "type")]
    pub kind: u8,
    pub format_type: u8,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub available: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub guild_id: Option<Snowflake>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user: Option<User>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sort_value: Option<u64>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct StickerPack {
    pub id: Snowflake,
    #[serde(default)]
    pub stickers: Vec<Sticker>,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sku_id: Option<Snowflake>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cover_sticker_id: Option<Snowflake>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub banner_asset_id: Option<Snowflake>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct StickerPackList {
    #[serde(default)]
    pub sticker_packs: Vec<StickerPack>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct StageInstance {
    pub id: Snowflake,
    pub guild_id: Snowflake,
    pub channel_id: Snowflake,
    pub topic: String,
    pub privacy_level: u8,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub discoverable_disabled: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub guild_scheduled_event_id: Option<Snowflake>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct GuildScheduledEvent {
    pub id: Snowflake,
    pub guild_id: Snowflake,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub channel_id: Option<Snowflake>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub creator_id: Option<Snowflake>,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub scheduled_start_time: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scheduled_end_time: Option<String>,
    pub privacy_level: u8,
    pub status: u8,
    pub entity_type: u8,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entity_id: Option<Snowflake>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entity_metadata: Option<GuildScheduledEventEntityMetadata>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub creator: Option<User>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_count: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recurrence_rule: Option<GuildScheduledEventRecurrenceRule>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct GuildScheduledEventEntityMetadata {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub location: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct GuildScheduledEventRecurrenceRuleNWeekday {
    pub n: i8,
    pub day: u8,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct GuildScheduledEventRecurrenceRule {
    pub start: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end: Option<String>,
    pub frequency: u8,
    pub interval: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub by_weekday: Option<Vec<u8>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub by_n_weekday: Option<Vec<GuildScheduledEventRecurrenceRuleNWeekday>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub by_month: Option<Vec<u8>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub by_month_day: Option<Vec<u8>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub by_year_day: Option<Vec<u16>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub count: Option<u64>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct GuildScheduledEventUser {
    pub guild_scheduled_event_id: Snowflake,
    pub user: User,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub member: Option<Member>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct Sku {
    pub id: Snowflake,
    #[serde(rename = "type")]
    pub kind: u8,
    pub application_id: Snowflake,
    pub name: String,
    pub slug: String,
    pub flags: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dependent_sku_id: Option<Snowflake>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub access_type: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub premium: Option<bool>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct Entitlement {
    pub id: Snowflake,
    pub sku_id: Snowflake,
    pub application_id: Snowflake,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_id: Option<Snowflake>,
    #[serde(rename = "type")]
    pub kind: u8,
    pub deleted: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub starts_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ends_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub guild_id: Option<Snowflake>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub consumed: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subscription_id: Option<Snowflake>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct CreateTestEntitlement {
    pub sku_id: Snowflake,
    pub owner_id: Snowflake,
    pub owner_type: u8,
}

#[derive(Clone, Debug, Default)]
pub struct EntitlementQuery {
    pub user_id: Option<Snowflake>,
    pub sku_ids: Vec<Snowflake>,
    pub before: Option<Snowflake>,
    pub after: Option<Snowflake>,
    pub limit: Option<u64>,
    pub guild_id: Option<Snowflake>,
    pub exclude_ended: Option<bool>,
    pub exclude_deleted: Option<bool>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct Subscription {
    pub id: Snowflake,
    pub user_id: Snowflake,
    #[serde(default)]
    pub sku_ids: Vec<Snowflake>,
    #[serde(default)]
    pub entitlement_ids: Vec<Snowflake>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub renewal_sku_ids: Option<Vec<Snowflake>>,
    pub current_period_start: String,
    pub current_period_end: String,
    pub status: u8,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub canceled_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub country: Option<String>,
}

#[derive(Clone, Debug, Default)]
pub struct SubscriptionQuery {
    pub before: Option<Snowflake>,
    pub after: Option<Snowflake>,
    pub limit: Option<u64>,
    pub user_id: Option<Snowflake>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct IntegrationAccount {
    pub id: String,
    pub name: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct IntegrationApplication {
    pub id: Snowflake,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
    #[serde(default)]
    pub description: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bot: Option<User>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct Integration {
    pub id: Snowflake,
    pub name: String,
    #[serde(rename = "type")]
    pub kind: String,
    #[serde(default)]
    pub enabled: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub syncing: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role_id: Option<Snowflake>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enable_emoticons: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expire_behavior: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expire_grace_period: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user: Option<User>,
    pub account: IntegrationAccount,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub synced_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subscriber_count: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub revoked: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub application: Option<IntegrationApplication>,
    #[serde(default)]
    pub scopes: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct SoundboardSound {
    pub name: String,
    pub sound_id: Snowflake,
    pub volume: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub emoji_id: Option<Snowflake>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub emoji_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub guild_id: Option<Snowflake>,
    #[serde(default)]
    pub available: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user: Option<User>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct SoundboardSoundList {
    #[serde(default)]
    pub items: Vec<SoundboardSound>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct GuildWidgetSettings {
    pub enabled: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub channel_id: Option<Snowflake>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct FollowedChannel {
    pub channel_id: Snowflake,
    pub webhook_id: Snowflake,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct WelcomeScreenChannel {
    pub channel_id: Snowflake,
    pub description: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub emoji_id: Option<Snowflake>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub emoji_name: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct WelcomeScreen {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default)]
    pub welcome_channels: Vec<WelcomeScreenChannel>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct GuildTemplate {
    pub code: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub usage_count: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub creator_id: Option<Snowflake>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub creator: Option<User>,
    pub created_at: String,
    pub updated_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_guild_id: Option<Snowflake>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub serialized_source_guild: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_dirty: Option<bool>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct GuildOnboarding {
    pub guild_id: Snowflake,
    #[serde(default)]
    pub prompts: Vec<serde_json::Value>,
    #[serde(default)]
    pub default_channel_ids: Vec<Snowflake>,
    pub enabled: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mode: Option<u8>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct Presence {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_id: Option<Snowflake>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub activities: Option<Vec<Activity>>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ActivityType(pub u8);

impl ActivityType {
    pub const PLAYING: Self = Self(0);
    pub const STREAMING: Self = Self(1);
    pub const LISTENING: Self = Self(2);
    pub const WATCHING: Self = Self(3);
    pub const CUSTOM: Self = Self(4);
    pub const COMPETING: Self = Self(5);
}

impl Default for ActivityType {
    fn default() -> Self {
        Self::PLAYING
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct ActivityTimestamps {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end: Option<u64>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct ActivityParty {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size: Option<Vec<u64>>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct ActivityAssets {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub large_image: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub large_text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub small_image: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub small_text: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct ActivitySecrets {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub join: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub spectate: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub match_secret: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct ActivityButton {
    pub label: String,
    pub url: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct Activity {
    pub name: String,
    #[serde(default, rename = "type")]
    pub kind: ActivityType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timestamps: Option<ActivityTimestamps>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub application_id: Option<Snowflake>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub state: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub emoji: Option<Emoji>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub party: Option<ActivityParty>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub assets: Option<ActivityAssets>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub secrets: Option<ActivitySecrets>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub buttons: Option<Vec<ActivityButton>>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct UpdatePresence {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub since: Option<u64>,
    #[serde(default)]
    pub activities: Vec<Activity>,
    pub status: String,
    #[serde(default)]
    pub afk: bool,
}

impl UpdatePresence {
    pub fn online_with_activity(name: impl Into<String>) -> Self {
        Self {
            since: None,
            activities: vec![Activity {
                name: name.into(),
                kind: ActivityType::PLAYING,
                ..Activity::default()
            }],
            status: "online".to_string(),
            afk: false,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RequestGuildMembers {
    pub guild_id: Snowflake,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub query: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub presences: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_ids: Option<Vec<Snowflake>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nonce: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct ThreadMetadata {
    #[serde(default)]
    pub archived: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auto_archive_duration: Option<u64>,
    #[serde(default)]
    pub locked: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub invitable: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub create_timestamp: Option<String>,
}

// --- DiscordModel trait ---

/// Trait for all Discord data models that have a Snowflake ID.
///
/// Parallels discord.js's `Base` class, providing a common interface
/// for ID access and creation timestamp extraction.
pub trait DiscordModel: Send + Sync + 'static {
    /// Returns the Snowflake ID of this model.
    fn id(&self) -> &Snowflake;

    /// Returns the Snowflake ID when the model has one.
    ///
    /// Most Discord models always carry an ID, so the default implementation
    /// simply delegates to [`DiscordModel::id`]. Models that can exist before
    /// Discord assigns an ID, such as `ApplicationCommand`, override this.
    fn id_opt(&self) -> Option<&Snowflake> {
        Some(self.id())
    }

    /// Returns the creation timestamp as Unix milliseconds, extracted from the Snowflake ID.
    fn created_at(&self) -> Option<u64> {
        self.id_opt().and_then(Snowflake::timestamp)
    }
}

impl DiscordModel for User {
    fn id(&self) -> &Snowflake {
        &self.id
    }
}

impl DiscordModel for Guild {
    fn id(&self) -> &Snowflake {
        &self.id
    }
}

impl DiscordModel for Channel {
    fn id(&self) -> &Snowflake {
        &self.id
    }
}

impl DiscordModel for Message {
    fn id(&self) -> &Snowflake {
        &self.id
    }
}

impl DiscordModel for Role {
    fn id(&self) -> &Snowflake {
        &self.id
    }
}

impl DiscordModel for Attachment {
    fn id(&self) -> &Snowflake {
        &self.id
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{
        ApplicationCommand, ApplicationCommandOptionChoice, Attachment, AutocompleteInteraction,
        Channel, ChatInputCommandInteraction, CommandInteractionData, CommandInteractionOption,
        ComponentInteraction, ComponentInteractionData, CreateDmChannel, CreateMessage, CreatePoll,
        DefaultReaction, DiscordModel, Embed, EmbedField, ForumTag, GatewayBot, Guild,
        GuildScheduledEvent, Integration, Interaction, InteractionCallbackResponse,
        InteractionContextData, Member, Message, MessageContextMenuInteraction,
        ModalSubmitInteraction, PermissionsBitField, PingInteraction, PollAnswer, PollAnswerCount,
        PollAnswerVoters, PollMedia, PollResults, Presence, Reaction, Role, SessionStartLimit,
        Snowflake, StickerItem, Subscription, ThreadListResponse, ThreadMember, ThreadMetadata,
        User, UserContextMenuInteraction,
    };
    use crate::parsers::V2ModalSubmission;

    #[test]
    fn snowflake_deserializes_from_string_and_number() {
        let string_value: Snowflake = serde_json::from_value(json!("123")).unwrap();
        let number_value: Snowflake = serde_json::from_value(json!(123)).unwrap();

        assert_eq!(string_value.as_str(), "123");
        assert_eq!(number_value.as_str(), "123");
    }

    #[test]
    fn permissions_round_trip_through_string_wire_format() {
        let permissions = PermissionsBitField(8);
        let json = serde_json::to_value(permissions).unwrap();
        assert_eq!(json, json!("8"));

        let parsed: PermissionsBitField = serde_json::from_value(json).unwrap();
        assert_eq!(parsed.bits(), 8);
    }

    #[test]
    fn typed_models_keep_wire_shape() {
        let user: User = serde_json::from_value(json!({
            "id": "42",
            "username": "discordrs",
            "global_name": "discordrs"
        }))
        .unwrap();

        let serialized = serde_json::to_value(&user).unwrap();
        assert_eq!(serialized["id"], json!("42"));
        assert_eq!(serialized["username"], json!("discordrs"));
    }

    #[test]
    fn application_command_option_choice_new_serializes_value() {
        let choice = ApplicationCommandOptionChoice::new("Support", "support");
        let serialized = serde_json::to_value(choice).unwrap();

        assert_eq!(serialized["name"], json!("Support"));
        assert_eq!(serialized["value"], json!("support"));
    }

    #[test]
    fn snowflake_timestamp_extracts_creation_time() {
        // Discord Snowflake: timestamp is in the top 42 bits
        let sf = Snowflake::from(1759288472266248192u64);
        let ts = sf.timestamp().expect("should extract timestamp");
        // Should be a reasonable Unix timestamp (after 2020)
        assert!(ts > 1_577_836_800_000u64); // after 2020-01-01
    }

    #[test]
    fn application_command_id_opt_is_none_until_discord_assigns_an_id() {
        let command = ApplicationCommand {
            name: "ping".to_string(),
            description: "Ping".to_string(),
            ..ApplicationCommand::default()
        };

        assert!(command.id_opt().is_none());
        assert_eq!(command.created_at(), None);
    }

    #[test]
    fn application_command_created_at_uses_assigned_id() {
        let command = ApplicationCommand {
            id: Some(Snowflake::from(1759288472266248192u64)),
            name: "ping".to_string(),
            description: "Ping".to_string(),
            ..ApplicationCommand::default()
        };

        assert_eq!(
            command.id_opt().map(Snowflake::as_str),
            Some("1759288472266248192")
        );
        assert!(command.created_at().is_some());
    }

    #[test]
    fn snowflake_helpers_cover_string_numeric_and_error_paths() {
        let snowflake = Snowflake::new("1759288472266248192");
        let parsed = "42".parse::<Snowflake>().unwrap();
        let invalid = Snowflake::new("not-a-number");

        assert_eq!(snowflake.as_str(), "1759288472266248192");
        assert_eq!(snowflake.as_u64(), Some(1_759_288_472_266_248_192));
        assert_eq!(snowflake.to_string(), "1759288472266248192");
        assert_eq!(parsed.as_str(), "42");
        assert_eq!(invalid.as_u64(), None);

        let error = serde_json::from_value::<Snowflake>(json!(-1)).unwrap_err();
        assert!(error.to_string().contains("snowflake cannot be negative"));
    }

    #[test]
    fn permissions_bitfield_helpers_cover_mutation_and_invalid_wire_values() {
        let mut permissions = PermissionsBitField(0b0011);
        assert!(permissions.contains(0b0001));
        assert!(!permissions.contains(0b0100));

        permissions.insert(0b0100);
        assert_eq!(permissions.bits(), 0b0111);
        assert!(permissions.contains(0b0110));

        permissions.remove(0b0010);
        assert_eq!(permissions.bits(), 0b0101);
        assert!(!permissions.contains(0b0010));

        let error = serde_json::from_value::<PermissionsBitField>(json!("oops")).unwrap_err();
        assert!(error.to_string().contains("invalid permission bitfield"));
    }

    #[test]
    fn channel_and_create_message_keep_wire_aliases_and_omit_absent_optionals() {
        let channel = Channel {
            id: Snowflake::from("10"),
            kind: 5,
            name: Some("announcements".to_string()),
            ..Channel::default()
        };
        let message = CreateMessage {
            content: Some("hello".to_string()),
            ..CreateMessage::default()
        };

        let channel_json = serde_json::to_value(&channel).unwrap();
        let message_json = serde_json::to_value(&message).unwrap();

        assert_eq!(channel_json["id"], json!("10"));
        assert_eq!(channel_json["type"], json!(5));
        assert_eq!(channel_json["name"], json!("announcements"));
        assert!(channel_json.get("guild_id").is_none());
        assert!(channel_json.get("topic").is_none());

        assert_eq!(message_json, json!({ "content": "hello" }));
    }

    #[test]
    fn forum_channel_fields_decode_tags_and_default_reaction() {
        let channel: Channel = serde_json::from_value(json!({
            "id": "10",
            "type": 15,
            "available_tags": [{
                "id": "11",
                "name": "Support",
                "moderated": true,
                "emoji_name": "ticket"
            }],
            "applied_tags": ["11"],
            "default_reaction_emoji": { "emoji_name": "ok" },
            "default_thread_rate_limit_per_user": 30,
            "default_sort_order": 1,
            "default_forum_layout": 2
        }))
        .unwrap();

        let tag = &channel.available_tags.as_ref().unwrap()[0];
        assert_eq!(tag.id.as_str(), "11");
        assert_eq!(tag.name, "Support");
        assert!(tag.moderated);
        assert_eq!(tag.emoji_name.as_deref(), Some("ticket"));
        assert_eq!(channel.applied_tags.as_ref().unwrap()[0].as_str(), "11");
        assert_eq!(
            channel
                .default_reaction_emoji
                .as_ref()
                .and_then(|reaction| reaction.emoji_name.as_deref()),
            Some("ok")
        );
        assert_eq!(channel.default_thread_rate_limit_per_user, Some(30));

        let serialized = serde_json::to_value(Channel {
            id: Snowflake::from("20"),
            kind: 15,
            available_tags: Some(vec![ForumTag {
                id: Snowflake::from("21"),
                name: "News".to_string(),
                ..ForumTag::default()
            }]),
            default_reaction_emoji: Some(DefaultReaction {
                emoji_id: Some(Snowflake::from("22")),
                ..DefaultReaction::default()
            }),
            ..Channel::default()
        })
        .unwrap();
        assert_eq!(serialized["available_tags"][0]["name"], json!("News"));
        assert_eq!(
            serialized["default_reaction_emoji"]["emoji_id"],
            json!("22")
        );
    }

    #[test]
    fn message_poll_decodes_and_create_poll_keeps_wire_shape() {
        let message: Message = serde_json::from_value(json!({
            "id": "500",
            "channel_id": "600",
            "poll": {
                "question": {
                    "text": "Ship it?"
                },
                "answers": [
                    {
                        "answer_id": 1,
                        "poll_media": {
                            "text": "Yes",
                            "emoji": { "name": "yes" }
                        }
                    },
                    {
                        "answer_id": 2,
                        "poll_media": {
                            "text": "No"
                        }
                    }
                ],
                "expiry": "2026-04-30T00:00:00Z",
                "allow_multiselect": true,
                "layout_type": 1,
                "results": {
                    "is_finalized": false,
                    "answer_counts": [
                        { "id": 1, "count": 3, "me_voted": true },
                        { "id": 2, "count": 1, "me_voted": false }
                    ]
                }
            }
        }))
        .unwrap();

        let poll = message.poll.expect("poll should decode");
        assert_eq!(poll.question.text.as_deref(), Some("Ship it?"));
        assert_eq!(poll.answers[0].answer_id, Some(1));
        assert_eq!(poll.answers[0].poll_media.text.as_deref(), Some("Yes"));
        assert_eq!(
            poll.answers[0]
                .poll_media
                .emoji
                .as_ref()
                .and_then(|emoji| emoji.name.as_deref()),
            Some("yes")
        );
        assert!(poll.allow_multiselect);
        assert_eq!(poll.layout_type, 1);
        let results = poll.results.expect("poll results should decode");
        assert!(!results.is_finalized);
        assert_eq!(results.answer_counts[0].count, 3);
        assert!(results.answer_counts[0].me_voted);

        let create_message = CreateMessage {
            poll: Some(CreatePoll {
                question: PollMedia {
                    text: Some("Pick one".to_string()),
                    ..PollMedia::default()
                },
                answers: vec![PollAnswer {
                    poll_media: PollMedia {
                        text: Some("A".to_string()),
                        ..PollMedia::default()
                    },
                    ..PollAnswer::default()
                }],
                duration: Some(24),
                allow_multiselect: Some(false),
                layout_type: Some(1),
            }),
            ..CreateMessage::default()
        };

        assert_eq!(
            serde_json::to_value(&create_message).unwrap(),
            json!({
                "poll": {
                    "question": { "text": "Pick one" },
                    "answers": [
                        { "poll_media": { "text": "A" } }
                    ],
                    "duration": 24,
                    "allow_multiselect": false,
                    "layout_type": 1
                }
            })
        );

        let _default_results = PollResults {
            answer_counts: vec![PollAnswerCount {
                id: 1,
                count: 0,
                me_voted: false,
            }],
            ..PollResults::default()
        };
    }

    #[test]
    fn monetization_poll_and_thread_response_models_decode() {
        let subscription: Subscription = serde_json::from_value(json!({
            "id": "1278078770116427839",
            "user_id": "1088605110638227537",
            "sku_ids": ["1158857122189168803"],
            "entitlement_ids": ["1"],
            "renewal_sku_ids": null,
            "current_period_start": "2024-08-27T19:48:44.406602+00:00",
            "current_period_end": "2024-09-27T19:48:44.406602+00:00",
            "status": 0,
            "canceled_at": null
        }))
        .unwrap();
        assert_eq!(subscription.sku_ids[0].as_str(), "1158857122189168803");
        assert_eq!(subscription.status, 0);

        let voters: PollAnswerVoters = serde_json::from_value(json!({
            "users": [{ "id": "42", "username": "voter" }]
        }))
        .unwrap();
        assert_eq!(voters.users[0].username, "voter");

        let threads: ThreadListResponse = serde_json::from_value(json!({
            "threads": [{ "id": "50", "type": 11, "name": "thread" }],
            "members": [{ "id": "50", "user_id": "42", "join_timestamp": "2026-04-29T00:00:00Z", "flags": 0 }],
            "has_more": false
        }))
        .unwrap();
        assert_eq!(threads.threads[0].id.as_str(), "50");
        assert_eq!(threads.members[0].user_id.as_ref().unwrap().as_str(), "42");

        let thread_member = ThreadMember {
            user_id: Some(Snowflake::from("42")),
            member: Some(Member {
                user: Some(User {
                    id: Snowflake::from("42"),
                    username: "member".to_string(),
                    ..User::default()
                }),
                ..Member::default()
            }),
            ..ThreadMember::default()
        };
        assert_eq!(
            thread_member
                .member
                .as_ref()
                .and_then(|member| member.user.as_ref())
                .map(|user| user.username.as_str()),
            Some("member")
        );
    }

    #[test]
    fn integration_model_decodes_core_guild_integration_shape() {
        let integration: Integration = serde_json::from_value(json!({
            "id": "100",
            "name": "Twitch",
            "type": "twitch",
            "enabled": true,
            "account": { "id": "abc", "name": "stream" },
            "application": {
                "id": "200",
                "name": "App",
                "description": "integration app"
            },
            "scopes": ["bot"]
        }))
        .unwrap();

        assert_eq!(integration.id.as_str(), "100");
        assert_eq!(integration.kind, "twitch");
        assert_eq!(integration.account.name, "stream");
        assert_eq!(integration.application.as_ref().unwrap().id.as_str(), "200");
    }

    #[test]
    fn embed_field_and_focus_helpers_follow_default_and_true_branches() {
        let default_field = EmbedField {
            name: "Name".to_string(),
            value: "Value".to_string(),
            ..EmbedField::default()
        };
        let inline_field = EmbedField {
            inline: true,
            ..default_field.clone()
        };
        let unfocused = CommandInteractionOption::default();
        let focused = CommandInteractionOption {
            focused: Some(true),
            ..CommandInteractionOption::default()
        };

        let default_json = serde_json::to_value(&default_field).unwrap();
        let inline_json = serde_json::to_value(&inline_field).unwrap();

        assert!(default_json.get("inline").is_none());
        assert_eq!(inline_json["inline"], json!(true));
        assert!(!unfocused.is_focused());
        assert!(focused.is_focused());
    }

    #[test]
    fn interaction_accessors_and_discord_model_trait_delegate_to_context_and_ids() {
        let context = InteractionContextData {
            id: Snowflake::from("99"),
            application_id: Snowflake::from("77"),
            token: "token-123".to_string(),
            ..InteractionContextData::default()
        };
        let interaction = Interaction::Component(ComponentInteraction {
            context: context.clone(),
            data: ComponentInteractionData {
                custom_id: "button".to_string(),
                component_type: 2,
                values: vec!["x".to_string()],
            },
        });
        let user = User {
            id: Snowflake::from(1759288472266248192u64),
            username: "discordrs".to_string(),
            ..User::default()
        };

        assert_eq!(interaction.context().id.as_str(), "99");
        assert_eq!(interaction.id().as_str(), "99");
        assert_eq!(interaction.application_id().as_str(), "77");
        assert_eq!(interaction.token(), "token-123");

        assert_eq!(DiscordModel::id(&user).as_str(), "1759288472266248192");
        assert_eq!(
            DiscordModel::id_opt(&user).map(Snowflake::as_str),
            Some("1759288472266248192")
        );
        assert!(DiscordModel::created_at(&user).is_some());
    }

    #[test]
    fn interaction_accessors_cover_all_variants() {
        fn context(id: &str, application_id: &str, token: &str) -> InteractionContextData {
            InteractionContextData {
                id: Snowflake::from(id),
                application_id: Snowflake::from(application_id),
                token: token.to_string(),
                ..InteractionContextData::default()
            }
        }

        let interactions = [
            Interaction::Ping(PingInteraction {
                context: context("1", "10", "ping-token"),
            }),
            Interaction::ChatInputCommand(ChatInputCommandInteraction {
                context: context("2", "20", "chat-token"),
                data: CommandInteractionData::default(),
            }),
            Interaction::UserContextMenu(UserContextMenuInteraction {
                context: context("3", "30", "user-token"),
                data: CommandInteractionData::default(),
            }),
            Interaction::MessageContextMenu(MessageContextMenuInteraction {
                context: context("4", "40", "message-token"),
                data: CommandInteractionData::default(),
            }),
            Interaction::Autocomplete(AutocompleteInteraction {
                context: context("5", "50", "autocomplete-token"),
                data: CommandInteractionData::default(),
            }),
            Interaction::Component(ComponentInteraction {
                context: context("6", "60", "component-token"),
                data: ComponentInteractionData::default(),
            }),
            Interaction::ModalSubmit(ModalSubmitInteraction {
                context: context("7", "70", "modal-token"),
                submission: V2ModalSubmission {
                    custom_id: "modal".to_string(),
                    components: vec![],
                },
            }),
            Interaction::Unknown {
                context: context("8", "80", "unknown-token"),
                kind: 99,
                raw_data: json!({ "kind": "unknown" }),
            },
        ];

        let expected = [
            ("1", "10", "ping-token"),
            ("2", "20", "chat-token"),
            ("3", "30", "user-token"),
            ("4", "40", "message-token"),
            ("5", "50", "autocomplete-token"),
            ("6", "60", "component-token"),
            ("7", "70", "modal-token"),
            ("8", "80", "unknown-token"),
        ];

        for (interaction, (id, application_id, token)) in interactions.iter().zip(expected) {
            assert_eq!(interaction.context().id.as_str(), id);
            assert_eq!(interaction.id().as_str(), id);
            assert_eq!(interaction.application_id().as_str(), application_id);
            assert_eq!(interaction.token(), token);
        }
    }

    #[test]
    fn discord_model_trait_returns_ids_for_core_models() {
        let guild = Guild {
            id: Snowflake::from("11"),
            name: "Guild".to_string(),
            ..Guild::default()
        };
        let channel = Channel {
            id: Snowflake::from("12"),
            kind: 0,
            ..Channel::default()
        };
        let message = Message {
            id: Snowflake::from("13"),
            channel_id: Snowflake::from("99"),
            ..Message::default()
        };
        let role = Role {
            id: Snowflake::from("14"),
            name: "Admin".to_string(),
            ..Role::default()
        };
        let attachment = Attachment {
            id: Snowflake::from("15"),
            filename: "file.txt".to_string(),
            ..Attachment::default()
        };

        assert_eq!(DiscordModel::id(&guild).as_str(), "11");
        assert_eq!(DiscordModel::id(&channel).as_str(), "12");
        assert_eq!(DiscordModel::id(&message).as_str(), "13");
        assert_eq!(DiscordModel::id(&role).as_str(), "14");
        assert_eq!(DiscordModel::id(&attachment).as_str(), "15");
    }

    #[test]
    fn serde_defaults_fill_missing_fields_for_core_models() {
        let member: Member = serde_json::from_value(json!({})).unwrap();
        let message: Message = serde_json::from_value(json!({
            "id": "55",
            "channel_id": "66"
        }))
        .unwrap();
        let reaction: Reaction = serde_json::from_value(json!({})).unwrap();
        let component: ComponentInteractionData = serde_json::from_value(json!({
            "custom_id": "menu",
            "component_type": 3
        }))
        .unwrap();
        let thread_metadata: ThreadMetadata = serde_json::from_value(json!({})).unwrap();

        assert!(member.roles.is_empty());
        assert_eq!(message.content, "");
        assert!(message.mentions.is_empty());
        assert!(message.attachments.is_empty());
        assert!(message.embeds.is_empty());
        assert!(message.reactions.is_empty());
        assert_eq!(reaction.count, 0);
        assert!(!reaction.me);
        assert!(reaction.emoji.is_none());
        assert!(component.values.is_empty());
        assert!(!thread_metadata.archived);
        assert!(!thread_metadata.locked);
        assert!(thread_metadata.auto_archive_duration.is_none());
    }

    #[test]
    fn simple_payload_models_keep_wire_aliases_and_omit_absent_optionals() {
        let callback = InteractionCallbackResponse {
            kind: 4,
            ..InteractionCallbackResponse::default()
        };
        let dm_channel = CreateDmChannel {
            recipient_id: Snowflake::from("321"),
        };
        let sticker = StickerItem {
            id: Snowflake::from("654"),
            name: "party".to_string(),
            kind: Some(1),
        };
        let gateway = GatewayBot {
            url: "wss://gateway.discord.gg".to_string(),
            shards: 2,
            session_start_limit: SessionStartLimit {
                total: 1000,
                remaining: 999,
                reset_after: 60_000,
                max_concurrency: 1,
            },
        };

        assert_eq!(
            serde_json::to_value(&callback).unwrap(),
            json!({ "type": 4 })
        );
        assert_eq!(
            serde_json::to_value(&dm_channel).unwrap(),
            json!({ "recipient_id": "321" })
        );
        assert_eq!(
            serde_json::to_value(&sticker).unwrap(),
            json!({ "id": "654", "name": "party", "format_type": 1 })
        );
        assert_eq!(
            serde_json::to_value(&gateway).unwrap()["session_start_limit"]["remaining"],
            999
        );
    }

    #[test]
    fn scheduled_event_recurrence_and_reaction_emoji_are_typed() {
        let event: GuildScheduledEvent = serde_json::from_value(json!({
            "id": "1",
            "guild_id": "2",
            "name": "standup",
            "scheduled_start_time": "2026-04-29T00:00:00.000000+00:00",
            "privacy_level": 2,
            "status": 1,
            "entity_type": 3,
            "entity_metadata": { "location": "voice" },
            "recurrence_rule": {
                "start": "2026-04-29T00:00:00.000000+00:00",
                "frequency": 2,
                "interval": 1,
                "by_weekday": [1],
                "by_n_weekday": [{ "n": 1, "day": 1 }]
            }
        }))
        .unwrap();
        let reaction: Reaction = serde_json::from_value(json!({
            "count": 2,
            "me": true,
            "emoji": { "id": "10", "name": "party", "animated": true }
        }))
        .unwrap();

        assert_eq!(
            event
                .entity_metadata
                .as_ref()
                .and_then(|metadata| metadata.location.as_deref()),
            Some("voice")
        );
        assert_eq!(
            event
                .recurrence_rule
                .as_ref()
                .and_then(|rule| rule.by_n_weekday.as_ref())
                .and_then(|weekdays| weekdays.first())
                .map(|weekday| weekday.day),
            Some(1)
        );
        assert_eq!(
            reaction
                .emoji
                .as_ref()
                .and_then(|emoji| emoji.name.as_deref()),
            Some("party")
        );
    }

    #[test]
    fn embed_presence_and_permissions_cover_optional_and_numeric_serde_paths() {
        let embed = Embed {
            title: Some("Docs".to_string()),
            ..Embed::default()
        };
        let presence = Presence {
            user_id: Some(Snowflake::from("777")),
            ..Presence::default()
        };
        let numeric_permissions: PermissionsBitField = serde_json::from_value(json!(16)).unwrap();
        let invalid_timestamp = Snowflake::new("not-a-number");

        let embed_json = serde_json::to_value(&embed).unwrap();
        let presence_json = serde_json::to_value(&presence).unwrap();

        assert_eq!(embed_json["title"], json!("Docs"));
        assert_eq!(embed_json["fields"], json!([]));
        assert!(embed_json.get("description").is_none());
        assert_eq!(presence_json, json!({ "user_id": "777" }));
        assert_eq!(numeric_permissions.bits(), 16);
        assert_eq!(invalid_timestamp.timestamp(), None);
    }
}
