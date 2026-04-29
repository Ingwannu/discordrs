use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::error::DiscordError;
use crate::model::{
    AuditLogEntry, Channel, Entitlement, Guild, Integration, Interaction, Member, Message,
    Presence, Role, Snowflake, SoundboardSound, StageInstance, Sticker, Subscription, User,
    VoiceServerUpdate, VoiceState,
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

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GuildMembersChunkPayload {
    pub guild_id: Snowflake,
    #[serde(default)]
    pub members: Vec<Member>,
    pub chunk_index: u64,
    pub chunk_count: u64,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub not_found: Vec<Snowflake>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub presences: Option<Vec<Presence>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nonce: Option<String>,
}

#[derive(Clone, Debug)]
pub struct GuildMembersChunkEvent {
    pub data: GuildMembersChunkPayload,
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
pub struct ResumedEvent {
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
pub struct EntitlementEvent {
    pub entitlement: Entitlement,
    pub raw: Value,
}

#[derive(Clone, Debug)]
pub struct SubscriptionEvent {
    pub subscription: Subscription,
    pub raw: Value,
}

#[derive(Clone, Debug)]
pub struct IntegrationEvent {
    pub integration: Integration,
    pub guild_id: Option<Snowflake>,
    pub raw: Value,
}

#[derive(Clone, Debug)]
pub struct IntegrationDeleteEvent {
    pub id: Option<Snowflake>,
    pub guild_id: Option<Snowflake>,
    pub application_id: Option<Snowflake>,
    pub raw: Value,
}

#[derive(Clone, Debug)]
pub struct PollVoteEvent {
    pub user_id: Option<Snowflake>,
    pub channel_id: Option<Snowflake>,
    pub message_id: Option<Snowflake>,
    pub guild_id: Option<Snowflake>,
    pub answer_id: Option<u64>,
    pub raw: Value,
}

#[derive(Clone, Debug)]
pub struct SoundboardSoundEvent {
    pub sound: SoundboardSound,
    pub raw: Value,
}

#[derive(Clone, Debug)]
pub struct SoundboardSoundDeleteEvent {
    pub sound_id: Snowflake,
    pub guild_id: Snowflake,
    pub raw: Value,
}

#[derive(Clone, Debug)]
pub struct SoundboardSoundsEvent {
    pub guild_id: Snowflake,
    pub soundboard_sounds: Vec<SoundboardSound>,
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
pub struct UserUpdateEvent {
    pub user: User,
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
pub struct ThreadEvent {
    pub thread: Channel,
    pub raw: Value,
}

#[derive(Clone, Debug)]
pub struct ThreadMemberUpdateEvent {
    pub guild_id: Option<Snowflake>,
    pub thread_id: Option<Snowflake>,
    pub raw: Value,
}

#[derive(Clone, Debug)]
pub struct ThreadMembersUpdateEvent {
    pub guild_id: Option<Snowflake>,
    pub thread_id: Option<Snowflake>,
    pub added_members: Option<Vec<serde_json::Value>>,
    pub removed_member_ids: Option<Vec<Snowflake>>,
    pub member_count: Option<u64>,
    pub raw: Value,
}

#[derive(Clone, Debug)]
pub struct ThreadListSyncEvent {
    pub guild_id: Option<Snowflake>,
    pub threads: Vec<Channel>,
    pub raw: Value,
}

#[derive(Clone, Debug)]
pub struct ReactionRemoveEmojiEvent {
    pub channel_id: Option<Snowflake>,
    pub message_id: Option<Snowflake>,
    pub guild_id: Option<Snowflake>,
    pub emoji: Option<Emoji>,
    pub raw: Value,
}

#[derive(Clone, Debug)]
pub struct GuildStickersUpdateEvent {
    pub guild_id: Option<Snowflake>,
    pub stickers: Vec<Sticker>,
    pub raw: Value,
}

#[derive(Clone, Debug, Default)]
pub struct ScheduledEvent {
    pub id: Option<Snowflake>,
    pub guild_id: Option<Snowflake>,
    pub channel_id: Option<Snowflake>,
    pub creator_id: Option<Snowflake>,
    pub name: Option<String>,
    pub description: Option<String>,
    pub scheduled_start_time: Option<String>,
    pub scheduled_end_time: Option<String>,
    pub privacy_level: Option<u64>,
    pub status: Option<u64>,
    pub entity_type: Option<u64>,
    pub entity_id: Option<Snowflake>,
    pub entity_metadata: Option<Value>,
    pub user_count: Option<u64>,
    pub image: Option<String>,
    pub raw: Value,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GuildScheduledEventUserEvent {
    pub guild_scheduled_event_id: Snowflake,
    pub user_id: Snowflake,
    pub guild_id: Snowflake,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub member: Option<Member>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user: Option<User>,
    #[serde(skip)]
    pub raw: Value,
}

#[derive(Clone, Debug)]
pub struct StageInstanceEvent {
    pub stage_instance: StageInstance,
    pub raw: Value,
}

#[derive(Clone, Debug)]
pub struct ApplicationCommandPermissionsUpdateEvent {
    pub id: Option<Snowflake>,
    pub application_id: Option<Snowflake>,
    pub guild_id: Option<Snowflake>,
    pub permissions: Vec<Value>,
    pub raw: Value,
}

#[derive(Clone, Debug)]
pub struct VoiceChannelEffectEvent {
    pub channel_id: Option<Snowflake>,
    pub guild_id: Option<Snowflake>,
    pub user_id: Option<Snowflake>,
    pub emoji: Option<Emoji>,
    pub animation_type: Option<u64>,
    pub animation_id: Option<u64>,
    pub sound_id: Option<Snowflake>,
    pub sound_volume: Option<f64>,
    pub raw: Value,
}

#[derive(Clone, Debug)]
pub struct VoiceChannelStartTimeUpdateEvent {
    pub channel_id: Option<Snowflake>,
    pub guild_id: Option<Snowflake>,
    pub voice_channel_start_time: Option<String>,
    pub raw: Value,
}

#[derive(Clone, Debug)]
pub struct VoiceChannelStatusUpdateEvent {
    pub channel_id: Option<Snowflake>,
    pub guild_id: Option<Snowflake>,
    pub status: Option<String>,
    pub raw: Value,
}

#[derive(Clone, Debug)]
pub struct AutoModerationEvent {
    pub id: Option<Snowflake>,
    pub guild_id: Option<Snowflake>,
    pub name: Option<String>,
    pub creator_id: Option<Snowflake>,
    pub event_type: Option<u64>,
    pub trigger_type: Option<u64>,
    pub trigger_metadata: Option<Value>,
    pub actions: Vec<Value>,
    pub enabled: Option<bool>,
    pub exempt_roles: Vec<Snowflake>,
    pub exempt_channels: Vec<Snowflake>,
    pub action: Option<Value>,
    pub rule_id: Option<Snowflake>,
    pub rule_trigger_type: Option<u64>,
    pub user_id: Option<Snowflake>,
    pub channel_id: Option<Snowflake>,
    pub message_id: Option<Snowflake>,
    pub alert_system_message_id: Option<Snowflake>,
    pub content: Option<String>,
    pub matched_keyword: Option<String>,
    pub matched_content: Option<String>,
    pub raw: Value,
}

#[derive(Clone, Debug)]
pub struct AuditLogEntryEvent {
    pub guild_id: Option<Snowflake>,
    pub entry: Option<AuditLogEntry>,
    pub id: Option<Snowflake>,
    pub user_id: Option<Snowflake>,
    pub target_id: Option<Snowflake>,
    pub action_type: Option<u64>,
    pub changes: Option<Vec<Value>>,
    pub options: Option<Value>,
    pub reason: Option<String>,
    pub raw: Value,
}

#[derive(Clone, Debug)]
#[non_exhaustive]
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
    GuildMembersChunk(GuildMembersChunkEvent),
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
    EntitlementCreate(EntitlementEvent),
    EntitlementUpdate(EntitlementEvent),
    EntitlementDelete(EntitlementEvent),
    SubscriptionCreate(SubscriptionEvent),
    SubscriptionUpdate(SubscriptionEvent),
    SubscriptionDelete(SubscriptionEvent),
    IntegrationCreate(IntegrationEvent),
    IntegrationUpdate(IntegrationEvent),
    IntegrationDelete(IntegrationDeleteEvent),
    GuildSoundboardSoundCreate(SoundboardSoundEvent),
    GuildSoundboardSoundUpdate(SoundboardSoundEvent),
    GuildSoundboardSoundDelete(SoundboardSoundDeleteEvent),
    GuildSoundboardSoundsUpdate(SoundboardSoundsEvent),
    SoundboardSounds(SoundboardSoundsEvent),
    WebhooksUpdate(WebhooksUpdateEvent),
    InviteCreate(InviteEvent),
    InviteDelete(InviteEvent),
    MessageReactionAdd(ReactionEvent),
    MessageReactionRemove(ReactionEvent),
    MessageReactionRemoveAll(ReactionRemoveAllEvent),
    TypingStart(TypingStartEvent),
    PresenceUpdate(PresenceUpdateEvent),
    UserUpdate(UserUpdateEvent),
    InteractionCreate(InteractionEvent),
    VoiceStateUpdate(VoiceStateEvent),
    VoiceServerUpdate(VoiceServerEvent),
    Resumed(ResumedEvent),
    ThreadCreate(ThreadEvent),
    ThreadUpdate(ThreadEvent),
    ThreadDelete(ThreadEvent),
    ThreadListSync(ThreadListSyncEvent),
    ThreadMemberUpdate(ThreadMemberUpdateEvent),
    ThreadMembersUpdate(ThreadMembersUpdateEvent),
    MessageReactionRemoveEmoji(ReactionRemoveEmojiEvent),
    MessagePollVoteAdd(PollVoteEvent),
    MessagePollVoteRemove(PollVoteEvent),
    GuildStickersUpdate(GuildStickersUpdateEvent),
    GuildScheduledEventCreate(ScheduledEvent),
    GuildScheduledEventUpdate(ScheduledEvent),
    GuildScheduledEventDelete(ScheduledEvent),
    GuildScheduledEventUserAdd(GuildScheduledEventUserEvent),
    GuildScheduledEventUserRemove(GuildScheduledEventUserEvent),
    StageInstanceCreate(StageInstanceEvent),
    StageInstanceUpdate(StageInstanceEvent),
    StageInstanceDelete(StageInstanceEvent),
    VoiceChannelEffectSend(VoiceChannelEffectEvent),
    VoiceChannelStartTimeUpdate(VoiceChannelStartTimeUpdateEvent),
    VoiceChannelStatusUpdate(VoiceChannelStatusUpdateEvent),
    ApplicationCommandPermissionsUpdate(ApplicationCommandPermissionsUpdateEvent),
    AutoModerationRuleCreate(AutoModerationEvent),
    AutoModerationRuleUpdate(AutoModerationEvent),
    AutoModerationRuleDelete(AutoModerationEvent),
    AutoModerationActionExecution(AutoModerationEvent),
    GuildAuditLogEntryCreate(AuditLogEntryEvent),
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
            Event::GuildMembersChunk(_) => "GUILD_MEMBERS_CHUNK",
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
            Event::EntitlementCreate(_) => "ENTITLEMENT_CREATE",
            Event::EntitlementUpdate(_) => "ENTITLEMENT_UPDATE",
            Event::EntitlementDelete(_) => "ENTITLEMENT_DELETE",
            Event::SubscriptionCreate(_) => "SUBSCRIPTION_CREATE",
            Event::SubscriptionUpdate(_) => "SUBSCRIPTION_UPDATE",
            Event::SubscriptionDelete(_) => "SUBSCRIPTION_DELETE",
            Event::IntegrationCreate(_) => "INTEGRATION_CREATE",
            Event::IntegrationUpdate(_) => "INTEGRATION_UPDATE",
            Event::IntegrationDelete(_) => "INTEGRATION_DELETE",
            Event::GuildSoundboardSoundCreate(_) => "GUILD_SOUNDBOARD_SOUND_CREATE",
            Event::GuildSoundboardSoundUpdate(_) => "GUILD_SOUNDBOARD_SOUND_UPDATE",
            Event::GuildSoundboardSoundDelete(_) => "GUILD_SOUNDBOARD_SOUND_DELETE",
            Event::GuildSoundboardSoundsUpdate(_) => "GUILD_SOUNDBOARD_SOUNDS_UPDATE",
            Event::SoundboardSounds(_) => "SOUNDBOARD_SOUNDS",
            Event::WebhooksUpdate(_) => "WEBHOOKS_UPDATE",
            Event::InviteCreate(_) => "INVITE_CREATE",
            Event::InviteDelete(_) => "INVITE_DELETE",
            Event::MessageReactionAdd(_) => "MESSAGE_REACTION_ADD",
            Event::MessageReactionRemove(_) => "MESSAGE_REACTION_REMOVE",
            Event::MessageReactionRemoveAll(_) => "MESSAGE_REACTION_REMOVE_ALL",
            Event::TypingStart(_) => "TYPING_START",
            Event::PresenceUpdate(_) => "PRESENCE_UPDATE",
            Event::UserUpdate(_) => "USER_UPDATE",
            Event::InteractionCreate(_) => "INTERACTION_CREATE",
            Event::VoiceStateUpdate(_) => "VOICE_STATE_UPDATE",
            Event::VoiceServerUpdate(_) => "VOICE_SERVER_UPDATE",
            Event::Resumed(_) => "RESUMED",
            Event::ThreadCreate(_) => "THREAD_CREATE",
            Event::ThreadUpdate(_) => "THREAD_UPDATE",
            Event::ThreadDelete(_) => "THREAD_DELETE",
            Event::ThreadListSync(_) => "THREAD_LIST_SYNC",
            Event::ThreadMemberUpdate(_) => "THREAD_MEMBER_UPDATE",
            Event::ThreadMembersUpdate(_) => "THREAD_MEMBERS_UPDATE",
            Event::MessageReactionRemoveEmoji(_) => "MESSAGE_REACTION_REMOVE_EMOJI",
            Event::MessagePollVoteAdd(_) => "MESSAGE_POLL_VOTE_ADD",
            Event::MessagePollVoteRemove(_) => "MESSAGE_POLL_VOTE_REMOVE",
            Event::GuildStickersUpdate(_) => "GUILD_STICKERS_UPDATE",
            Event::GuildScheduledEventCreate(_) => "GUILD_SCHEDULED_EVENT_CREATE",
            Event::GuildScheduledEventUpdate(_) => "GUILD_SCHEDULED_EVENT_UPDATE",
            Event::GuildScheduledEventDelete(_) => "GUILD_SCHEDULED_EVENT_DELETE",
            Event::GuildScheduledEventUserAdd(_) => "GUILD_SCHEDULED_EVENT_USER_ADD",
            Event::GuildScheduledEventUserRemove(_) => "GUILD_SCHEDULED_EVENT_USER_REMOVE",
            Event::StageInstanceCreate(_) => "STAGE_INSTANCE_CREATE",
            Event::StageInstanceUpdate(_) => "STAGE_INSTANCE_UPDATE",
            Event::StageInstanceDelete(_) => "STAGE_INSTANCE_DELETE",
            Event::VoiceChannelEffectSend(_) => "VOICE_CHANNEL_EFFECT_SEND",
            Event::VoiceChannelStartTimeUpdate(_) => "VOICE_CHANNEL_START_TIME_UPDATE",
            Event::VoiceChannelStatusUpdate(_) => "VOICE_CHANNEL_STATUS_UPDATE",
            Event::ApplicationCommandPermissionsUpdate(_) => {
                "APPLICATION_COMMAND_PERMISSIONS_UPDATE"
            }
            Event::AutoModerationRuleCreate(_) => "AUTO_MODERATION_RULE_CREATE",
            Event::AutoModerationRuleUpdate(_) => "AUTO_MODERATION_RULE_UPDATE",
            Event::AutoModerationRuleDelete(_) => "AUTO_MODERATION_RULE_DELETE",
            Event::AutoModerationActionExecution(_) => "AUTO_MODERATION_ACTION_EXECUTION",
            Event::GuildAuditLogEntryCreate(_) => "GUILD_AUDIT_LOG_ENTRY_CREATE",
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
            Event::GuildMembersChunk(event) => &event.raw,
            Event::RoleCreate(event) | Event::RoleUpdate(event) => &event.raw,
            Event::RoleDelete(event) => &event.raw,
            Event::MessageCreate(event) | Event::MessageUpdate(event) => &event.raw,
            Event::MessageDelete(event) => &event.raw,
            Event::MessageDeleteBulk(event) => &event.raw,
            Event::ChannelPinsUpdate(event) => &event.raw,
            Event::GuildBanAdd(event) | Event::GuildBanRemove(event) => &event.raw,
            Event::GuildEmojisUpdate(event) => &event.raw,
            Event::GuildIntegrationsUpdate(event) => &event.raw,
            Event::EntitlementCreate(event)
            | Event::EntitlementUpdate(event)
            | Event::EntitlementDelete(event) => &event.raw,
            Event::SubscriptionCreate(event)
            | Event::SubscriptionUpdate(event)
            | Event::SubscriptionDelete(event) => &event.raw,
            Event::IntegrationCreate(event) | Event::IntegrationUpdate(event) => &event.raw,
            Event::IntegrationDelete(event) => &event.raw,
            Event::GuildSoundboardSoundCreate(event) | Event::GuildSoundboardSoundUpdate(event) => {
                &event.raw
            }
            Event::GuildSoundboardSoundDelete(event) => &event.raw,
            Event::GuildSoundboardSoundsUpdate(event) | Event::SoundboardSounds(event) => {
                &event.raw
            }
            Event::WebhooksUpdate(event) => &event.raw,
            Event::InviteCreate(event) | Event::InviteDelete(event) => &event.raw,
            Event::MessageReactionAdd(event) | Event::MessageReactionRemove(event) => &event.raw,
            Event::MessageReactionRemoveAll(event) => &event.raw,
            Event::TypingStart(event) => &event.raw,
            Event::PresenceUpdate(event) => &event.raw,
            Event::UserUpdate(event) => &event.raw,
            Event::InteractionCreate(event) => &event.raw,
            Event::VoiceStateUpdate(event) => &event.raw,
            Event::VoiceServerUpdate(event) => &event.raw,
            Event::Resumed(event) => &event.raw,
            Event::ThreadCreate(event)
            | Event::ThreadUpdate(event)
            | Event::ThreadDelete(event) => &event.raw,
            Event::ThreadListSync(event) => &event.raw,
            Event::ThreadMemberUpdate(event) => &event.raw,
            Event::ThreadMembersUpdate(event) => &event.raw,
            Event::MessageReactionRemoveEmoji(event) => &event.raw,
            Event::MessagePollVoteAdd(event) | Event::MessagePollVoteRemove(event) => &event.raw,
            Event::GuildStickersUpdate(event) => &event.raw,
            Event::GuildScheduledEventCreate(event)
            | Event::GuildScheduledEventUpdate(event)
            | Event::GuildScheduledEventDelete(event) => &event.raw,
            Event::GuildScheduledEventUserAdd(event)
            | Event::GuildScheduledEventUserRemove(event) => &event.raw,
            Event::StageInstanceCreate(event)
            | Event::StageInstanceUpdate(event)
            | Event::StageInstanceDelete(event) => &event.raw,
            Event::VoiceChannelEffectSend(event) => &event.raw,
            Event::VoiceChannelStartTimeUpdate(event) => &event.raw,
            Event::VoiceChannelStatusUpdate(event) => &event.raw,
            Event::ApplicationCommandPermissionsUpdate(event) => &event.raw,
            Event::AutoModerationRuleCreate(event)
            | Event::AutoModerationRuleUpdate(event)
            | Event::AutoModerationRuleDelete(event)
            | Event::AutoModerationActionExecution(event) => &event.raw,
            Event::GuildAuditLogEntryCreate(event) => &event.raw,
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
        "GUILD_MEMBERS_CHUNK" => Event::GuildMembersChunk(GuildMembersChunkEvent {
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
        "USER_UPDATE" => Event::UserUpdate(UserUpdateEvent {
            user: serde_json::from_value(data.clone())?,
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
        "RESUMED" => Event::Resumed(ResumedEvent { raw: data }),
        "THREAD_CREATE" => Event::ThreadCreate(ThreadEvent {
            thread: serde_json::from_value(data.clone())?,
            raw: data,
        }),
        "THREAD_UPDATE" => Event::ThreadUpdate(ThreadEvent {
            thread: serde_json::from_value(data.clone())?,
            raw: data,
        }),
        "THREAD_DELETE" => Event::ThreadDelete(ThreadEvent {
            thread: serde_json::from_value(data.clone())?,
            raw: data,
        }),
        "THREAD_LIST_SYNC" => Event::ThreadListSync(ThreadListSyncEvent {
            guild_id: data
                .get("guild_id")
                .and_then(|v| serde_json::from_value(v.clone()).ok()),
            threads: data
                .get("threads")
                .and_then(|v| serde_json::from_value(v.clone()).ok())
                .unwrap_or_default(),
            raw: data,
        }),
        "THREAD_MEMBER_UPDATE" => Event::ThreadMemberUpdate(ThreadMemberUpdateEvent {
            guild_id: data
                .get("guild_id")
                .and_then(|v| serde_json::from_value(v.clone()).ok()),
            thread_id: data
                .get("id")
                .and_then(|v| serde_json::from_value(v.clone()).ok()),
            raw: data,
        }),
        "THREAD_MEMBERS_UPDATE" => Event::ThreadMembersUpdate(ThreadMembersUpdateEvent {
            guild_id: data
                .get("guild_id")
                .and_then(|v| serde_json::from_value(v.clone()).ok()),
            thread_id: data
                .get("id")
                .and_then(|v| serde_json::from_value(v.clone()).ok()),
            added_members: data
                .get("added_members")
                .and_then(|v| serde_json::from_value(v.clone()).ok()),
            removed_member_ids: data
                .get("removed_member_ids")
                .and_then(|v| serde_json::from_value(v.clone()).ok()),
            member_count: data.get("member_count").and_then(|v| v.as_u64()),
            raw: data,
        }),
        "MESSAGE_REACTION_REMOVE_EMOJI" => {
            Event::MessageReactionRemoveEmoji(ReactionRemoveEmojiEvent {
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
            })
        }
        "GUILD_STICKERS_UPDATE" => Event::GuildStickersUpdate(GuildStickersUpdateEvent {
            guild_id: data
                .get("guild_id")
                .and_then(|v| serde_json::from_value(v.clone()).ok()),
            stickers: data
                .get("stickers")
                .and_then(|v| serde_json::from_value(v.clone()).ok())
                .unwrap_or_default(),
            raw: data,
        }),
        "ENTITLEMENT_CREATE" => Event::EntitlementCreate(EntitlementEvent {
            entitlement: serde_json::from_value(data.clone())?,
            raw: data,
        }),
        "ENTITLEMENT_UPDATE" => Event::EntitlementUpdate(EntitlementEvent {
            entitlement: serde_json::from_value(data.clone())?,
            raw: data,
        }),
        "ENTITLEMENT_DELETE" => Event::EntitlementDelete(EntitlementEvent {
            entitlement: serde_json::from_value(data.clone())?,
            raw: data,
        }),
        "SUBSCRIPTION_CREATE" => Event::SubscriptionCreate(SubscriptionEvent {
            subscription: serde_json::from_value(data.clone())?,
            raw: data,
        }),
        "SUBSCRIPTION_UPDATE" => Event::SubscriptionUpdate(SubscriptionEvent {
            subscription: serde_json::from_value(data.clone())?,
            raw: data,
        }),
        "SUBSCRIPTION_DELETE" => Event::SubscriptionDelete(SubscriptionEvent {
            subscription: serde_json::from_value(data.clone())?,
            raw: data,
        }),
        "INTEGRATION_CREATE" => Event::IntegrationCreate(IntegrationEvent {
            guild_id: read_optional_snowflake(&data, "guild_id"),
            integration: serde_json::from_value(data.clone())?,
            raw: data,
        }),
        "INTEGRATION_UPDATE" => Event::IntegrationUpdate(IntegrationEvent {
            guild_id: read_optional_snowflake(&data, "guild_id"),
            integration: serde_json::from_value(data.clone())?,
            raw: data,
        }),
        "INTEGRATION_DELETE" => Event::IntegrationDelete(IntegrationDeleteEvent {
            id: read_optional_snowflake(&data, "id"),
            guild_id: read_optional_snowflake(&data, "guild_id"),
            application_id: read_optional_snowflake(&data, "application_id"),
            raw: data,
        }),
        "GUILD_SOUNDBOARD_SOUND_CREATE" => {
            Event::GuildSoundboardSoundCreate(SoundboardSoundEvent {
                sound: serde_json::from_value(data.clone())?,
                raw: data,
            })
        }
        "GUILD_SOUNDBOARD_SOUND_UPDATE" => {
            Event::GuildSoundboardSoundUpdate(SoundboardSoundEvent {
                sound: serde_json::from_value(data.clone())?,
                raw: data,
            })
        }
        "GUILD_SOUNDBOARD_SOUND_DELETE" => {
            Event::GuildSoundboardSoundDelete(SoundboardSoundDeleteEvent {
                sound_id: read_required_snowflake(&data, "sound_id")?,
                guild_id: read_required_snowflake(&data, "guild_id")?,
                raw: data,
            })
        }
        "GUILD_SOUNDBOARD_SOUNDS_UPDATE" => {
            Event::GuildSoundboardSoundsUpdate(decode_soundboard_sounds_event(data)?)
        }
        "SOUNDBOARD_SOUNDS" => Event::SoundboardSounds(decode_soundboard_sounds_event(data)?),
        "GUILD_SCHEDULED_EVENT_CREATE" => {
            Event::GuildScheduledEventCreate(decode_scheduled_event(data)?)
        }
        "GUILD_SCHEDULED_EVENT_UPDATE" => {
            Event::GuildScheduledEventUpdate(decode_scheduled_event(data)?)
        }
        "GUILD_SCHEDULED_EVENT_DELETE" => {
            Event::GuildScheduledEventDelete(decode_scheduled_event(data)?)
        }
        "GUILD_SCHEDULED_EVENT_USER_ADD" => {
            Event::GuildScheduledEventUserAdd(decode_scheduled_event_user_event(data)?)
        }
        "GUILD_SCHEDULED_EVENT_USER_REMOVE" => {
            Event::GuildScheduledEventUserRemove(decode_scheduled_event_user_event(data)?)
        }
        "STAGE_INSTANCE_CREATE" => Event::StageInstanceCreate(decode_stage_instance_event(data)?),
        "STAGE_INSTANCE_UPDATE" => Event::StageInstanceUpdate(decode_stage_instance_event(data)?),
        "STAGE_INSTANCE_DELETE" => Event::StageInstanceDelete(decode_stage_instance_event(data)?),
        "VOICE_CHANNEL_EFFECT_SEND" => {
            Event::VoiceChannelEffectSend(decode_voice_channel_effect_event(data))
        }
        "VOICE_CHANNEL_START_TIME_UPDATE" => {
            Event::VoiceChannelStartTimeUpdate(decode_voice_channel_start_time_update_event(data))
        }
        "VOICE_CHANNEL_STATUS_UPDATE" => {
            Event::VoiceChannelStatusUpdate(decode_voice_channel_status_update_event(data))
        }
        "APPLICATION_COMMAND_PERMISSIONS_UPDATE" => Event::ApplicationCommandPermissionsUpdate(
            decode_application_command_permissions_update_event(data),
        ),
        "AUTO_MODERATION_RULE_CREATE" => {
            Event::AutoModerationRuleCreate(decode_auto_moderation_event(data))
        }
        "AUTO_MODERATION_RULE_UPDATE" => {
            Event::AutoModerationRuleUpdate(decode_auto_moderation_event(data))
        }
        "AUTO_MODERATION_RULE_DELETE" => {
            Event::AutoModerationRuleDelete(decode_auto_moderation_event(data))
        }
        "AUTO_MODERATION_ACTION_EXECUTION" => {
            Event::AutoModerationActionExecution(decode_auto_moderation_event(data))
        }
        "GUILD_AUDIT_LOG_ENTRY_CREATE" => {
            Event::GuildAuditLogEntryCreate(decode_audit_log_entry_event(data))
        }
        "MESSAGE_POLL_VOTE_ADD" => Event::MessagePollVoteAdd(decode_poll_vote_event(data)),
        "MESSAGE_POLL_VOTE_REMOVE" => Event::MessagePollVoteRemove(decode_poll_vote_event(data)),
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

fn read_optional_snowflake(value: &Value, field: &str) -> Option<Snowflake> {
    value
        .get(field)
        .and_then(|v| serde_json::from_value(v.clone()).ok())
}

fn read_optional_string(value: &Value, field: &str) -> Option<String> {
    value.get(field).and_then(|v| v.as_str().map(String::from))
}

fn read_optional_u64(value: &Value, field: &str) -> Option<u64> {
    value.get(field).and_then(Value::as_u64)
}

fn decode_scheduled_event(data: Value) -> Result<ScheduledEvent, DiscordError> {
    Ok(ScheduledEvent {
        id: read_optional_snowflake(&data, "id"),
        guild_id: read_optional_snowflake(&data, "guild_id"),
        channel_id: read_optional_snowflake(&data, "channel_id"),
        creator_id: read_optional_snowflake(&data, "creator_id"),
        name: read_optional_string(&data, "name"),
        description: read_optional_string(&data, "description"),
        scheduled_start_time: read_optional_string(&data, "scheduled_start_time"),
        scheduled_end_time: read_optional_string(&data, "scheduled_end_time"),
        privacy_level: read_optional_u64(&data, "privacy_level"),
        status: read_optional_u64(&data, "status"),
        entity_type: read_optional_u64(&data, "entity_type"),
        entity_id: read_optional_snowflake(&data, "entity_id"),
        entity_metadata: data.get("entity_metadata").cloned(),
        user_count: read_optional_u64(&data, "user_count"),
        image: read_optional_string(&data, "image"),
        raw: data,
    })
}

fn decode_soundboard_sounds_event(data: Value) -> Result<SoundboardSoundsEvent, DiscordError> {
    Ok(SoundboardSoundsEvent {
        guild_id: read_required_snowflake(&data, "guild_id")?,
        soundboard_sounds: data
            .get("soundboard_sounds")
            .and_then(|v| serde_json::from_value(v.clone()).ok())
            .unwrap_or_default(),
        raw: data,
    })
}

fn decode_poll_vote_event(data: Value) -> PollVoteEvent {
    PollVoteEvent {
        user_id: read_optional_snowflake(&data, "user_id"),
        channel_id: read_optional_snowflake(&data, "channel_id"),
        message_id: read_optional_snowflake(&data, "message_id"),
        guild_id: read_optional_snowflake(&data, "guild_id"),
        answer_id: read_optional_u64(&data, "answer_id"),
        raw: data,
    }
}

fn decode_scheduled_event_user_event(
    data: Value,
) -> Result<GuildScheduledEventUserEvent, DiscordError> {
    let mut event: GuildScheduledEventUserEvent = serde_json::from_value(data.clone())?;
    event.raw = data;
    Ok(event)
}

fn decode_stage_instance_event(data: Value) -> Result<StageInstanceEvent, DiscordError> {
    Ok(StageInstanceEvent {
        stage_instance: serde_json::from_value(data.clone())?,
        raw: data,
    })
}

fn decode_application_command_permissions_update_event(
    data: Value,
) -> ApplicationCommandPermissionsUpdateEvent {
    ApplicationCommandPermissionsUpdateEvent {
        id: read_optional_snowflake(&data, "id"),
        application_id: read_optional_snowflake(&data, "application_id"),
        guild_id: read_optional_snowflake(&data, "guild_id"),
        permissions: data
            .get("permissions")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default(),
        raw: data,
    }
}

fn decode_voice_channel_effect_event(data: Value) -> VoiceChannelEffectEvent {
    VoiceChannelEffectEvent {
        channel_id: read_optional_snowflake(&data, "channel_id"),
        guild_id: read_optional_snowflake(&data, "guild_id"),
        user_id: read_optional_snowflake(&data, "user_id"),
        emoji: data
            .get("emoji")
            .cloned()
            .and_then(|value| serde_json::from_value(value).ok()),
        animation_type: read_optional_u64(&data, "animation_type"),
        animation_id: read_optional_u64(&data, "animation_id"),
        sound_id: read_optional_snowflake(&data, "sound_id"),
        sound_volume: data.get("sound_volume").and_then(Value::as_f64),
        raw: data,
    }
}

fn decode_voice_channel_start_time_update_event(data: Value) -> VoiceChannelStartTimeUpdateEvent {
    VoiceChannelStartTimeUpdateEvent {
        channel_id: read_optional_snowflake(&data, "channel_id"),
        guild_id: read_optional_snowflake(&data, "guild_id"),
        voice_channel_start_time: read_optional_string(&data, "voice_channel_start_time"),
        raw: data,
    }
}

fn decode_voice_channel_status_update_event(data: Value) -> VoiceChannelStatusUpdateEvent {
    VoiceChannelStatusUpdateEvent {
        channel_id: read_optional_snowflake(&data, "channel_id"),
        guild_id: read_optional_snowflake(&data, "guild_id"),
        status: read_optional_string(&data, "status"),
        raw: data,
    }
}

fn decode_auto_moderation_event(data: Value) -> AutoModerationEvent {
    AutoModerationEvent {
        id: read_optional_snowflake(&data, "id"),
        guild_id: read_optional_snowflake(&data, "guild_id"),
        name: read_optional_string(&data, "name"),
        creator_id: read_optional_snowflake(&data, "creator_id"),
        event_type: read_optional_u64(&data, "event_type"),
        trigger_type: read_optional_u64(&data, "trigger_type"),
        trigger_metadata: data.get("trigger_metadata").cloned(),
        actions: data
            .get("actions")
            .and_then(|v| serde_json::from_value(v.clone()).ok())
            .unwrap_or_default(),
        enabled: data.get("enabled").and_then(Value::as_bool),
        exempt_roles: data
            .get("exempt_roles")
            .and_then(|v| serde_json::from_value(v.clone()).ok())
            .unwrap_or_default(),
        exempt_channels: data
            .get("exempt_channels")
            .and_then(|v| serde_json::from_value(v.clone()).ok())
            .unwrap_or_default(),
        action: data.get("action").cloned(),
        rule_id: read_optional_snowflake(&data, "rule_id"),
        rule_trigger_type: read_optional_u64(&data, "rule_trigger_type"),
        user_id: read_optional_snowflake(&data, "user_id"),
        channel_id: read_optional_snowflake(&data, "channel_id"),
        message_id: read_optional_snowflake(&data, "message_id"),
        alert_system_message_id: read_optional_snowflake(&data, "alert_system_message_id"),
        content: read_optional_string(&data, "content"),
        matched_keyword: read_optional_string(&data, "matched_keyword"),
        matched_content: read_optional_string(&data, "matched_content"),
        raw: data,
    }
}

fn decode_audit_log_entry_event(data: Value) -> AuditLogEntryEvent {
    AuditLogEntryEvent {
        guild_id: read_optional_snowflake(&data, "guild_id"),
        entry: serde_json::from_value(data.clone()).ok(),
        id: read_optional_snowflake(&data, "id"),
        user_id: read_optional_snowflake(&data, "user_id"),
        target_id: read_optional_snowflake(&data, "target_id"),
        action_type: read_optional_u64(&data, "action_type"),
        changes: data
            .get("changes")
            .and_then(|v| serde_json::from_value(v.clone()).ok()),
        options: data.get("options").cloned(),
        reason: read_optional_string(&data, "reason"),
        raw: data,
    }
}

#[cfg(test)]
mod tests {
    use serde_json::{json, Value};

    use super::*;
    use crate::error::DiscordError;
    use crate::model::{
        Channel, Entitlement, Guild, Integration, IntegrationAccount, Interaction,
        InteractionContextData, Member, Message, PingInteraction, Role, Snowflake, SoundboardSound,
        Subscription, User, VoiceServerUpdate, VoiceState,
    };
    use crate::types::Emoji;

    fn snowflake(id: &str) -> Snowflake {
        Snowflake::new(id)
    }

    fn raw(kind: &str) -> Value {
        json!({ "kind": kind })
    }

    fn user(id: &str, username: &str) -> User {
        User {
            id: snowflake(id),
            username: username.to_string(),
            ..Default::default()
        }
    }

    fn guild(id: &str, name: &str) -> Guild {
        Guild {
            id: snowflake(id),
            name: name.to_string(),
            ..Default::default()
        }
    }

    fn channel(id: &str) -> Channel {
        Channel {
            id: snowflake(id),
            kind: 0,
            ..Default::default()
        }
    }

    fn member(id: &str, username: &str) -> Member {
        Member {
            user: Some(user(id, username)),
            ..Default::default()
        }
    }

    fn role(id: &str, name: &str) -> Role {
        Role {
            id: snowflake(id),
            name: name.to_string(),
            ..Default::default()
        }
    }

    fn message(id: &str, channel_id: &str, content: &str) -> Message {
        Message {
            id: snowflake(id),
            channel_id: snowflake(channel_id),
            content: content.to_string(),
            ..Default::default()
        }
    }

    fn interaction_context() -> InteractionContextData {
        InteractionContextData {
            id: snowflake("400"),
            application_id: snowflake("401"),
            token: "token".to_string(),
            ..Default::default()
        }
    }

    fn assert_kind_and_raw(event: Event, expected_kind: &str) {
        assert_eq!(event.kind(), expected_kind);
        assert_eq!(event.raw(), &raw(expected_kind));
    }

    #[test]
    fn decode_message_create_event_returns_typed_payload() {
        let raw = json!({
            "id": "2",
            "channel_id": "1",
            "content": "hello",
            "mentions": [],
            "attachments": []
        });
        let event = decode_event("MESSAGE_CREATE", raw.clone()).unwrap();

        match event {
            Event::MessageCreate(message) => {
                assert_eq!(message.message.content, "hello");
                assert_eq!(message.raw, raw);
            }
            other => panic!("unexpected event: {other:?}"),
        }
    }

    #[test]
    fn decode_event_handles_optional_field_fallbacks() {
        let emojis_update = decode_event(
            "GUILD_EMOJIS_UPDATE",
            json!({
                "guild_id": "1",
                "emojis": {}
            }),
        )
        .unwrap();
        match emojis_update {
            Event::GuildEmojisUpdate(event) => {
                assert_eq!(event.guild_id, snowflake("1"));
                assert!(event.emojis.is_empty());
                assert_eq!(event.raw, json!({"guild_id": "1", "emojis": {}}));
            }
            other => panic!("unexpected event: {other:?}"),
        }

        let webhooks_update = decode_event(
            "WEBHOOKS_UPDATE",
            json!({
                "guild_id": {},
                "channel_id": {}
            }),
        )
        .unwrap();
        match webhooks_update {
            Event::WebhooksUpdate(event) => {
                assert_eq!(event.guild_id, None);
                assert_eq!(event.channel_id, None);
            }
            other => panic!("unexpected event: {other:?}"),
        }

        let invite_create = decode_event(
            "INVITE_CREATE",
            json!({
                "guild_id": {},
                "channel_id": {},
                "code": 42
            }),
        )
        .unwrap();
        match invite_create {
            Event::InviteCreate(event) => {
                assert_eq!(event.guild_id, None);
                assert_eq!(event.channel_id, None);
                assert_eq!(event.code, None);
            }
            other => panic!("unexpected event: {other:?}"),
        }

        let invite_delete = decode_event(
            "INVITE_DELETE",
            json!({
                "guild_id": {},
                "channel_id": {},
                "code": 42
            }),
        )
        .unwrap();
        match invite_delete {
            Event::InviteDelete(event) => {
                assert_eq!(event.guild_id, None);
                assert_eq!(event.channel_id, None);
                assert_eq!(event.code, None);
            }
            other => panic!("unexpected event: {other:?}"),
        }

        let pins_update = decode_event(
            "CHANNEL_PINS_UPDATE",
            json!({
                "channel_id": "2",
                "guild_id": {},
                "last_pin_timestamp": 123
            }),
        )
        .unwrap();
        match pins_update {
            Event::ChannelPinsUpdate(event) => {
                assert_eq!(event.channel_id, snowflake("2"));
                assert_eq!(event.guild_id, None);
                assert_eq!(event.last_pin_timestamp, None);
            }
            other => panic!("unexpected event: {other:?}"),
        }

        let typing_start = decode_event(
            "TYPING_START",
            json!({
                "channel_id": {},
                "guild_id": {},
                "user_id": {},
                "timestamp": "later"
            }),
        )
        .unwrap();
        match typing_start {
            Event::TypingStart(event) => {
                assert_eq!(event.channel_id, None);
                assert_eq!(event.guild_id, None);
                assert_eq!(event.user_id, None);
                assert_eq!(event.timestamp, None);
            }
            other => panic!("unexpected event: {other:?}"),
        }

        let presence_update = decode_event(
            "PRESENCE_UPDATE",
            json!({
                "guild_id": {},
                "status": 1,
                "user": { "id": 9 }
            }),
        )
        .unwrap();
        match presence_update {
            Event::PresenceUpdate(event) => {
                assert_eq!(event.user_id, None);
                assert_eq!(event.guild_id, None);
                assert_eq!(event.status, None);
            }
            other => panic!("unexpected event: {other:?}"),
        }

        let integrations_update = decode_event(
            "GUILD_INTEGRATIONS_UPDATE",
            json!({
                "guild_id": {}
            }),
        )
        .unwrap();
        match integrations_update {
            Event::GuildIntegrationsUpdate(event) => {
                assert_eq!(event.guild_id, None);
            }
            other => panic!("unexpected event: {other:?}"),
        }
    }

    #[test]
    fn decode_event_reads_nested_and_required_payloads() {
        let member_add = decode_event(
            "GUILD_MEMBER_ADD",
            json!({
                "guild_id": "100",
                "user": {
                    "id": "200",
                    "username": "member"
                }
            }),
        )
        .unwrap();
        match member_add {
            Event::MemberAdd(event) => {
                assert_eq!(event.guild_id, snowflake("100"));
                assert_eq!(
                    event
                        .member
                        .user
                        .as_ref()
                        .map(|user| user.username.as_str()),
                    Some("member")
                );
            }
            other => panic!("unexpected event: {other:?}"),
        }

        let role_create = decode_event(
            "GUILD_ROLE_CREATE",
            json!({
                "guild_id": "100",
                "role": {
                    "id": "300",
                    "name": "mods"
                }
            }),
        )
        .unwrap();
        match role_create {
            Event::RoleCreate(event) => {
                assert_eq!(event.guild_id, snowflake("100"));
                assert_eq!(event.role.id, snowflake("300"));
                assert_eq!(event.role.name, "mods");
            }
            other => panic!("unexpected event: {other:?}"),
        }

        let bulk_delete = decode_event(
            "MESSAGE_DELETE_BULK",
            json!({
                "ids": ["10", "11"],
                "channel_id": "12",
                "guild_id": "13"
            }),
        )
        .unwrap();
        match bulk_delete {
            Event::MessageDeleteBulk(event) => {
                assert_eq!(event.ids, vec![snowflake("10"), snowflake("11")]);
                assert_eq!(event.channel_id, snowflake("12"));
                assert_eq!(event.guild_id, Some(snowflake("13")));
                assert_eq!(
                    event.raw,
                    json!({
                        "ids": ["10", "11"],
                        "channel_id": "12",
                        "guild_id": "13"
                    })
                );
            }
            other => panic!("unexpected event: {other:?}"),
        }

        let pins_update = decode_event(
            "CHANNEL_PINS_UPDATE",
            json!({
                "channel_id": "14",
                "last_pin_timestamp": "2024-01-01T00:00:00Z"
            }),
        )
        .unwrap();
        match pins_update {
            Event::ChannelPinsUpdate(event) => {
                assert_eq!(event.channel_id, snowflake("14"));
                assert_eq!(
                    event.last_pin_timestamp.as_deref(),
                    Some("2024-01-01T00:00:00Z")
                );
            }
            other => panic!("unexpected event: {other:?}"),
        }

        let typing_start = decode_event(
            "TYPING_START",
            json!({
                "channel_id": "15",
                "guild_id": "16",
                "user_id": "17",
                "timestamp": 12345
            }),
        )
        .unwrap();
        match typing_start {
            Event::TypingStart(event) => {
                assert_eq!(event.channel_id, Some(snowflake("15")));
                assert_eq!(event.guild_id, Some(snowflake("16")));
                assert_eq!(event.user_id, Some(snowflake("17")));
                assert_eq!(event.timestamp, Some(12345));
            }
            other => panic!("unexpected event: {other:?}"),
        }

        let presence_update = decode_event(
            "PRESENCE_UPDATE",
            json!({
                "guild_id": "18",
                "status": "online",
                "user": { "id": "19" }
            }),
        )
        .unwrap();
        match presence_update {
            Event::PresenceUpdate(event) => {
                assert_eq!(event.guild_id, Some(snowflake("18")));
                assert_eq!(event.user_id, Some(snowflake("19")));
                assert_eq!(event.status.as_deref(), Some("online"));
            }
            other => panic!("unexpected event: {other:?}"),
        }
    }

    #[test]
    fn decode_event_covers_additional_typed_gateway_payloads() {
        match decode_event(
            "GUILD_CREATE",
            json!({
                "id": "1",
                "name": "discordrs",
                "roles": []
            }),
        )
        .unwrap()
        {
            Event::GuildCreate(event) => {
                assert_eq!(event.guild.id, snowflake("1"));
                assert_eq!(event.guild.name, "discordrs");
            }
            other => panic!("unexpected guild event: {other:?}"),
        }

        match decode_event(
            "CHANNEL_CREATE",
            json!({
                "id": "2",
                "type": 0,
                "name": "general"
            }),
        )
        .unwrap()
        {
            Event::ChannelCreate(event) => {
                assert_eq!(event.channel.id, snowflake("2"));
                assert_eq!(event.channel.name.as_deref(), Some("general"));
            }
            other => panic!("unexpected channel event: {other:?}"),
        }

        match decode_event(
            "GUILD_MEMBER_REMOVE",
            json!({
                "guild_id": "3",
                "user": {
                    "id": "4",
                    "username": "member"
                }
            }),
        )
        .unwrap()
        {
            Event::MemberRemove(event) => {
                assert_eq!(event.data.guild_id, snowflake("3"));
                assert_eq!(event.data.user.id, snowflake("4"));
            }
            other => panic!("unexpected member removal event: {other:?}"),
        }

        match decode_event(
            "GUILD_ROLE_DELETE",
            json!({
                "guild_id": "5",
                "role_id": "6"
            }),
        )
        .unwrap()
        {
            Event::RoleDelete(event) => {
                assert_eq!(event.data.guild_id, snowflake("5"));
                assert_eq!(event.data.role_id, snowflake("6"));
            }
            other => panic!("unexpected role delete event: {other:?}"),
        }
    }

    #[test]
    fn decode_event_exposes_common_fields_for_newer_gateway_payloads() {
        match decode_event(
            "GUILD_SCHEDULED_EVENT_CREATE",
            json!({
                "id": "700",
                "guild_id": "701",
                "channel_id": "702",
                "creator_id": "703",
                "name": "Launch",
                "description": "Release stream",
                "scheduled_start_time": "2026-04-30T01:00:00Z",
                "scheduled_end_time": "2026-04-30T02:00:00Z",
                "privacy_level": 2,
                "status": 1,
                "entity_type": 2,
                "entity_id": "704",
                "entity_metadata": { "location": "voice" },
                "user_count": 42,
                "image": "cover"
            }),
        )
        .unwrap()
        {
            Event::GuildScheduledEventCreate(event) => {
                assert_eq!(event.id, Some(snowflake("700")));
                assert_eq!(event.guild_id, Some(snowflake("701")));
                assert_eq!(event.channel_id, Some(snowflake("702")));
                assert_eq!(event.creator_id, Some(snowflake("703")));
                assert_eq!(event.name.as_deref(), Some("Launch"));
                assert_eq!(event.description.as_deref(), Some("Release stream"));
                assert_eq!(
                    event.scheduled_start_time.as_deref(),
                    Some("2026-04-30T01:00:00Z")
                );
                assert_eq!(event.status, Some(1));
                assert_eq!(event.entity_type, Some(2));
                assert_eq!(event.entity_id, Some(snowflake("704")));
                assert_eq!(event.entity_metadata, Some(json!({ "location": "voice" })));
                assert_eq!(event.user_count, Some(42));
                assert_eq!(event.image.as_deref(), Some("cover"));
            }
            other => panic!("unexpected scheduled event: {other:?}"),
        }

        match decode_event(
            "AUTO_MODERATION_RULE_CREATE",
            json!({
                "id": "710",
                "guild_id": "711",
                "name": "Keyword Filter",
                "creator_id": "712",
                "event_type": 1,
                "trigger_type": 1,
                "trigger_metadata": { "keyword_filter": ["bad"] },
                "actions": [{ "type": 1 }],
                "enabled": true,
                "exempt_roles": ["713"],
                "exempt_channels": ["714"]
            }),
        )
        .unwrap()
        {
            Event::AutoModerationRuleCreate(event) => {
                assert_eq!(event.id, Some(snowflake("710")));
                assert_eq!(event.guild_id, Some(snowflake("711")));
                assert_eq!(event.name.as_deref(), Some("Keyword Filter"));
                assert_eq!(event.creator_id, Some(snowflake("712")));
                assert_eq!(event.event_type, Some(1));
                assert_eq!(event.trigger_type, Some(1));
                assert_eq!(
                    event.trigger_metadata,
                    Some(json!({ "keyword_filter": ["bad"] }))
                );
                assert_eq!(event.actions, vec![json!({ "type": 1 })]);
                assert_eq!(event.enabled, Some(true));
                assert_eq!(event.exempt_roles, vec![snowflake("713")]);
                assert_eq!(event.exempt_channels, vec![snowflake("714")]);
            }
            other => panic!("unexpected auto moderation rule event: {other:?}"),
        }

        match decode_event(
            "AUTO_MODERATION_ACTION_EXECUTION",
            json!({
                "guild_id": "720",
                "action": { "type": 2, "metadata": { "channel_id": "721" } },
                "rule_id": "722",
                "rule_trigger_type": 1,
                "user_id": "723",
                "channel_id": "724",
                "message_id": "725",
                "alert_system_message_id": "726",
                "content": "blocked text",
                "matched_keyword": "blocked",
                "matched_content": "blocked"
            }),
        )
        .unwrap()
        {
            Event::AutoModerationActionExecution(event) => {
                assert_eq!(event.guild_id, Some(snowflake("720")));
                assert_eq!(
                    event.action,
                    Some(json!({ "type": 2, "metadata": { "channel_id": "721" } }))
                );
                assert_eq!(event.rule_id, Some(snowflake("722")));
                assert_eq!(event.rule_trigger_type, Some(1));
                assert_eq!(event.user_id, Some(snowflake("723")));
                assert_eq!(event.channel_id, Some(snowflake("724")));
                assert_eq!(event.message_id, Some(snowflake("725")));
                assert_eq!(event.alert_system_message_id, Some(snowflake("726")));
                assert_eq!(event.content.as_deref(), Some("blocked text"));
                assert_eq!(event.matched_keyword.as_deref(), Some("blocked"));
                assert_eq!(event.matched_content.as_deref(), Some("blocked"));
            }
            other => panic!("unexpected auto moderation action event: {other:?}"),
        }

        match decode_event(
            "GUILD_AUDIT_LOG_ENTRY_CREATE",
            json!({
                "guild_id": "730",
                "id": "731",
                "user_id": "732",
                "target_id": "733",
                "action_type": 22,
                "changes": [{ "key": "nick", "new_value": "new" }],
                "options": { "delete_member_days": "1" },
                "reason": "cleanup"
            }),
        )
        .unwrap()
        {
            Event::GuildAuditLogEntryCreate(event) => {
                assert_eq!(event.guild_id, Some(snowflake("730")));
                assert_eq!(event.id, Some(snowflake("731")));
                assert_eq!(event.user_id, Some(snowflake("732")));
                assert_eq!(event.target_id, Some(snowflake("733")));
                assert_eq!(event.action_type, Some(22));
                assert_eq!(
                    event.changes,
                    Some(vec![json!({ "key": "nick", "new_value": "new" })])
                );
                assert_eq!(event.options, Some(json!({ "delete_member_days": "1" })));
                assert_eq!(event.reason.as_deref(), Some("cleanup"));
                assert_eq!(
                    event
                        .entry
                        .as_ref()
                        .and_then(|entry| entry.id.as_ref())
                        .map(Snowflake::as_str),
                    Some("731")
                );
            }
            other => panic!("unexpected audit log entry event: {other:?}"),
        }

        match decode_event(
            "USER_UPDATE",
            json!({
                "id": "740",
                "username": "bot"
            }),
        )
        .unwrap()
        {
            Event::UserUpdate(event) => {
                assert_eq!(event.user.id, snowflake("740"));
                assert_eq!(event.user.username, "bot");
            }
            other => panic!("unexpected user update event: {other:?}"),
        }
    }

    #[test]
    fn decode_event_covers_voice_ban_reaction_and_interaction_variants() {
        match decode_event(
            "VOICE_STATE_UPDATE",
            json!({
                "guild_id": "1",
                "channel_id": "2",
                "user_id": "3"
            }),
        )
        .unwrap()
        {
            Event::VoiceStateUpdate(event) => {
                assert_eq!(event.state.guild_id, Some(snowflake("1")));
                assert_eq!(event.state.channel_id, Some(snowflake("2")));
                assert_eq!(event.state.user_id, Some(snowflake("3")));
            }
            other => panic!("unexpected voice state event: {other:?}"),
        }

        match decode_event(
            "VOICE_SERVER_UPDATE",
            json!({
                "guild_id": "4",
                "token": "voice-token",
                "endpoint": "wss://voice.discord.test"
            }),
        )
        .unwrap()
        {
            Event::VoiceServerUpdate(event) => {
                assert_eq!(event.data.guild_id, snowflake("4"));
                assert_eq!(event.data.token, "voice-token");
                assert_eq!(
                    event.data.endpoint.as_deref(),
                    Some("wss://voice.discord.test")
                );
            }
            other => panic!("unexpected voice server event: {other:?}"),
        }

        match decode_event(
            "GUILD_BAN_ADD",
            json!({
                "guild_id": "7",
                "user": {
                    "id": "8",
                    "username": "banned"
                }
            }),
        )
        .unwrap()
        {
            Event::GuildBanAdd(event) => {
                assert_eq!(event.guild_id, snowflake("7"));
                assert_eq!(event.user.username, "banned");
            }
            other => panic!("unexpected guild ban event: {other:?}"),
        }

        match decode_event(
            "MESSAGE_REACTION_ADD",
            json!({
                "user_id": "9",
                "channel_id": "10",
                "message_id": "11",
                "guild_id": "12",
                "emoji": {
                    "name": "🔥"
                }
            }),
        )
        .unwrap()
        {
            Event::MessageReactionAdd(event) => {
                assert_eq!(event.user_id, Some(snowflake("9")));
                assert_eq!(event.channel_id, Some(snowflake("10")));
                assert_eq!(event.message_id, Some(snowflake("11")));
                assert_eq!(event.guild_id, Some(snowflake("12")));
                assert_eq!(
                    event.emoji.and_then(|emoji| emoji.name),
                    Some("🔥".to_string())
                );
            }
            other => panic!("unexpected reaction event: {other:?}"),
        }

        match decode_event(
            "INTERACTION_CREATE",
            json!({
                "id": "13",
                "application_id": "14",
                "token": "interaction-token",
                "type": 1
            }),
        )
        .unwrap()
        {
            Event::InteractionCreate(event) => {
                assert!(matches!(event.interaction, Interaction::Ping(_)));
            }
            other => panic!("unexpected interaction event: {other:?}"),
        }
    }

    #[test]
    fn decode_event_covers_remaining_success_variants() {
        match decode_event(
            "READY",
            json!({
                "user": {
                    "id": "50",
                    "username": "ready"
                },
                "session_id": "session-50"
            }),
        )
        .unwrap()
        {
            Event::Ready(event) => {
                assert_eq!(event.data.user.id, snowflake("50"));
                assert_eq!(event.data.session_id, "session-50");
                assert!(event.data.application.is_none());
                assert!(event.data.resume_gateway_url.is_none());
            }
            other => panic!("unexpected ready event: {other:?}"),
        }

        match decode_event(
            "GUILD_UPDATE",
            json!({
                "id": "51",
                "name": "guild-update",
                "roles": []
            }),
        )
        .unwrap()
        {
            Event::GuildUpdate(event) => {
                assert_eq!(event.guild.id, snowflake("51"));
                assert_eq!(event.guild.name, "guild-update");
            }
            other => panic!("unexpected guild update event: {other:?}"),
        }

        match decode_event(
            "GUILD_DELETE",
            json!({
                "id": "52"
            }),
        )
        .unwrap()
        {
            Event::GuildDelete(event) => {
                assert_eq!(event.data.id, snowflake("52"));
                assert_eq!(event.data.unavailable, None);
            }
            other => panic!("unexpected guild delete event: {other:?}"),
        }

        match decode_event(
            "CHANNEL_UPDATE",
            json!({
                "id": "53",
                "type": 0
            }),
        )
        .unwrap()
        {
            Event::ChannelUpdate(event) => {
                assert_eq!(event.channel.id, snowflake("53"));
                assert_eq!(event.channel.kind, 0);
            }
            other => panic!("unexpected channel update event: {other:?}"),
        }

        match decode_event(
            "CHANNEL_DELETE",
            json!({
                "id": "54",
                "type": 0
            }),
        )
        .unwrap()
        {
            Event::ChannelDelete(event) => {
                assert_eq!(event.channel.id, snowflake("54"));
                assert_eq!(event.channel.kind, 0);
            }
            other => panic!("unexpected channel delete event: {other:?}"),
        }

        match decode_event(
            "GUILD_MEMBER_UPDATE",
            json!({
                "guild_id": "55",
                "user": {
                    "id": "56",
                    "username": "member-update"
                }
            }),
        )
        .unwrap()
        {
            Event::MemberUpdate(event) => {
                assert_eq!(event.guild_id, snowflake("55"));
                assert_eq!(
                    event
                        .member
                        .user
                        .as_ref()
                        .map(|user| user.username.as_str()),
                    Some("member-update")
                );
            }
            other => panic!("unexpected member update event: {other:?}"),
        }

        match decode_event(
            "GUILD_ROLE_UPDATE",
            json!({
                "guild_id": "57",
                "role": {
                    "id": "58",
                    "name": "role-update"
                }
            }),
        )
        .unwrap()
        {
            Event::RoleUpdate(event) => {
                assert_eq!(event.guild_id, snowflake("57"));
                assert_eq!(event.role.id, snowflake("58"));
                assert_eq!(event.role.name, "role-update");
            }
            other => panic!("unexpected role update event: {other:?}"),
        }

        match decode_event(
            "MESSAGE_UPDATE",
            json!({
                "id": "59",
                "channel_id": "60",
                "content": "edited",
                "mentions": [],
                "attachments": []
            }),
        )
        .unwrap()
        {
            Event::MessageUpdate(event) => {
                assert_eq!(event.message.id, snowflake("59"));
                assert_eq!(event.message.channel_id, snowflake("60"));
                assert_eq!(event.message.content, "edited");
            }
            other => panic!("unexpected message update event: {other:?}"),
        }

        match decode_event(
            "MESSAGE_DELETE",
            json!({
                "id": "61",
                "channel_id": "62"
            }),
        )
        .unwrap()
        {
            Event::MessageDelete(event) => {
                assert_eq!(event.data.id, snowflake("61"));
                assert_eq!(event.data.channel_id, snowflake("62"));
                assert_eq!(event.data.guild_id, None);
            }
            other => panic!("unexpected message delete event: {other:?}"),
        }

        match decode_event(
            "GUILD_BAN_REMOVE",
            json!({
                "guild_id": "63",
                "user": {
                    "id": "64",
                    "username": "ban-remove"
                }
            }),
        )
        .unwrap()
        {
            Event::GuildBanRemove(event) => {
                assert_eq!(event.guild_id, snowflake("63"));
                assert_eq!(event.user.id, snowflake("64"));
                assert_eq!(event.user.username, "ban-remove");
            }
            other => panic!("unexpected guild ban remove event: {other:?}"),
        }

        match decode_event(
            "GUILD_EMOJIS_UPDATE",
            json!({
                "guild_id": "65"
            }),
        )
        .unwrap()
        {
            Event::GuildEmojisUpdate(event) => {
                assert_eq!(event.guild_id, snowflake("65"));
                assert!(event.emojis.is_empty());
            }
            other => panic!("unexpected guild emojis update event: {other:?}"),
        }

        match decode_event(
            "GUILD_INTEGRATIONS_UPDATE",
            json!({
                "guild_id": "66"
            }),
        )
        .unwrap()
        {
            Event::GuildIntegrationsUpdate(event) => {
                assert_eq!(event.guild_id, Some(snowflake("66")));
            }
            other => panic!("unexpected integrations update event: {other:?}"),
        }

        match decode_event(
            "WEBHOOKS_UPDATE",
            json!({
                "guild_id": "67",
                "channel_id": "68"
            }),
        )
        .unwrap()
        {
            Event::WebhooksUpdate(event) => {
                assert_eq!(event.guild_id, Some(snowflake("67")));
                assert_eq!(event.channel_id, Some(snowflake("68")));
            }
            other => panic!("unexpected webhooks update event: {other:?}"),
        }

        match decode_event(
            "INVITE_DELETE",
            json!({
                "guild_id": "69",
                "channel_id": "70",
                "code": "invite-code"
            }),
        )
        .unwrap()
        {
            Event::InviteDelete(event) => {
                assert_eq!(event.guild_id, Some(snowflake("69")));
                assert_eq!(event.channel_id, Some(snowflake("70")));
                assert_eq!(event.code.as_deref(), Some("invite-code"));
            }
            other => panic!("unexpected invite delete event: {other:?}"),
        }

        match decode_event(
            "MESSAGE_REACTION_REMOVE",
            json!({
                "user_id": "71",
                "channel_id": "72",
                "message_id": "73",
                "guild_id": "74",
                "emoji": {
                    "name": "x"
                }
            }),
        )
        .unwrap()
        {
            Event::MessageReactionRemove(event) => {
                assert_eq!(event.user_id, Some(snowflake("71")));
                assert_eq!(event.channel_id, Some(snowflake("72")));
                assert_eq!(event.message_id, Some(snowflake("73")));
                assert_eq!(event.guild_id, Some(snowflake("74")));
                assert_eq!(
                    event.emoji.and_then(|emoji| emoji.name),
                    Some("x".to_string())
                );
            }
            other => panic!("unexpected reaction remove event: {other:?}"),
        }

        match decode_event(
            "MESSAGE_REACTION_REMOVE_ALL",
            json!({
                "channel_id": "75",
                "message_id": "76",
                "guild_id": "77"
            }),
        )
        .unwrap()
        {
            Event::MessageReactionRemoveAll(event) => {
                assert_eq!(event.channel_id, Some(snowflake("75")));
                assert_eq!(event.message_id, Some(snowflake("76")));
                assert_eq!(event.guild_id, Some(snowflake("77")));
            }
            other => panic!("unexpected reaction remove all event: {other:?}"),
        }
    }

    #[test]
    fn decode_event_covers_success_payloads_with_present_optional_fields() {
        match decode_event(
            "READY",
            json!({
                "user": {
                    "id": "80",
                    "username": "ready-plus"
                },
                "session_id": "session-80",
                "application": {
                    "id": "81"
                },
                "resume_gateway_url": "wss://gateway.discord.test"
            }),
        )
        .unwrap()
        {
            Event::Ready(event) => {
                assert_eq!(event.data.user.id, snowflake("80"));
                assert_eq!(
                    event.data.application.map(|app| app.id),
                    Some(snowflake("81"))
                );
                assert_eq!(
                    event.data.resume_gateway_url.as_deref(),
                    Some("wss://gateway.discord.test")
                );
            }
            other => panic!("unexpected ready event: {other:?}"),
        }

        match decode_event(
            "GUILD_DELETE",
            json!({
                "id": "82",
                "unavailable": true
            }),
        )
        .unwrap()
        {
            Event::GuildDelete(event) => {
                assert_eq!(event.data.id, snowflake("82"));
                assert_eq!(event.data.unavailable, Some(true));
            }
            other => panic!("unexpected guild delete event: {other:?}"),
        }

        match decode_event(
            "MESSAGE_DELETE",
            json!({
                "id": "83",
                "channel_id": "84",
                "guild_id": "85"
            }),
        )
        .unwrap()
        {
            Event::MessageDelete(event) => {
                assert_eq!(event.data.id, snowflake("83"));
                assert_eq!(event.data.channel_id, snowflake("84"));
                assert_eq!(event.data.guild_id, Some(snowflake("85")));
            }
            other => panic!("unexpected message delete event: {other:?}"),
        }

        match decode_event(
            "CHANNEL_PINS_UPDATE",
            json!({
                "channel_id": "86",
                "guild_id": "87",
                "last_pin_timestamp": "2024-06-01T00:00:00Z"
            }),
        )
        .unwrap()
        {
            Event::ChannelPinsUpdate(event) => {
                assert_eq!(event.channel_id, snowflake("86"));
                assert_eq!(event.guild_id, Some(snowflake("87")));
                assert_eq!(
                    event.last_pin_timestamp.as_deref(),
                    Some("2024-06-01T00:00:00Z")
                );
            }
            other => panic!("unexpected channel pins update event: {other:?}"),
        }

        match decode_event(
            "GUILD_EMOJIS_UPDATE",
            json!({
                "guild_id": "88",
                "emojis": [
                    {
                        "name": "wave"
                    }
                ]
            }),
        )
        .unwrap()
        {
            Event::GuildEmojisUpdate(event) => {
                assert_eq!(event.guild_id, snowflake("88"));
                assert_eq!(event.emojis.len(), 1);
                assert_eq!(event.emojis[0].name.as_deref(), Some("wave"));
            }
            other => panic!("unexpected guild emojis update event: {other:?}"),
        }

        match decode_event(
            "INVITE_CREATE",
            json!({
                "guild_id": "89",
                "channel_id": "90",
                "code": "invite-create"
            }),
        )
        .unwrap()
        {
            Event::InviteCreate(event) => {
                assert_eq!(event.guild_id, Some(snowflake("89")));
                assert_eq!(event.channel_id, Some(snowflake("90")));
                assert_eq!(event.code.as_deref(), Some("invite-create"));
            }
            other => panic!("unexpected invite create event: {other:?}"),
        }
    }

    #[test]
    fn decode_event_covers_new_gateway_surface_variants() {
        match decode_event(
            "GUILD_MEMBERS_CHUNK",
            json!({
                "guild_id": "1",
                "members": [{
                    "user": { "id": "2", "username": "member" },
                    "roles": ["3"]
                }],
                "chunk_index": 0,
                "chunk_count": 1,
                "not_found": ["4"],
                "presences": [{
                    "user_id": "2",
                    "status": "online",
                    "activities": [{ "name": "Testing", "type": 0 }]
                }],
                "nonce": "abc"
            }),
        )
        .unwrap()
        {
            Event::GuildMembersChunk(event) => {
                assert_eq!(event.data.guild_id.as_str(), "1");
                assert_eq!(event.data.members.len(), 1);
                assert_eq!(event.data.not_found[0].as_str(), "4");
                assert_eq!(
                    event.data.presences.unwrap()[0]
                        .activities
                        .as_ref()
                        .unwrap()[0]
                        .name,
                    "Testing"
                );
                assert_eq!(event.data.nonce.as_deref(), Some("abc"));
            }
            other => panic!("unexpected guild members chunk event: {other:?}"),
        }

        match decode_event("RESUMED", json!({ "trace": [] })).unwrap() {
            Event::Resumed(event) => assert_eq!(event.raw["trace"], json!([])),
            other => panic!("unexpected resumed event: {other:?}"),
        }

        match decode_event(
            "VOICE_CHANNEL_STATUS_UPDATE",
            json!({
                "guild_id": "1",
                "channel_id": "2",
                "status": "Live"
            }),
        )
        .unwrap()
        {
            Event::VoiceChannelStatusUpdate(event) => {
                assert_eq!(event.channel_id.unwrap().as_str(), "2");
                assert_eq!(event.status.as_deref(), Some("Live"));
            }
            other => panic!("unexpected voice channel status event: {other:?}"),
        }
    }

    #[test]
    fn decode_event_reports_required_field_errors_and_preserves_unknown_events() {
        let missing_guild_id = decode_event(
            "GUILD_MEMBER_ADD",
            json!({
                "user": {
                    "id": "20",
                    "username": "member"
                }
            }),
        )
        .unwrap_err();
        match missing_guild_id {
            DiscordError::Model { message } => assert_eq!(message, "missing field guild_id"),
            other => panic!("unexpected error: {other:?}"),
        }

        let invalid_guild_id = decode_event(
            "GUILD_ROLE_CREATE",
            json!({
                "guild_id": {},
                "role": {
                    "id": "21",
                    "name": "mods"
                }
            }),
        )
        .unwrap_err();
        assert!(
            matches!(invalid_guild_id, DiscordError::Json(message) if message.contains("snowflake"))
        );

        let missing_ids = decode_event(
            "MESSAGE_DELETE_BULK",
            json!({
                "channel_id": "22"
            }),
        )
        .unwrap_err();
        assert!(matches!(missing_ids, DiscordError::Json(_)));

        let raw = json!({ "x": 1 });
        let unknown = decode_event("SOMETHING_NEW", raw.clone()).unwrap();
        match unknown {
            Event::Unknown {
                kind,
                raw: event_raw,
            } => {
                assert_eq!(kind, "SOMETHING_NEW");
                assert_eq!(event_raw, raw);
            }
            other => panic!("unexpected event: {other:?}"),
        }
    }

    #[test]
    fn event_kind_and_raw_cover_remaining_variants() {
        let cases = vec![
            (
                "READY",
                Event::Ready(ReadyEvent {
                    data: ReadyPayload {
                        user: user("1", "ready"),
                        session_id: "session".to_string(),
                        application: Some(ReadyApplication { id: snowflake("2") }),
                        resume_gateway_url: Some("wss://gateway.discord.test".to_string()),
                    },
                    raw: raw("READY"),
                }),
            ),
            (
                "GUILD_CREATE",
                Event::GuildCreate(GuildEvent {
                    guild: guild("10", "guild-create"),
                    raw: raw("GUILD_CREATE"),
                }),
            ),
            (
                "GUILD_UPDATE",
                Event::GuildUpdate(GuildEvent {
                    guild: guild("11", "guild-update"),
                    raw: raw("GUILD_UPDATE"),
                }),
            ),
            (
                "GUILD_DELETE",
                Event::GuildDelete(GuildDeleteEvent {
                    data: GuildDeletePayload {
                        id: snowflake("12"),
                        unavailable: Some(true),
                    },
                    raw: raw("GUILD_DELETE"),
                }),
            ),
            (
                "CHANNEL_CREATE",
                Event::ChannelCreate(ChannelEvent {
                    channel: channel("13"),
                    raw: raw("CHANNEL_CREATE"),
                }),
            ),
            (
                "CHANNEL_UPDATE",
                Event::ChannelUpdate(ChannelEvent {
                    channel: channel("14"),
                    raw: raw("CHANNEL_UPDATE"),
                }),
            ),
            (
                "CHANNEL_DELETE",
                Event::ChannelDelete(ChannelEvent {
                    channel: channel("15"),
                    raw: raw("CHANNEL_DELETE"),
                }),
            ),
            (
                "GUILD_MEMBER_UPDATE",
                Event::MemberUpdate(MemberEvent {
                    guild_id: snowflake("16"),
                    member: member("17", "member-update"),
                    raw: raw("GUILD_MEMBER_UPDATE"),
                }),
            ),
            (
                "GUILD_MEMBER_REMOVE",
                Event::MemberRemove(MemberRemoveEvent {
                    data: MemberRemovePayload {
                        guild_id: snowflake("18"),
                        user: user("19", "member-remove"),
                    },
                    raw: raw("GUILD_MEMBER_REMOVE"),
                }),
            ),
            (
                "GUILD_ROLE_UPDATE",
                Event::RoleUpdate(RoleEvent {
                    guild_id: snowflake("20"),
                    role: role("21", "role-update"),
                    raw: raw("GUILD_ROLE_UPDATE"),
                }),
            ),
            (
                "GUILD_ROLE_DELETE",
                Event::RoleDelete(RoleDeleteEvent {
                    data: RoleDeletePayload {
                        guild_id: snowflake("22"),
                        role_id: snowflake("23"),
                    },
                    raw: raw("GUILD_ROLE_DELETE"),
                }),
            ),
            (
                "MESSAGE_UPDATE",
                Event::MessageUpdate(MessageEvent {
                    message: message("24", "25", "updated"),
                    raw: raw("MESSAGE_UPDATE"),
                }),
            ),
            (
                "MESSAGE_DELETE",
                Event::MessageDelete(MessageDeleteEvent {
                    data: MessageDeletePayload {
                        id: snowflake("26"),
                        channel_id: snowflake("27"),
                        guild_id: Some(snowflake("28")),
                    },
                    raw: raw("MESSAGE_DELETE"),
                }),
            ),
            (
                "GUILD_BAN_ADD",
                Event::GuildBanAdd(GuildBanEvent {
                    guild_id: snowflake("29"),
                    user: user("30", "ban-add"),
                    raw: raw("GUILD_BAN_ADD"),
                }),
            ),
            (
                "GUILD_BAN_REMOVE",
                Event::GuildBanRemove(GuildBanEvent {
                    guild_id: snowflake("31"),
                    user: user("32", "ban-remove"),
                    raw: raw("GUILD_BAN_REMOVE"),
                }),
            ),
            (
                "MESSAGE_REACTION_ADD",
                Event::MessageReactionAdd(ReactionEvent {
                    user_id: Some(snowflake("33")),
                    channel_id: Some(snowflake("34")),
                    message_id: Some(snowflake("35")),
                    guild_id: Some(snowflake("36")),
                    emoji: Some(Emoji::unicode("🔥")),
                    raw: raw("MESSAGE_REACTION_ADD"),
                }),
            ),
            (
                "MESSAGE_REACTION_REMOVE",
                Event::MessageReactionRemove(ReactionEvent {
                    user_id: Some(snowflake("37")),
                    channel_id: Some(snowflake("38")),
                    message_id: Some(snowflake("39")),
                    guild_id: Some(snowflake("40")),
                    emoji: Some(Emoji::unicode("🔥")),
                    raw: raw("MESSAGE_REACTION_REMOVE"),
                }),
            ),
            (
                "MESSAGE_REACTION_REMOVE_ALL",
                Event::MessageReactionRemoveAll(ReactionRemoveAllEvent {
                    channel_id: Some(snowflake("41")),
                    message_id: Some(snowflake("42")),
                    guild_id: Some(snowflake("43")),
                    raw: raw("MESSAGE_REACTION_REMOVE_ALL"),
                }),
            ),
            (
                "INTERACTION_CREATE",
                Event::InteractionCreate(InteractionEvent {
                    interaction: Interaction::Ping(PingInteraction {
                        context: interaction_context(),
                    }),
                    raw: raw("INTERACTION_CREATE"),
                }),
            ),
            (
                "VOICE_STATE_UPDATE",
                Event::VoiceStateUpdate(VoiceStateEvent {
                    state: VoiceState {
                        guild_id: Some(snowflake("44")),
                        channel_id: Some(snowflake("45")),
                        user_id: Some(snowflake("46")),
                        ..Default::default()
                    },
                    raw: raw("VOICE_STATE_UPDATE"),
                }),
            ),
            (
                "VOICE_SERVER_UPDATE",
                Event::VoiceServerUpdate(VoiceServerEvent {
                    data: VoiceServerUpdate {
                        guild_id: snowflake("47"),
                        token: "voice-token".to_string(),
                        endpoint: Some("wss://voice.discord.test".to_string()),
                    },
                    raw: raw("VOICE_SERVER_UPDATE"),
                }),
            ),
        ];

        for (kind, event) in cases {
            assert_kind_and_raw(event, kind);
        }
    }

    #[test]
    fn event_kind_and_raw_cover_missing_variants() {
        let cases = vec![
            (
                "GUILD_MEMBER_ADD",
                Event::MemberAdd(MemberEvent {
                    guild_id: snowflake("80"),
                    member: member("81", "member-add"),
                    raw: raw("GUILD_MEMBER_ADD"),
                }),
            ),
            (
                "GUILD_ROLE_CREATE",
                Event::RoleCreate(RoleEvent {
                    guild_id: snowflake("82"),
                    role: role("83", "role-create"),
                    raw: raw("GUILD_ROLE_CREATE"),
                }),
            ),
            (
                "MESSAGE_CREATE",
                Event::MessageCreate(MessageEvent {
                    message: message("84", "85", "created"),
                    raw: raw("MESSAGE_CREATE"),
                }),
            ),
            (
                "MESSAGE_DELETE_BULK",
                Event::MessageDeleteBulk(BulkMessageDeleteEvent {
                    ids: vec![snowflake("86"), snowflake("87")],
                    channel_id: snowflake("88"),
                    guild_id: Some(snowflake("89")),
                    raw: raw("MESSAGE_DELETE_BULK"),
                }),
            ),
            (
                "CHANNEL_PINS_UPDATE",
                Event::ChannelPinsUpdate(ChannelPinsUpdateEvent {
                    channel_id: snowflake("90"),
                    guild_id: Some(snowflake("91")),
                    last_pin_timestamp: Some("2024-07-01T00:00:00Z".to_string()),
                    raw: raw("CHANNEL_PINS_UPDATE"),
                }),
            ),
            (
                "GUILD_EMOJIS_UPDATE",
                Event::GuildEmojisUpdate(GuildEmojisUpdateEvent {
                    guild_id: snowflake("92"),
                    emojis: vec![Emoji::unicode("wave")],
                    raw: raw("GUILD_EMOJIS_UPDATE"),
                }),
            ),
            (
                "GUILD_INTEGRATIONS_UPDATE",
                Event::GuildIntegrationsUpdate(GuildIntegrationsUpdateEvent {
                    guild_id: Some(snowflake("93")),
                    raw: raw("GUILD_INTEGRATIONS_UPDATE"),
                }),
            ),
            (
                "ENTITLEMENT_CREATE",
                Event::EntitlementCreate(EntitlementEvent {
                    entitlement: Entitlement {
                        id: snowflake("930"),
                        sku_id: snowflake("931"),
                        application_id: snowflake("932"),
                        kind: 8,
                        deleted: false,
                        ..Entitlement::default()
                    },
                    raw: raw("ENTITLEMENT_CREATE"),
                }),
            ),
            (
                "SUBSCRIPTION_CREATE",
                Event::SubscriptionCreate(SubscriptionEvent {
                    subscription: Subscription {
                        id: snowflake("940"),
                        user_id: snowflake("941"),
                        current_period_start: "2026-04-01T00:00:00Z".to_string(),
                        current_period_end: "2026-05-01T00:00:00Z".to_string(),
                        status: 0,
                        ..Subscription::default()
                    },
                    raw: raw("SUBSCRIPTION_CREATE"),
                }),
            ),
            (
                "INTEGRATION_CREATE",
                Event::IntegrationCreate(IntegrationEvent {
                    guild_id: Some(snowflake("942")),
                    integration: Integration {
                        id: snowflake("943"),
                        name: "integration".to_string(),
                        kind: "discord".to_string(),
                        account: IntegrationAccount {
                            id: "account".to_string(),
                            name: "account".to_string(),
                        },
                        ..Integration::default()
                    },
                    raw: raw("INTEGRATION_CREATE"),
                }),
            ),
            (
                "INTEGRATION_DELETE",
                Event::IntegrationDelete(IntegrationDeleteEvent {
                    id: Some(snowflake("944")),
                    guild_id: Some(snowflake("945")),
                    application_id: Some(snowflake("946")),
                    raw: raw("INTEGRATION_DELETE"),
                }),
            ),
            (
                "GUILD_SOUNDBOARD_SOUND_CREATE",
                Event::GuildSoundboardSoundCreate(SoundboardSoundEvent {
                    sound: SoundboardSound {
                        name: "quack".to_string(),
                        sound_id: snowflake("933"),
                        guild_id: Some(snowflake("934")),
                        volume: 1.0,
                        available: true,
                        ..SoundboardSound::default()
                    },
                    raw: raw("GUILD_SOUNDBOARD_SOUND_CREATE"),
                }),
            ),
            (
                "GUILD_SOUNDBOARD_SOUND_DELETE",
                Event::GuildSoundboardSoundDelete(SoundboardSoundDeleteEvent {
                    sound_id: snowflake("935"),
                    guild_id: snowflake("936"),
                    raw: raw("GUILD_SOUNDBOARD_SOUND_DELETE"),
                }),
            ),
            (
                "SOUNDBOARD_SOUNDS",
                Event::SoundboardSounds(SoundboardSoundsEvent {
                    guild_id: snowflake("937"),
                    soundboard_sounds: vec![SoundboardSound {
                        name: "quack".to_string(),
                        sound_id: snowflake("938"),
                        volume: 1.0,
                        available: true,
                        ..SoundboardSound::default()
                    }],
                    raw: raw("SOUNDBOARD_SOUNDS"),
                }),
            ),
            (
                "WEBHOOKS_UPDATE",
                Event::WebhooksUpdate(WebhooksUpdateEvent {
                    guild_id: Some(snowflake("94")),
                    channel_id: Some(snowflake("95")),
                    raw: raw("WEBHOOKS_UPDATE"),
                }),
            ),
            (
                "INVITE_CREATE",
                Event::InviteCreate(InviteEvent {
                    guild_id: Some(snowflake("96")),
                    channel_id: Some(snowflake("97")),
                    code: Some("invite-create".to_string()),
                    raw: raw("INVITE_CREATE"),
                }),
            ),
            (
                "INVITE_DELETE",
                Event::InviteDelete(InviteEvent {
                    guild_id: Some(snowflake("98")),
                    channel_id: Some(snowflake("99")),
                    code: Some("invite-delete".to_string()),
                    raw: raw("INVITE_DELETE"),
                }),
            ),
            (
                "MESSAGE_POLL_VOTE_ADD",
                Event::MessagePollVoteAdd(PollVoteEvent {
                    user_id: Some(snowflake("980")),
                    channel_id: Some(snowflake("981")),
                    message_id: Some(snowflake("982")),
                    guild_id: Some(snowflake("983")),
                    answer_id: Some(1),
                    raw: raw("MESSAGE_POLL_VOTE_ADD"),
                }),
            ),
            (
                "TYPING_START",
                Event::TypingStart(TypingStartEvent {
                    channel_id: Some(snowflake("100")),
                    guild_id: Some(snowflake("101")),
                    user_id: Some(snowflake("102")),
                    timestamp: Some(123),
                    raw: raw("TYPING_START"),
                }),
            ),
            (
                "PRESENCE_UPDATE",
                Event::PresenceUpdate(PresenceUpdateEvent {
                    user_id: Some(snowflake("103")),
                    guild_id: Some(snowflake("104")),
                    status: Some("idle".to_string()),
                    raw: raw("PRESENCE_UPDATE"),
                }),
            ),
            (
                "SOMETHING_NEW",
                Event::Unknown {
                    kind: "SOMETHING_NEW".to_string(),
                    raw: raw("SOMETHING_NEW"),
                },
            ),
        ];

        for (kind, event) in cases {
            assert_kind_and_raw(event, kind);
        }
    }

    #[test]
    fn decode_event_handles_entitlement_and_soundboard_events() {
        let entitlement = decode_event(
            "ENTITLEMENT_UPDATE",
            json!({
                "id": "1",
                "sku_id": "2",
                "application_id": "3",
                "type": 8,
                "deleted": false,
                "consumed": false
            }),
        )
        .unwrap();
        match entitlement {
            Event::EntitlementUpdate(event) => {
                assert_eq!(event.entitlement.sku_id.as_str(), "2");
                assert!(!event.entitlement.deleted);
            }
            other => panic!("unexpected event: {other:?}"),
        }

        let subscription = decode_event(
            "SUBSCRIPTION_UPDATE",
            json!({
                "id": "30",
                "user_id": "31",
                "sku_ids": ["32"],
                "entitlement_ids": ["33"],
                "current_period_start": "2026-04-01T00:00:00Z",
                "current_period_end": "2026-05-01T00:00:00Z",
                "status": 1
            }),
        )
        .unwrap();
        match subscription {
            Event::SubscriptionUpdate(event) => {
                assert_eq!(event.subscription.id.as_str(), "30");
                assert_eq!(event.subscription.status, 1);
            }
            other => panic!("unexpected event: {other:?}"),
        }

        let integration = decode_event(
            "INTEGRATION_CREATE",
            json!({
                "id": "40",
                "guild_id": "41",
                "name": "integration",
                "type": "discord",
                "enabled": true,
                "account": { "id": "acc", "name": "account" }
            }),
        )
        .unwrap();
        match integration {
            Event::IntegrationCreate(event) => {
                assert_eq!(event.guild_id.unwrap().as_str(), "41");
                assert_eq!(event.integration.id.as_str(), "40");
            }
            other => panic!("unexpected event: {other:?}"),
        }

        let integration_delete = decode_event(
            "INTEGRATION_DELETE",
            json!({
                "id": "40",
                "guild_id": "41",
                "application_id": "42"
            }),
        )
        .unwrap();
        match integration_delete {
            Event::IntegrationDelete(event) => {
                assert_eq!(event.id.unwrap().as_str(), "40");
                assert_eq!(event.application_id.unwrap().as_str(), "42");
            }
            other => panic!("unexpected event: {other:?}"),
        }

        let poll_vote = decode_event(
            "MESSAGE_POLL_VOTE_REMOVE",
            json!({
                "user_id": "50",
                "channel_id": "51",
                "message_id": "52",
                "guild_id": "53",
                "answer_id": 2
            }),
        )
        .unwrap();
        match poll_vote {
            Event::MessagePollVoteRemove(event) => {
                assert_eq!(event.user_id.unwrap().as_str(), "50");
                assert_eq!(event.answer_id, Some(2));
            }
            other => panic!("unexpected event: {other:?}"),
        }

        let sound = decode_event(
            "GUILD_SOUNDBOARD_SOUND_UPDATE",
            json!({
                "name": "quack",
                "sound_id": "10",
                "volume": 1.0,
                "guild_id": "20",
                "available": true
            }),
        )
        .unwrap();
        match sound {
            Event::GuildSoundboardSoundUpdate(event) => {
                assert_eq!(event.sound.sound_id.as_str(), "10");
                assert_eq!(event.sound.guild_id.unwrap().as_str(), "20");
            }
            other => panic!("unexpected event: {other:?}"),
        }

        let delete = decode_event(
            "GUILD_SOUNDBOARD_SOUND_DELETE",
            json!({
                "sound_id": "10",
                "guild_id": "20"
            }),
        )
        .unwrap();
        match delete {
            Event::GuildSoundboardSoundDelete(event) => {
                assert_eq!(event.sound_id.as_str(), "10");
                assert_eq!(event.guild_id.as_str(), "20");
            }
            other => panic!("unexpected event: {other:?}"),
        }

        let sounds = decode_event(
            "GUILD_SOUNDBOARD_SOUNDS_UPDATE",
            json!({
                "guild_id": "20",
                "soundboard_sounds": [{
                    "name": "quack",
                    "sound_id": "10",
                    "volume": 1.0,
                    "available": true
                }]
            }),
        )
        .unwrap();
        match sounds {
            Event::GuildSoundboardSoundsUpdate(event) => {
                assert_eq!(event.guild_id.as_str(), "20");
                assert_eq!(event.soundboard_sounds.len(), 1);
            }
            other => panic!("unexpected event: {other:?}"),
        }
    }
}
