pub use crate::bitfield::{BitField, Intents, MessageFlags, Permissions};
pub use crate::builders::{
    create_container, ActionRowBuilder, ButtonBuilder, ComponentsV2Message, ContainerBuilder,
    EmbedBuilder, ModalBuilder, SelectMenuBuilder, TextInputBuilder,
};
#[cfg(feature = "collectors")]
pub use crate::collector::{
    CollectorHub, ComponentCollector, InteractionCollector, MessageCollector, ModalCollector,
};
pub use crate::command::{
    command_type, option_type, CommandDefinition, CommandOptionBuilder, MessageCommandBuilder,
    SlashCommandBuilder, UserCommandBuilder,
};
pub use crate::constants::{button_style, gateway_intents, text_input_style};
pub use crate::error::DiscordError;
pub use crate::event::Event;
#[cfg(all(feature = "gateway", feature = "sharding"))]
pub use crate::gateway::ShardSupervisor;
#[cfg(feature = "gateway")]
pub use crate::gateway::{Client, Context, EventHandler, ShardMessenger};
pub use crate::helpers::{
    defer_interaction, defer_update_interaction, delete_followup_response,
    delete_original_response, edit_original_response, followup_message, get_original_response,
    launch_activity, respond_to_interaction, respond_with_autocomplete_choices,
    respond_with_message, respond_with_modal_typed, send_message, update_interaction_message,
};
pub use crate::http::RestClient;
pub use crate::model::{
    ApplicationCommandOptionChoice, CommandInteractionData, CommandInteractionOption,
    CreateMessage, GatewayBot, Interaction, InteractionCallbackResponse, InteractionContextData,
    Message, PermissionsBitField, SessionStartLimit, Snowflake, VoiceServerUpdate, VoiceState,
};
pub use crate::response::{InteractionResponseBuilder, MessageBuilder};
#[cfg(feature = "voice")]
pub use crate::voice::{
    AudioTrack, VoiceConnectionConfig, VoiceEncryptionMode, VoiceGatewayCommand, VoiceManager,
    VoiceSpeakingFlags, VoiceTransportState, VoiceUdpDiscoveryPacket,
};
#[cfg(feature = "voice")]
pub use crate::voice_runtime::{
    connect as connect_voice_runtime, VoiceRuntimeConfig, VoiceRuntimeHandle,
};

#[cfg(test)]
mod tests {
    use crate::prelude::{
        button_style, gateway_intents, ButtonBuilder, MessageBuilder, SlashCommandBuilder,
    };

    #[test]
    fn prelude_surfaces_common_gateway_and_command_types() {
        let _intents = gateway_intents::GUILDS | gateway_intents::GUILD_MESSAGES;
        let _button = ButtonBuilder::new().style(button_style::PRIMARY);
        let _command =
            SlashCommandBuilder::new("ping", "Ping").string_option("target", "Target", false);
        let _message = MessageBuilder::new().content("hello");
    }
}
