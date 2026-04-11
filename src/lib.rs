//! discordrs - Standalone Discord bot framework with Components V2, Gateway, and HTTP client
//!
//! # Features
//! - `gateway` - Gateway WebSocket client, BotClient, EventHandler
//! - `interactions` - HTTP Interactions Endpoint with Ed25519 verification

pub mod bitfield;
pub mod builders;
pub mod cache;
pub mod collection;
#[cfg(feature = "collectors")]
pub mod collector;
pub mod command;
pub mod constants;
pub mod error;
pub mod event;
pub mod helpers;
pub mod http;
pub mod manager;
pub mod model;
pub mod parsers;
pub mod prelude;
pub mod response;
#[cfg(feature = "sharding")]
pub mod sharding;
pub mod types;
#[cfg(feature = "voice")]
pub mod voice;
#[cfg(feature = "voice")]
pub mod voice_runtime;
#[cfg(any(feature = "gateway", feature = "sharding"))]
pub mod ws;

#[cfg(feature = "gateway")]
pub mod gateway;

#[cfg(feature = "interactions")]
pub mod interactions;

pub use cache::{
    CacheHandle, ChannelManager, GuildManager, MemberManager, MessageManager, RoleManager,
};
pub use collection::Collection;
pub use command::{
    command_type, option_type, CommandDefinition, CommandOptionBuilder, MessageCommandBuilder,
    SlashCommandBuilder, UserCommandBuilder,
};
pub use error::{DiscordError, HttpError};
pub use event::{
    decode_event, ChannelEvent, Event, GuildDeleteEvent, GuildDeletePayload, GuildEvent,
    InteractionEvent, MemberEvent, MemberRemoveEvent, MemberRemovePayload, MessageDeleteEvent,
    MessageDeletePayload, MessageEvent, ReadyEvent, ReadyPayload, RoleDeleteEvent,
    RoleDeletePayload, RoleEvent, VoiceServerEvent, VoiceStateEvent,
};
pub use manager::CachedManager;
pub use model::{
    ApplicationCommand, ApplicationCommandOption, ApplicationCommandOptionChoice, Attachment,
    AutocompleteInteraction, Channel, ChatInputCommandInteraction, CommandInteractionData,
    CommandInteractionOption, ComponentInteraction, CreateDmChannel, CreateMessage, DiscordModel,
    GatewayBot, Guild, Interaction, InteractionCallbackResponse, InteractionContextData, Member,
    Message, MessageContextMenuInteraction, ModalSubmitInteraction, PermissionsBitField, Role,
    SessionStartLimit, Snowflake, User, UserContextMenuInteraction, VoiceServerUpdate, VoiceState,
};
pub use response::{InteractionResponseBuilder, MessageBuilder};
#[cfg(feature = "sharding")]
pub use sharding::{
    ShardConfig, ShardInfo, ShardIpcMessage, ShardRuntimeState, ShardRuntimeStatus,
    ShardSupervisorEvent, ShardingManager,
};
pub use types::{ButtonConfig, Emoji, MediaGalleryItem, MediaInfo, SelectOption};
/// Backward-compatible alias. Prefer `DiscordError`.
#[deprecated(since = "0.4.0", note = "Use DiscordError instead")]
pub type Error = DiscordError;
pub use bitfield::{
    BitField, BitFieldFlags, IntentFlags, Intents, MessageFlagBits, MessageFlags, PermissionFlags,
    Permissions,
};
#[cfg(feature = "voice")]
pub use voice::{
    AudioPlayer, AudioTrack, VoiceConnectionConfig, VoiceConnectionState, VoiceConnectionStatus,
    VoiceEncryptionMode, VoiceEvent, VoiceGatewayCommand, VoiceGatewayHello, VoiceGatewayOpcode,
    VoiceGatewayReady, VoiceManager, VoiceSelectProtocolCommand, VoiceSpeakingCommand,
    VoiceSpeakingFlags, VoiceSpeakingState, VoiceTransportProtocol, VoiceTransportState,
    VoiceUdpDiscoveryPacket,
};
#[cfg(feature = "voice")]
pub use voice_runtime::{
    connect as connect_voice_runtime, VoiceRuntimeConfig, VoiceRuntimeHandle, VoiceRuntimeState,
    VoiceSessionDescription,
};
#[cfg(any(feature = "gateway", feature = "sharding"))]
pub use ws::{GatewayCompression, GatewayConnectionConfig, GatewayEncoding};

pub use builders::{
    create_container, create_default_buttons, ActionRowBuilder, ButtonBuilder, CheckboxBuilder,
    CheckboxGroupBuilder, ComponentsV2Message, ContainerBuilder, EmbedBuilder, FileBuilder,
    FileUploadBuilder, LabelBuilder, MediaGalleryBuilder, ModalBuilder, RadioGroupBuilder,
    SectionBuilder, SelectMenuBuilder, SeparatorBuilder, TextDisplayBuilder, TextInputBuilder,
    ThumbnailBuilder,
};

pub use parsers::{
    parse_interaction, parse_interaction_context, parse_modal_submission, parse_raw_interaction,
    InteractionContext, RawInteraction, V2ModalComponent, V2ModalSubmission,
};

pub use constants::{
    button_style, component_type, gateway_intents, separator_spacing, text_input_style,
    MESSAGE_FLAG_IS_COMPONENTS_V2,
};

pub use http::{DiscordHttpClient, RestClient};

pub use helpers::{
    defer_and_followup_container, defer_interaction, defer_update_interaction,
    delete_followup_response, delete_original_response, edit_message_with_container,
    edit_original_response, followup_message, followup_with_container, get_original_response,
    launch_activity, respond_component_with_components_v2, respond_component_with_container,
    respond_modal_with_container, respond_to_interaction, respond_with_autocomplete_choices,
    respond_with_components_v2, respond_with_container, respond_with_message, respond_with_modal,
    respond_with_modal_typed, send_components_v2, send_container_message, send_message,
    send_to_channel, update_component_with_container, update_interaction_message,
};

#[cfg(all(feature = "gateway", feature = "sharding"))]
pub use gateway::ShardSupervisor;
#[cfg(feature = "gateway")]
pub use gateway::{
    BotClient, BotClientBuilder, Client, ClientBuilder, Context, EventHandler, ShardMessenger,
    TypeMap,
};

#[cfg(feature = "interactions")]
pub use interactions::{
    interactions_endpoint, try_interactions_endpoint, try_typed_interactions_endpoint,
    typed_interactions_endpoint, verify_discord_signature, InteractionHandler, InteractionResponse,
    TypedInteractionHandler,
};

#[cfg(feature = "collectors")]
pub use collector::{
    CollectorHub, ComponentCollector, InteractionCollector, MessageCollector, ModalCollector,
};
