//! discordrs - Standalone Discord bot framework with Components V2, Gateway, and HTTP client
//!
//! # Features
//! - `gateway` - Gateway WebSocket client, BotClient, EventHandler
//! - `interactions` - HTTP Interactions Endpoint with Ed25519 verification

pub mod builders;
pub mod constants;
pub mod helpers;
pub mod http;
pub mod parsers;
pub mod types;

#[cfg(feature = "gateway")]
pub mod gateway;

#[cfg(feature = "interactions")]
pub mod interactions;

pub use types::{ButtonConfig, Emoji, Error, MediaGalleryItem, MediaInfo, SelectOption};

pub use builders::{
    create_container, create_default_buttons, ActionRowBuilder, ButtonBuilder,
    CheckboxBuilder, CheckboxGroupBuilder, ComponentsV2Message, ContainerBuilder, FileBuilder,
    FileUploadBuilder, LabelBuilder, MediaGalleryBuilder, ModalBuilder, RadioGroupBuilder,
    SectionBuilder, SelectMenuBuilder, SeparatorBuilder, TextDisplayBuilder, TextInputBuilder,
    ThumbnailBuilder,
};

pub use parsers::{
    parse_interaction_context, parse_modal_submission, parse_raw_interaction,
    InteractionContext, RawInteraction, V2ModalComponent, V2ModalSubmission,
};

pub use constants::{
    button_style, component_type, gateway_intents, separator_spacing, text_input_style,
    MESSAGE_FLAG_IS_COMPONENTS_V2,
};

pub use http::DiscordHttpClient;

pub use helpers::{
    defer_and_followup_container, edit_message_with_container, followup_with_container,
    respond_component_with_components_v2, respond_component_with_container,
    respond_modal_with_container, respond_with_components_v2, respond_with_container,
    respond_with_modal, send_components_v2, send_container_message, send_to_channel,
    update_component_with_container,
};

#[cfg(feature = "gateway")]
pub use gateway::{BotClient, BotClientBuilder, Context, EventHandler, TypeMap};

#[cfg(feature = "interactions")]
pub use interactions::{
    interactions_endpoint, verify_discord_signature, InteractionHandler, InteractionResponse,
};
