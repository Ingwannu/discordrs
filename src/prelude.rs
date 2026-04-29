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
    PrimaryEntryPointCommandBuilder, SlashCommandBuilder, UserCommandBuilder,
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
    delete_original_response, edit_followup_response, edit_original_response, followup_message,
    get_original_response, launch_activity, respond_to_interaction,
    respond_with_autocomplete_choices, respond_with_message, respond_with_modal_typed,
    send_message, update_interaction_message,
};
pub use crate::http::RestClient;
pub use crate::model::{
    Activity, ActivityAssets, ActivityButton, ActivityParty, ActivitySecrets, ActivityTimestamps,
    ActivityType, Application, ApplicationCommandHandlerType, ApplicationCommandOptionChoice,
    ApplicationIntegrationType, ApplicationRoleConnectionMetadata, ArchivedThreadsQuery,
    AutoModerationRule, BulkGuildBanRequest, BulkGuildBanResponse, CommandInteractionData,
    CommandInteractionOption, CreateMessage, CreateTestEntitlement, CurrentUserGuild, Entitlement,
    EntitlementQuery, GatewayBot, GuildPreview, GuildPruneCount, GuildScheduledEventRecurrenceRule,
    Integration, Interaction, InteractionCallbackResponse, InteractionContextData,
    InteractionContextType, JoinedArchivedThreadsQuery, Message, PermissionsBitField,
    PollAnswerVoters, RequestGuildMembers, SessionStartLimit, Sku, Snowflake, SoundboardSound,
    SoundboardSoundList, Subscription, SubscriptionQuery, ThreadListResponse, ThreadMember,
    ThreadMemberQuery, UpdatePresence, VanityUrl, VoiceRegion, VoiceServerUpdate, VoiceState,
};
pub use crate::oauth2::{
    OAuth2AuthorizationRequest, OAuth2Client, OAuth2CodeExchange, OAuth2RefreshToken, OAuth2Scope,
    OAuth2TokenResponse,
};
pub use crate::response::{InteractionResponseBuilder, MessageBuilder};
#[cfg(feature = "voice")]
pub use crate::voice::{
    AudioTrack, VoiceConnectionConfig, VoiceEncryptionMode, VoiceGatewayCommand, VoiceManager,
    VoiceSpeakingFlags, VoiceTransportState, VoiceUdpDiscoveryPacket,
};
#[cfg(feature = "voice")]
pub use crate::voice_runtime::{
    connect as connect_voice_runtime, VoiceDaveFrame, VoiceDaveFrameDecryptor, VoiceDecodedPacket,
    VoiceOpusDecoder, VoiceOpusFrame, VoiceOutboundPacket, VoiceOutboundRtpState,
    VoiceRawUdpPacket, VoiceReceivedPacket, VoiceRuntimeConfig, VoiceRuntimeHandle,
};
#[cfg(all(feature = "voice", feature = "voice-encode"))]
pub use crate::voice_runtime::{AudioMixer, AudioSource, PcmFrame, VoiceOpusEncoder};
#[cfg(all(feature = "voice", feature = "dave"))]
pub use crate::voice_runtime::{VoiceDaveFrameEncryptor, VoiceDaveyDecryptor, VoiceDaveySession};

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
