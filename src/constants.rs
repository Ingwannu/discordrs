pub mod component_type {
    pub const ACTION_ROW: u8 = 1;
    pub const BUTTON: u8 = 2;
    pub const STRING_SELECT: u8 = 3;
    pub const TEXT_INPUT: u8 = 4;
    pub const USER_SELECT: u8 = 5;
    pub const ROLE_SELECT: u8 = 6;
    pub const MENTIONABLE_SELECT: u8 = 7;
    pub const CHANNEL_SELECT: u8 = 8;
    pub const SECTION: u8 = 9;
    pub const TEXT_DISPLAY: u8 = 10;
    pub const THUMBNAIL: u8 = 11;
    pub const MEDIA_GALLERY: u8 = 12;
    pub const FILE: u8 = 13;
    pub const SEPARATOR: u8 = 14;
    pub const CONTENT_INVENTORY_ENTRY: u8 = 16;
    pub const CONTAINER: u8 = 17;
    pub const LABEL: u8 = 18;
    pub const FILE_UPLOAD: u8 = 19;
    pub const RADIO_GROUP: u8 = 21;
    pub const CHECKBOX_GROUP: u8 = 22;
    pub const CHECKBOX: u8 = 23;
}

pub mod button_style {
    pub const PRIMARY: u8 = 1;
    pub const SECONDARY: u8 = 2;
    pub const SUCCESS: u8 = 3;
    pub const DANGER: u8 = 4;
    pub const LINK: u8 = 5;
}

pub mod separator_spacing {
    pub const SMALL: u8 = 1;
    pub const LARGE: u8 = 2;
}

pub mod text_input_style {
    pub const SHORT: u8 = 1;
    pub const PARAGRAPH: u8 = 2;
}

pub mod gateway_intents {
    pub const GUILDS: u64 = 1 << 0;
    pub const GUILD_MEMBERS: u64 = 1 << 1;
    pub const GUILD_MODERATION: u64 = 1 << 2;
    pub const GUILD_EMOJIS_AND_STICKERS: u64 = 1 << 3;
    pub const GUILD_INTEGRATIONS: u64 = 1 << 4;
    pub const GUILD_WEBHOOKS: u64 = 1 << 5;
    pub const GUILD_INVITES: u64 = 1 << 6;
    pub const GUILD_VOICE_STATES: u64 = 1 << 7;
    pub const GUILD_PRESENCES: u64 = 1 << 8;
    pub const GUILD_MESSAGES: u64 = 1 << 9;
    pub const GUILD_MESSAGE_REACTIONS: u64 = 1 << 10;
    pub const GUILD_MESSAGE_TYPING: u64 = 1 << 11;
    pub const DIRECT_MESSAGES: u64 = 1 << 12;
    pub const DIRECT_MESSAGE_REACTIONS: u64 = 1 << 13;
    pub const DIRECT_MESSAGE_TYPING: u64 = 1 << 14;
    pub const MESSAGE_CONTENT: u64 = 1 << 15;
    pub const GUILD_SCHEDULED_EVENTS: u64 = 1 << 16;
    pub const AUTO_MODERATION_CONFIGURATION: u64 = 1 << 20;
    pub const AUTO_MODERATION_EXECUTION: u64 = 1 << 21;
}

pub const MESSAGE_FLAG_IS_COMPONENTS_V2: u64 = 1 << 15;
