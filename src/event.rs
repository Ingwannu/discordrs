use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::error::DiscordError;
use crate::model::{
    Channel, Guild, Interaction, Member, Message, Role, Snowflake, User, VoiceServerUpdate,
    VoiceState,
};
use crate::parsers::parse_interaction;
use crate::types::Emoji;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ReadyApplication {
    pub id: Snowflake,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ReadyPayload {
    pub user: User,
    pub session_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub application: Option<ReadyApplication>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resume_gateway_url: Option<String>,
}

#[derive(Clone, Debug)]
pub struct ReadyEvent {
    pub data: ReadyPayload,
    pub raw: Value,
}

#[derive(Clone, Debug)]
pub struct GuildEvent {
    pub guild: Guild,
    pub raw: Value,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GuildDeletePayload {
    pub id: Snowflake,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unavailable: Option<bool>,
}

#[derive(Clone, Debug)]
pub struct GuildDeleteEvent {
    pub data: GuildDeletePayload,
    pub raw: Value,
}

#[derive(Clone, Debug)]
pub struct ChannelEvent {
    pub channel: Channel,
    pub raw: Value,
}

#[derive(Clone, Debug)]
pub struct MemberEvent {
    pub guild_id: Snowflake,
    pub member: Member,
    pub raw: Value,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MemberRemovePayload {
    pub guild_id: Snowflake,
    pub user: User,
}

#[derive(Clone, Debug)]
pub struct MemberRemoveEvent {
    pub data: MemberRemovePayload,
    pub raw: Value,
}

#[derive(Clone, Debug)]
pub struct RoleEvent {
    pub guild_id: Snowflake,
    pub role: Role,
    pub raw: Value,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RoleDeletePayload {
    pub guild_id: Snowflake,
    pub role_id: Snowflake,
}

#[derive(Clone, Debug)]
pub struct RoleDeleteEvent {
    pub data: RoleDeletePayload,
    pub raw: Value,
}

#[derive(Clone, Debug)]
pub struct MessageEvent {
    pub message: Message,
    pub raw: Value,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MessageDeletePayload {
    pub id: Snowflake,
    pub channel_id: Snowflake,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub guild_id: Option<Snowflake>,
}

#[derive(Clone, Debug)]
pub struct MessageDeleteEvent {
    pub data: MessageDeletePayload,
    pub raw: Value,
}

#[derive(Clone, Debug)]
pub struct InteractionEvent {
    pub interaction: Interaction,
    pub raw: Value,
}

#[derive(Clone, Debug)]
pub struct VoiceStateEvent {
    pub state: VoiceState,
    pub raw: Value,
}

#[derive(Clone, Debug)]
pub struct VoiceServerEvent {
    pub data: VoiceServerUpdate,
    pub raw: Value,
}

#[derive(Clone, Debug)]
pub struct BulkMessageDeleteEvent {
    pub ids: Vec<Snowflake>,
    pub channel_id: Snowflake,
    pub guild_id: Option<Snowflake>,
    pub raw: Value,
}

#[derive(Clone, Debug)]
pub struct ChannelPinsUpdateEvent {
    pub channel_id: Snowflake,
    pub guild_id: Option<Snowflake>,
    pub last_pin_timestamp: Option<String>,
    pub raw: Value,
}

#[derive(Clone, Debug)]
pub struct GuildBanEvent {
    pub guild_id: Snowflake,
    pub user: User,
    pub raw: Value,
}

#[derive(Clone, Debug)]
pub struct GuildEmojisUpdateEvent {
    pub guild_id: Snowflake,
    pub emojis: Vec<Emoji>,
    pub raw: Value,
}

#[derive(Clone, Debug)]
pub struct InviteEvent {
    pub guild_id: Option<Snowflake>,
    pub channel_id: Option<Snowflake>,
    pub code: Option<String>,
    pub raw: Value,
}

#[derive(Clone, Debug)]
pub struct ReactionEvent {
    pub user_id: Option<Snowflake>,
    pub channel_id: Option<Snowflake>,
    pub message_id: Option<Snowflake>,
    pub guild_id: Option<Snowflake>,
    pub emoji: Option<Emoji>,
    pub raw: Value,
}

#[derive(Clone, Debug)]
pub struct ReactionRemoveAllEvent {
    pub channel_id: Option<Snowflake>,
    pub message_id: Option<Snowflake>,
    pub guild_id: Option<Snowflake>,
    pub raw: Value,
}

#[derive(Clone, Debug)]
pub struct TypingStartEvent {
    pub channel_id: Option<Snowflake>,
    pub guild_id: Option<Snowflake>,
    pub user_id: Option<Snowflake>,
    pub timestamp: Option<u64>,
    pub raw: Value,
}

#[derive(Clone, Debug)]
pub struct PresenceUpdateEvent {
    pub user_id: Option<Snowflake>,
    pub guild_id: Option<Snowflake>,
    pub status: Option<String>,
    pub raw: Value,
}

#[derive(Clone, Debug)]
pub struct WebhooksUpdateEvent {
    pub guild_id: Option<Snowflake>,
    pub channel_id: Option<Snowflake>,
    pub raw: Value,
}

#[derive(Clone, Debug)]
pub struct GuildIntegrationsUpdateEvent {
    pub guild_id: Option<Snowflake>,
    pub raw: Value,
}

#[derive(Clone, Debug)]
pub enum Event {
    Ready(ReadyEvent),
    GuildCreate(GuildEvent),
    GuildUpdate(GuildEvent),
    GuildDelete(GuildDeleteEvent),
    ChannelCreate(ChannelEvent),
    ChannelUpdate(ChannelEvent),
    ChannelDelete(ChannelEvent),
    MemberAdd(MemberEvent),
    MemberUpdate(MemberEvent),
    MemberRemove(MemberRemoveEvent),
    RoleCreate(RoleEvent),
    RoleUpdate(RoleEvent),
    RoleDelete(RoleDeleteEvent),
    MessageCreate(MessageEvent),
    MessageUpdate(MessageEvent),
    MessageDelete(MessageDeleteEvent),
    MessageDeleteBulk(BulkMessageDeleteEvent),
    ChannelPinsUpdate(ChannelPinsUpdateEvent),
    GuildBanAdd(GuildBanEvent),
    GuildBanRemove(GuildBanEvent),
    GuildEmojisUpdate(GuildEmojisUpdateEvent),
    GuildIntegrationsUpdate(GuildIntegrationsUpdateEvent),
    WebhooksUpdate(WebhooksUpdateEvent),
    InviteCreate(InviteEvent),
    InviteDelete(InviteEvent),
    MessageReactionAdd(ReactionEvent),
    MessageReactionRemove(ReactionEvent),
    MessageReactionRemoveAll(ReactionRemoveAllEvent),
    TypingStart(TypingStartEvent),
    PresenceUpdate(PresenceUpdateEvent),
    InteractionCreate(InteractionEvent),
    VoiceStateUpdate(VoiceStateEvent),
    VoiceServerUpdate(VoiceServerEvent),
    Unknown { kind: String, raw: Value },
}

impl Event {
    pub fn kind(&self) -> &str {
        match self {
            Event::Ready(_) => "READY",
            Event::GuildCreate(_) => "GUILD_CREATE",
            Event::GuildUpdate(_) => "GUILD_UPDATE",
            Event::GuildDelete(_) => "GUILD_DELETE",
            Event::ChannelCreate(_) => "CHANNEL_CREATE",
            Event::ChannelUpdate(_) => "CHANNEL_UPDATE",
            Event::ChannelDelete(_) => "CHANNEL_DELETE",
            Event::MemberAdd(_) => "GUILD_MEMBER_ADD",
            Event::MemberUpdate(_) => "GUILD_MEMBER_UPDATE",
            Event::MemberRemove(_) => "GUILD_MEMBER_REMOVE",
            Event::RoleCreate(_) => "GUILD_ROLE_CREATE",
            Event::RoleUpdate(_) => "GUILD_ROLE_UPDATE",
            Event::RoleDelete(_) => "GUILD_ROLE_DELETE",
            Event::MessageCreate(_) => "MESSAGE_CREATE",
            Event::MessageUpdate(_) => "MESSAGE_UPDATE",
            Event::MessageDelete(_) => "MESSAGE_DELETE",
            Event::MessageDeleteBulk(_) => "MESSAGE_DELETE_BULK",
            Event::ChannelPinsUpdate(_) => "CHANNEL_PINS_UPDATE",
            Event::GuildBanAdd(_) => "GUILD_BAN_ADD",
            Event::GuildBanRemove(_) => "GUILD_BAN_REMOVE",
            Event::GuildEmojisUpdate(_) => "GUILD_EMOJIS_UPDATE",
            Event::GuildIntegrationsUpdate(_) => "GUILD_INTEGRATIONS_UPDATE",
            Event::WebhooksUpdate(_) => "WEBHOOKS_UPDATE",
            Event::InviteCreate(_) => "INVITE_CREATE",
            Event::InviteDelete(_) => "INVITE_DELETE",
            Event::MessageReactionAdd(_) => "MESSAGE_REACTION_ADD",
            Event::MessageReactionRemove(_) => "MESSAGE_REACTION_REMOVE",
            Event::MessageReactionRemoveAll(_) => "MESSAGE_REACTION_REMOVE_ALL",
            Event::TypingStart(_) => "TYPING_START",
            Event::PresenceUpdate(_) => "PRESENCE_UPDATE",
            Event::InteractionCreate(_) => "INTERACTION_CREATE",
            Event::VoiceStateUpdate(_) => "VOICE_STATE_UPDATE",
            Event::VoiceServerUpdate(_) => "VOICE_SERVER_UPDATE",
            Event::Unknown { kind, .. } => kind.as_str(),
        }
    }

    pub fn raw(&self) -> &Value {
        match self {
            Event::Ready(event) => &event.raw,
            Event::GuildCreate(event) | Event::GuildUpdate(event) => &event.raw,
            Event::GuildDelete(event) => &event.raw,
            Event::ChannelCreate(event)
            | Event::ChannelUpdate(event)
            | Event::ChannelDelete(event) => &event.raw,
            Event::MemberAdd(event) | Event::MemberUpdate(event) => &event.raw,
            Event::MemberRemove(event) => &event.raw,
            Event::RoleCreate(event) | Event::RoleUpdate(event) => &event.raw,
            Event::RoleDelete(event) => &event.raw,
            Event::MessageCreate(event) | Event::MessageUpdate(event) => &event.raw,
            Event::MessageDelete(event) => &event.raw,
            Event::MessageDeleteBulk(event) => &event.raw,
            Event::ChannelPinsUpdate(event) => &event.raw,
            Event::GuildBanAdd(event) | Event::GuildBanRemove(event) => &event.raw,
            Event::GuildEmojisUpdate(event) => &event.raw,
            Event::GuildIntegrationsUpdate(event) => &event.raw,
            Event::WebhooksUpdate(event) => &event.raw,
            Event::InviteCreate(event) | Event::InviteDelete(event) => &event.raw,
            Event::MessageReactionAdd(event) | Event::MessageReactionRemove(event) => &event.raw,
            Event::MessageReactionRemoveAll(event) => &event.raw,
            Event::TypingStart(event) => &event.raw,
            Event::PresenceUpdate(event) => &event.raw,
            Event::InteractionCreate(event) => &event.raw,
            Event::VoiceStateUpdate(event) => &event.raw,
            Event::VoiceServerUpdate(event) => &event.raw,
            Event::Unknown { raw, .. } => raw,
        }
    }
}

pub fn decode_event(event_name: &str, data: Value) -> Result<Event, DiscordError> {
    let event = match event_name {
        "READY" => Event::Ready(ReadyEvent {
            data: serde_json::from_value(data.clone())?,
            raw: data,
        }),
        "GUILD_CREATE" => Event::GuildCreate(GuildEvent {
            guild: serde_json::from_value(data.clone())?,
            raw: data,
        }),
        "GUILD_UPDATE" => Event::GuildUpdate(GuildEvent {
            guild: serde_json::from_value(data.clone())?,
            raw: data,
        }),
        "GUILD_DELETE" => Event::GuildDelete(GuildDeleteEvent {
            data: serde_json::from_value(data.clone())?,
            raw: data,
        }),
        "CHANNEL_CREATE" => Event::ChannelCreate(ChannelEvent {
            channel: serde_json::from_value(data.clone())?,
            raw: data,
        }),
        "CHANNEL_UPDATE" => Event::ChannelUpdate(ChannelEvent {
            channel: serde_json::from_value(data.clone())?,
            raw: data,
        }),
        "CHANNEL_DELETE" => Event::ChannelDelete(ChannelEvent {
            channel: serde_json::from_value(data.clone())?,
            raw: data,
        }),
        "GUILD_MEMBER_ADD" => Event::MemberAdd(MemberEvent {
            guild_id: read_required_snowflake(&data, "guild_id")?,
            member: serde_json::from_value(data.clone())?,
            raw: data,
        }),
        "GUILD_MEMBER_UPDATE" => Event::MemberUpdate(MemberEvent {
            guild_id: read_required_snowflake(&data, "guild_id")?,
            member: serde_json::from_value(data.clone())?,
            raw: data,
        }),
        "GUILD_MEMBER_REMOVE" => Event::MemberRemove(MemberRemoveEvent {
            data: serde_json::from_value(data.clone())?,
            raw: data,
        }),
        "GUILD_ROLE_CREATE" => Event::RoleCreate(RoleEvent {
            guild_id: read_required_snowflake(&data, "guild_id")?,
            role: serde_json::from_value(data.get("role").cloned().unwrap_or(Value::Null))?,
            raw: data,
        }),
        "GUILD_ROLE_UPDATE" => Event::RoleUpdate(RoleEvent {
            guild_id: read_required_snowflake(&data, "guild_id")?,
            role: serde_json::from_value(data.get("role").cloned().unwrap_or(Value::Null))?,
            raw: data,
        }),
        "GUILD_ROLE_DELETE" => Event::RoleDelete(RoleDeleteEvent {
            data: serde_json::from_value(data.clone())?,
            raw: data,
        }),
        "MESSAGE_CREATE" => Event::MessageCreate(MessageEvent {
            message: serde_json::from_value(data.clone())?,
            raw: data,
        }),
        "MESSAGE_UPDATE" => Event::MessageUpdate(MessageEvent {
            message: serde_json::from_value(data.clone())?,
            raw: data,
        }),
        "MESSAGE_DELETE" => Event::MessageDelete(MessageDeleteEvent {
            data: serde_json::from_value(data.clone())?,
            raw: data,
        }),
        "MESSAGE_DELETE_BULK" => {
            let ids: Vec<Snowflake> =
                serde_json::from_value(data.get("ids").cloned().unwrap_or(Value::Null))?;
            Event::MessageDeleteBulk(BulkMessageDeleteEvent {
                channel_id: read_required_snowflake(&data, "channel_id")?,
                guild_id: data
                    .get("guild_id")
                    .and_then(|v| serde_json::from_value(v.clone()).ok()),
                ids,
                raw: data,
            })
        }
        "CHANNEL_PINS_UPDATE" => Event::ChannelPinsUpdate(ChannelPinsUpdateEvent {
            channel_id: read_required_snowflake(&data, "channel_id")?,
            guild_id: data
                .get("guild_id")
                .and_then(|v| serde_json::from_value(v.clone()).ok()),
            last_pin_timestamp: data
                .get("last_pin_timestamp")
                .and_then(|v| v.as_str().map(String::from)),
            raw: data,
        }),
        "GUILD_BAN_ADD" => Event::GuildBanAdd(GuildBanEvent {
            guild_id: read_required_snowflake(&data, "guild_id")?,
            user: serde_json::from_value(data.get("user").cloned().unwrap_or(Value::Null))?,
            raw: data,
        }),
        "GUILD_BAN_REMOVE" => Event::GuildBanRemove(GuildBanEvent {
            guild_id: read_required_snowflake(&data, "guild_id")?,
            user: serde_json::from_value(data.get("user").cloned().unwrap_or(Value::Null))?,
            raw: data,
        }),
        "GUILD_EMOJIS_UPDATE" => Event::GuildEmojisUpdate(GuildEmojisUpdateEvent {
            guild_id: read_required_snowflake(&data, "guild_id")?,
            emojis: serde_json::from_value(data.get("emojis").cloned().unwrap_or(Value::Null))
                .unwrap_or_default(),
            raw: data,
        }),
        "GUILD_INTEGRATIONS_UPDATE" => {
            Event::GuildIntegrationsUpdate(GuildIntegrationsUpdateEvent {
                guild_id: data
                    .get("guild_id")
                    .and_then(|v| serde_json::from_value(v.clone()).ok()),
                raw: data,
            })
        }
        "WEBHOOKS_UPDATE" => Event::WebhooksUpdate(WebhooksUpdateEvent {
            guild_id: data
                .get("guild_id")
                .and_then(|v| serde_json::from_value(v.clone()).ok()),
            channel_id: data
                .get("channel_id")
                .and_then(|v| serde_json::from_value(v.clone()).ok()),
            raw: data,
        }),
        "INVITE_CREATE" => Event::InviteCreate(InviteEvent {
            guild_id: data
                .get("guild_id")
                .and_then(|v| serde_json::from_value(v.clone()).ok()),
            channel_id: data
                .get("channel_id")
                .and_then(|v| serde_json::from_value(v.clone()).ok()),
            code: data.get("code").and_then(|v| v.as_str().map(String::from)),
            raw: data,
        }),
        "INVITE_DELETE" => Event::InviteDelete(InviteEvent {
            guild_id: data
                .get("guild_id")
                .and_then(|v| serde_json::from_value(v.clone()).ok()),
            channel_id: data
                .get("channel_id")
                .and_then(|v| serde_json::from_value(v.clone()).ok()),
            code: data.get("code").and_then(|v| v.as_str().map(String::from)),
            raw: data,
        }),
        "MESSAGE_REACTION_ADD" => Event::MessageReactionAdd(ReactionEvent {
            user_id: data
                .get("user_id")
                .and_then(|v| serde_json::from_value(v.clone()).ok()),
            channel_id: data
                .get("channel_id")
                .and_then(|v| serde_json::from_value(v.clone()).ok()),
            message_id: data
                .get("message_id")
                .and_then(|v| serde_json::from_value(v.clone()).ok()),
            guild_id: data
                .get("guild_id")
                .and_then(|v| serde_json::from_value(v.clone()).ok()),
            emoji: data
                .get("emoji")
                .and_then(|v| serde_json::from_value(v.clone()).ok()),
            raw: data,
        }),
        "MESSAGE_REACTION_REMOVE" => Event::MessageReactionRemove(ReactionEvent {
            user_id: data
                .get("user_id")
                .and_then(|v| serde_json::from_value(v.clone()).ok()),
            channel_id: data
                .get("channel_id")
                .and_then(|v| serde_json::from_value(v.clone()).ok()),
            message_id: data
                .get("message_id")
                .and_then(|v| serde_json::from_value(v.clone()).ok()),
            guild_id: data
                .get("guild_id")
                .and_then(|v| serde_json::from_value(v.clone()).ok()),
            emoji: data
                .get("emoji")
                .and_then(|v| serde_json::from_value(v.clone()).ok()),
            raw: data,
        }),
        "MESSAGE_REACTION_REMOVE_ALL" => Event::MessageReactionRemoveAll(ReactionRemoveAllEvent {
            channel_id: data
                .get("channel_id")
                .and_then(|v| serde_json::from_value(v.clone()).ok()),
            message_id: data
                .get("message_id")
                .and_then(|v| serde_json::from_value(v.clone()).ok()),
            guild_id: data
                .get("guild_id")
                .and_then(|v| serde_json::from_value(v.clone()).ok()),
            raw: data,
        }),
        "TYPING_START" => Event::TypingStart(TypingStartEvent {
            channel_id: data
                .get("channel_id")
                .and_then(|v| serde_json::from_value(v.clone()).ok()),
            guild_id: data
                .get("guild_id")
                .and_then(|v| serde_json::from_value(v.clone()).ok()),
            user_id: data
                .get("user_id")
                .and_then(|v| serde_json::from_value(v.clone()).ok()),
            timestamp: data.get("timestamp").and_then(|v| v.as_u64()),
            raw: data,
        }),
        "PRESENCE_UPDATE" => Event::PresenceUpdate(PresenceUpdateEvent {
            user_id: data
                .pointer("/user/id")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
                .map(Snowflake::new),
            guild_id: data
                .get("guild_id")
                .and_then(|v| serde_json::from_value(v.clone()).ok()),
            status: data
                .get("status")
                .and_then(|v| v.as_str().map(String::from)),
            raw: data,
        }),
        "INTERACTION_CREATE" => Event::InteractionCreate(InteractionEvent {
            interaction: parse_interaction(&data)?,
            raw: data,
        }),
        "VOICE_STATE_UPDATE" => Event::VoiceStateUpdate(VoiceStateEvent {
            state: serde_json::from_value(data.clone())?,
            raw: data,
        }),
        "VOICE_SERVER_UPDATE" => Event::VoiceServerUpdate(VoiceServerEvent {
            data: serde_json::from_value(data.clone())?,
            raw: data,
        }),
        _ => Event::Unknown {
            kind: event_name.to_string(),
            raw: data,
        },
    };

    Ok(event)
}

fn read_required_snowflake(value: &Value, field: &str) -> Result<Snowflake, DiscordError> {
    let Some(raw) = value.get(field) else {
        return Err(format!("missing field {field}").into());
    };

    serde_json::from_value(raw.clone()).map_err(Into::into)
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{decode_event, Event};

    #[test]
    fn decode_message_create_event_returns_typed_payload() {
        let event = decode_event(
            "MESSAGE_CREATE",
            json!({
                "id": "2",
                "channel_id": "1",
                "content": "hello",
                "mentions": [],
                "attachments": []
            }),
        )
        .unwrap();

        match event {
            Event::MessageCreate(message) => assert_eq!(message.message.content, "hello"),
            other => panic!("unexpected event: {other:?}"),
        }
    }
}
