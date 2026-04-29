use std::fmt;
use std::marker::PhantomData;
use std::ops::{BitAnd, BitAndAssign, BitOr, BitOrAssign, BitXor, BitXorAssign, Not, Sub};

use serde::de::{self, Visitor};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// Trait defining the flags for a specific BitField type.
///
/// Implement this for a zero-sized marker type to create a typed BitField.
pub trait BitFieldFlags:
    Copy + Clone + fmt::Debug + PartialEq + Eq + Send + Sync + 'static
{
    /// All defined flag bits as `(bit_value, name)` pairs.
    const BITS: &'static [(u64, &'static str)];
}

/// A generic bitfield type with named flags, paralleling discord.js's `BitField`.
///
/// # Examples
/// ```
/// use discordrs::bitfield::{BitField, BitFieldFlags};
///
/// #[derive(Copy, Clone, Debug, PartialEq, Eq)]
/// struct MyFlags;
/// impl BitFieldFlags for MyFlags {
///     const BITS: &'static [(u64, &'static str)] = &[
///         (1 << 0, "READ"),
///         (1 << 1, "WRITE"),
///     ];
/// }
///
/// type MyBitField = BitField<MyFlags>;
/// let bf = MyBitField::from_bits(1 << 0);
/// assert!(bf.contains(1 << 0));
/// assert!(!bf.contains(1 << 1));
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct BitField<T: BitFieldFlags>(u64, PhantomData<T>);

impl<T: BitFieldFlags> BitField<T> {
    /// Create a BitField from raw bits.
    pub const fn from_bits(bits: u64) -> Self {
        Self(bits, PhantomData)
    }

    /// Returns the raw bit value.
    pub const fn bits(self) -> u64 {
        self.0
    }

    /// Returns `true` if the specified flag bit is set.
    pub const fn contains(self, flag: u64) -> bool {
        (self.0 & flag) == flag
    }

    /// Alias for `contains`.
    pub const fn is_set(self, flag: u64) -> bool {
        self.contains(flag)
    }

    /// Sets the specified flag bit, returning a new BitField.
    pub const fn add(self, flag: u64) -> Self {
        Self(self.0 | flag, PhantomData)
    }

    /// Clears the specified flag bit, returning a new BitField.
    pub const fn remove(self, flag: u64) -> Self {
        Self(self.0 & !flag, PhantomData)
    }

    /// Returns a new BitField with the union of this and another.
    pub const fn merge(self, other: Self) -> Self {
        Self(self.0 | other.0, PhantomData)
    }

    /// Returns `true` if no bits are set.
    pub const fn is_empty(self) -> bool {
        self.0 == 0
    }

    /// Returns the bits that are set in `other` but not in `self`.
    pub const fn missing(self, other: Self) -> Self {
        Self((!self.0) & other.0, PhantomData)
    }

    /// Returns a BitField with all defined bits set.
    pub fn all() -> Self {
        let mut bits = 0u64;
        let mut i = 0;
        while i < T::BITS.len() {
            bits |= T::BITS[i].0;
            i += 1;
        }
        Self(bits, PhantomData)
    }

    /// Returns `true` if any of the specified flags are set.
    pub const fn any(self, flags: u64) -> bool {
        (self.0 & flags) != 0
    }

    /// Returns `true` if all of the specified flags are set.
    pub const fn has_all(self, flags: u64) -> bool {
        (self.0 & flags) == flags
    }

    /// Returns the names of all set flags.
    pub fn flag_names(self) -> Vec<&'static str> {
        T::BITS
            .iter()
            .filter(|(bit, _)| self.contains(*bit))
            .map(|(_, name)| *name)
            .collect()
    }

    /// Serialize this bitfield as a string (Discord API format).
    pub fn to_api_string(self) -> String {
        self.0.to_string()
    }
}

impl<T: BitFieldFlags> Default for BitField<T> {
    fn default() -> Self {
        Self(0, PhantomData)
    }
}

// --- Operator overloads ---

impl<T: BitFieldFlags> BitOr for BitField<T> {
    type Output = Self;
    fn bitor(self, rhs: Self) -> Self::Output {
        Self(self.0 | rhs.0, PhantomData)
    }
}

impl<T: BitFieldFlags> BitOr<u64> for BitField<T> {
    type Output = Self;
    fn bitor(self, rhs: u64) -> Self::Output {
        Self(self.0 | rhs, PhantomData)
    }
}

impl<T: BitFieldFlags> BitOrAssign for BitField<T> {
    fn bitor_assign(&mut self, rhs: Self) {
        self.0 |= rhs.0;
    }
}

impl<T: BitFieldFlags> BitOrAssign<u64> for BitField<T> {
    fn bitor_assign(&mut self, rhs: u64) {
        self.0 |= rhs;
    }
}

impl<T: BitFieldFlags> BitAnd for BitField<T> {
    type Output = Self;
    fn bitand(self, rhs: Self) -> Self::Output {
        Self(self.0 & rhs.0, PhantomData)
    }
}

impl<T: BitFieldFlags> BitAnd<u64> for BitField<T> {
    type Output = Self;
    fn bitand(self, rhs: u64) -> Self::Output {
        Self(self.0 & rhs, PhantomData)
    }
}

impl<T: BitFieldFlags> BitAndAssign for BitField<T> {
    fn bitand_assign(&mut self, rhs: Self) {
        self.0 &= rhs.0;
    }
}

impl<T: BitFieldFlags> BitAndAssign<u64> for BitField<T> {
    fn bitand_assign(&mut self, rhs: u64) {
        self.0 &= rhs;
    }
}

impl<T: BitFieldFlags> BitXor for BitField<T> {
    type Output = Self;
    fn bitxor(self, rhs: Self) -> Self::Output {
        Self(self.0 ^ rhs.0, PhantomData)
    }
}

impl<T: BitFieldFlags> BitXorAssign for BitField<T> {
    fn bitxor_assign(&mut self, rhs: Self) {
        self.0 ^= rhs.0;
    }
}

impl<T: BitFieldFlags> Sub for BitField<T> {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self::Output {
        Self(self.0 & !rhs.0, PhantomData)
    }
}

impl<T: BitFieldFlags> Sub<u64> for BitField<T> {
    type Output = Self;
    fn sub(self, rhs: u64) -> Self::Output {
        Self(self.0 & !rhs, PhantomData)
    }
}

impl<T: BitFieldFlags> Not for BitField<T> {
    type Output = Self;
    fn not(self) -> Self::Output {
        Self(!self.0, PhantomData)
    }
}

impl<T: BitFieldFlags> From<u64> for BitField<T> {
    fn from(bits: u64) -> Self {
        Self(bits, PhantomData)
    }
}

impl<T: BitFieldFlags> From<BitField<T>> for u64 {
    fn from(bf: BitField<T>) -> u64 {
        bf.0
    }
}

// --- Serde: string-based serialization (Discord API format) ---

impl<T: BitFieldFlags> Serialize for BitField<T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.0.to_string())
    }
}

impl<'de, T: BitFieldFlags> Deserialize<'de> for BitField<T> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct BitFieldVisitor<T>(PhantomData<T>);

        impl<'de, T: BitFieldFlags> Visitor<'de> for BitFieldVisitor<T> {
            type Value = BitField<T>;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("a bitfield encoded as a string or integer")
            }

            fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(BitField::from_bits(value))
            }

            fn visit_i64<E>(self, value: i64) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(BitField::from_bits(value as u64))
            }

            fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                value
                    .parse::<u64>()
                    .map(BitField::from_bits)
                    .map_err(|e| E::custom(format!("invalid bitfield value: {e}")))
            }

            fn visit_string<E>(self, value: String) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                self.visit_str(&value)
            }
        }

        deserializer.deserialize_any(BitFieldVisitor(PhantomData))
    }
}

impl<T: BitFieldFlags> fmt::Display for BitField<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

// --- Concrete BitField flag definitions ---

/// Gateway Intents flags.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct IntentFlags;

impl BitFieldFlags for IntentFlags {
    const BITS: &'static [(u64, &'static str)] = &[
        (1 << 0, "GUILDS"),
        (1 << 1, "GUILD_MEMBERS"),
        (1 << 2, "GUILD_MODERATION"),
        (1 << 3, "GUILD_EMOJIS_AND_STICKERS"),
        (1 << 4, "GUILD_INTEGRATIONS"),
        (1 << 5, "GUILD_WEBHOOKS"),
        (1 << 6, "GUILD_INVITES"),
        (1 << 7, "GUILD_VOICE_STATES"),
        (1 << 8, "GUILD_PRESENCES"),
        (1 << 9, "GUILD_MESSAGES"),
        (1 << 10, "GUILD_MESSAGE_REACTIONS"),
        (1 << 11, "GUILD_MESSAGE_TYPING"),
        (1 << 12, "DIRECT_MESSAGES"),
        (1 << 13, "DIRECT_MESSAGE_REACTIONS"),
        (1 << 14, "DIRECT_MESSAGE_TYPING"),
        (1 << 15, "MESSAGE_CONTENT"),
        (1 << 16, "GUILD_SCHEDULED_EVENTS"),
        (1 << 20, "AUTO_MODERATION_CONFIGURATION"),
        (1 << 21, "AUTO_MODERATION_EXECUTION"),
    ];
}

/// Gateway Intents type alias.
pub type Intents = BitField<IntentFlags>;

/// Convenience constants for Intents, matching the old `gateway_intents` module.
pub mod gateway_intents {
    use super::{BitField, IntentFlags};

    pub const GUILDS: BitField<IntentFlags> = BitField::from_bits(1 << 0);
    pub const GUILD_MEMBERS: BitField<IntentFlags> = BitField::from_bits(1 << 1);
    pub const GUILD_MODERATION: BitField<IntentFlags> = BitField::from_bits(1 << 2);
    pub const GUILD_EMOJIS_AND_STICKERS: BitField<IntentFlags> = BitField::from_bits(1 << 3);
    pub const GUILD_INTEGRATIONS: BitField<IntentFlags> = BitField::from_bits(1 << 4);
    pub const GUILD_WEBHOOKS: BitField<IntentFlags> = BitField::from_bits(1 << 5);
    pub const GUILD_INVITES: BitField<IntentFlags> = BitField::from_bits(1 << 6);
    pub const GUILD_VOICE_STATES: BitField<IntentFlags> = BitField::from_bits(1 << 7);
    pub const GUILD_PRESENCES: BitField<IntentFlags> = BitField::from_bits(1 << 8);
    pub const GUILD_MESSAGES: BitField<IntentFlags> = BitField::from_bits(1 << 9);
    pub const GUILD_MESSAGE_REACTIONS: BitField<IntentFlags> = BitField::from_bits(1 << 10);
    pub const GUILD_MESSAGE_TYPING: BitField<IntentFlags> = BitField::from_bits(1 << 11);
    pub const DIRECT_MESSAGES: BitField<IntentFlags> = BitField::from_bits(1 << 12);
    pub const DIRECT_MESSAGE_REACTIONS: BitField<IntentFlags> = BitField::from_bits(1 << 13);
    pub const DIRECT_MESSAGE_TYPING: BitField<IntentFlags> = BitField::from_bits(1 << 14);
    pub const MESSAGE_CONTENT: BitField<IntentFlags> = BitField::from_bits(1 << 15);
    pub const GUILD_SCHEDULED_EVENTS: BitField<IntentFlags> = BitField::from_bits(1 << 16);
    pub const AUTO_MODERATION_CONFIGURATION: BitField<IntentFlags> = BitField::from_bits(1 << 20);
    pub const AUTO_MODERATION_EXECUTION: BitField<IntentFlags> = BitField::from_bits(1 << 21);

    /// All non-privileged intents (safe for default use).
    pub const NON_PRIVILEGED: BitField<IntentFlags> = BitField::from_bits(
        (1 << 0)
            | (1 << 2)
            | (1 << 3)
            | (1 << 4)
            | (1 << 5)
            | (1 << 6)
            | (1 << 7)
            | (1 << 9)
            | (1 << 10)
            | (1 << 11)
            | (1 << 12)
            | (1 << 13)
            | (1 << 14)
            | (1 << 16)
            | (1 << 20)
            | (1 << 21),
    );

    /// All privileged intents (requires enabling in Developer Portal).
    pub const PRIVILEGED: BitField<IntentFlags> =
        BitField::from_bits((1 << 1) | (1 << 8) | (1 << 15));
}

/// Permission flags.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct PermissionFlags;

impl BitFieldFlags for PermissionFlags {
    const BITS: &'static [(u64, &'static str)] = &[
        (1 << 0, "CREATE_INSTANT_INVITE"),
        (1 << 1, "KICK_MEMBERS"),
        (1 << 2, "BAN_MEMBERS"),
        (1 << 3, "ADMINISTRATOR"),
        (1 << 4, "MANAGE_CHANNELS"),
        (1 << 5, "MANAGE_GUILD"),
        (1 << 6, "ADD_REACTIONS"),
        (1 << 7, "VIEW_AUDIT_LOG"),
        (1 << 8, "PRIORITY_SPEAKER"),
        (1 << 9, "STREAM"),
        (1 << 10, "VIEW_CHANNEL"),
        (1 << 11, "SEND_MESSAGES"),
        (1 << 12, "SEND_TTS_MESSAGES"),
        (1 << 13, "MANAGE_MESSAGES"),
        (1 << 14, "EMBED_LINKS"),
        (1 << 15, "ATTACH_FILES"),
        (1 << 16, "READ_MESSAGE_HISTORY"),
        (1 << 17, "MENTION_EVERYONE"),
        (1 << 18, "USE_EXTERNAL_EMOJIS"),
        (1 << 19, "VIEW_GUILD_INSIGHTS"),
        (1 << 20, "CONNECT"),
        (1 << 21, "SPEAK"),
        (1 << 22, "MUTE_MEMBERS"),
        (1 << 23, "DEAFEN_MEMBERS"),
        (1 << 24, "MOVE_MEMBERS"),
        (1 << 25, "USE_VAD"),
        (1 << 26, "CHANGE_NICKNAME"),
        (1 << 27, "MANAGE_NICKNAMES"),
        (1 << 28, "MANAGE_ROLES"),
        (1 << 29, "MANAGE_WEBHOOKS"),
        (1 << 30, "MANAGE_EMOJIS_AND_STICKERS"),
        (1 << 31, "USE_APPLICATION_COMMANDS"),
        (1 << 32, "REQUEST_TO_SPEAK"),
        (1 << 33, "MANAGE_EVENTS"),
        (1 << 34, "MANAGE_THREADS"),
        (1 << 35, "CREATE_PUBLIC_THREADS"),
        (1 << 36, "CREATE_PRIVATE_THREADS"),
        (1 << 37, "USE_EXTERNAL_STICKERS"),
        (1 << 38, "SEND_MESSAGES_IN_THREADS"),
        (1 << 39, "USE_EMBEDDED_ACTIVITIES"),
        (1 << 40, "MODERATE_MEMBERS"),
        (1 << 42, "VIEW_CREATOR_MONETIZATION_ANALYTICS"),
        (1 << 43, "USE_SOUNDBOARD"),
        (1 << 44, "CREATE_GUILD_EXPRESSIONS"),
        (1 << 45, "CREATE_EVENTS"),
        (1 << 46, "USE_EXTERNAL_SOUNDS"),
        (1 << 47, "SEND_VOICE_MESSAGES"),
        (1 << 48, "USE_CLYDE_AI"),
        (1 << 49, "SET_VOICE_CHANNEL_STATUS"),
        (1 << 50, "SEND_POLLS"),
        (1 << 52, "USE_EXTERNAL_APPS"),
    ];
}

/// Permissions type alias.
pub type Permissions = BitField<PermissionFlags>;

/// Convenience constants for Permissions.
pub mod permissions {
    use super::{BitField, PermissionFlags};

    pub const CREATE_INSTANT_INVITE: BitField<PermissionFlags> = BitField::from_bits(1 << 0);
    pub const KICK_MEMBERS: BitField<PermissionFlags> = BitField::from_bits(1 << 1);
    pub const BAN_MEMBERS: BitField<PermissionFlags> = BitField::from_bits(1 << 2);
    pub const ADMINISTRATOR: BitField<PermissionFlags> = BitField::from_bits(1 << 3);
    pub const MANAGE_CHANNELS: BitField<PermissionFlags> = BitField::from_bits(1 << 4);
    pub const MANAGE_GUILD: BitField<PermissionFlags> = BitField::from_bits(1 << 5);
    pub const ADD_REACTIONS: BitField<PermissionFlags> = BitField::from_bits(1 << 6);
    pub const VIEW_AUDIT_LOG: BitField<PermissionFlags> = BitField::from_bits(1 << 7);
    pub const PRIORITY_SPEAKER: BitField<PermissionFlags> = BitField::from_bits(1 << 8);
    pub const STREAM: BitField<PermissionFlags> = BitField::from_bits(1 << 9);
    pub const VIEW_CHANNEL: BitField<PermissionFlags> = BitField::from_bits(1 << 10);
    pub const SEND_MESSAGES: BitField<PermissionFlags> = BitField::from_bits(1 << 11);
    pub const SEND_TTS_MESSAGES: BitField<PermissionFlags> = BitField::from_bits(1 << 12);
    pub const MANAGE_MESSAGES: BitField<PermissionFlags> = BitField::from_bits(1 << 13);
    pub const EMBED_LINKS: BitField<PermissionFlags> = BitField::from_bits(1 << 14);
    pub const ATTACH_FILES: BitField<PermissionFlags> = BitField::from_bits(1 << 15);
    pub const READ_MESSAGE_HISTORY: BitField<PermissionFlags> = BitField::from_bits(1 << 16);
    pub const MENTION_EVERYONE: BitField<PermissionFlags> = BitField::from_bits(1 << 17);
    pub const USE_EXTERNAL_EMOJIS: BitField<PermissionFlags> = BitField::from_bits(1 << 18);
    pub const VIEW_GUILD_INSIGHTS: BitField<PermissionFlags> = BitField::from_bits(1 << 19);
    pub const CONNECT: BitField<PermissionFlags> = BitField::from_bits(1 << 20);
    pub const SPEAK: BitField<PermissionFlags> = BitField::from_bits(1 << 21);
    pub const MUTE_MEMBERS: BitField<PermissionFlags> = BitField::from_bits(1 << 22);
    pub const DEAFEN_MEMBERS: BitField<PermissionFlags> = BitField::from_bits(1 << 23);
    pub const MOVE_MEMBERS: BitField<PermissionFlags> = BitField::from_bits(1 << 24);
    pub const USE_VAD: BitField<PermissionFlags> = BitField::from_bits(1 << 25);
    pub const CHANGE_NICKNAME: BitField<PermissionFlags> = BitField::from_bits(1 << 26);
    pub const MANAGE_NICKNAMES: BitField<PermissionFlags> = BitField::from_bits(1 << 27);
    pub const MANAGE_ROLES: BitField<PermissionFlags> = BitField::from_bits(1 << 28);
    pub const MANAGE_WEBHOOKS: BitField<PermissionFlags> = BitField::from_bits(1 << 29);
    pub const MANAGE_EMOJIS_AND_STICKERS: BitField<PermissionFlags> = BitField::from_bits(1 << 30);
    pub const USE_APPLICATION_COMMANDS: BitField<PermissionFlags> = BitField::from_bits(1 << 31);
    pub const REQUEST_TO_SPEAK: BitField<PermissionFlags> = BitField::from_bits(1 << 32);
    pub const MANAGE_EVENTS: BitField<PermissionFlags> = BitField::from_bits(1 << 33);
    pub const MANAGE_THREADS: BitField<PermissionFlags> = BitField::from_bits(1 << 34);
    pub const CREATE_PUBLIC_THREADS: BitField<PermissionFlags> = BitField::from_bits(1 << 35);
    pub const CREATE_PRIVATE_THREADS: BitField<PermissionFlags> = BitField::from_bits(1 << 36);
    pub const USE_EXTERNAL_STICKERS: BitField<PermissionFlags> = BitField::from_bits(1 << 37);
    pub const SEND_MESSAGES_IN_THREADS: BitField<PermissionFlags> = BitField::from_bits(1 << 38);
    pub const USE_EMBEDDED_ACTIVITIES: BitField<PermissionFlags> = BitField::from_bits(1 << 39);
    pub const MODERATE_MEMBERS: BitField<PermissionFlags> = BitField::from_bits(1 << 40);
    pub const SEND_POLLS: BitField<PermissionFlags> = BitField::from_bits(1 << 50);
}

/// Message flags.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct MessageFlagBits;

impl BitFieldFlags for MessageFlagBits {
    const BITS: &'static [(u64, &'static str)] = &[
        (1 << 0, "CROSSPOSTED"),
        (1 << 1, "IS_CROSSPOST"),
        (1 << 2, "SUPPRESS_EMBEDS"),
        (1 << 3, "SOURCE_MESSAGE_DELETED"),
        (1 << 4, "URGENT"),
        (1 << 5, "HAS_THREAD"),
        (1 << 6, "EPHEMERAL"),
        (1 << 7, "LOADING"),
        (1 << 8, "FAILED_TO_MENTION_SOME_ROLES_IN_THREAD"),
        (1 << 10, "SUPPRESS_NOTIFICATIONS"),
        (1 << 12, "IS_VOICE_MESSAGE"),
        (1 << 15, "IS_COMPONENTS_V2"),
    ];
}

/// MessageFlags type alias.
pub type MessageFlags = BitField<MessageFlagBits>;

/// Convenience constants for MessageFlags.
pub mod message_flags {
    use super::{BitField, MessageFlagBits};

    pub const CROSSPOSTED: BitField<MessageFlagBits> = BitField::from_bits(1 << 0);
    pub const IS_CROSSPOST: BitField<MessageFlagBits> = BitField::from_bits(1 << 1);
    pub const SUPPRESS_EMBEDS: BitField<MessageFlagBits> = BitField::from_bits(1 << 2);
    pub const SOURCE_MESSAGE_DELETED: BitField<MessageFlagBits> = BitField::from_bits(1 << 3);
    pub const URGENT: BitField<MessageFlagBits> = BitField::from_bits(1 << 4);
    pub const HAS_THREAD: BitField<MessageFlagBits> = BitField::from_bits(1 << 5);
    pub const EPHEMERAL: BitField<MessageFlagBits> = BitField::from_bits(1 << 6);
    pub const LOADING: BitField<MessageFlagBits> = BitField::from_bits(1 << 7);
    pub const FAILED_TO_MENTION_SOME_ROLES_IN_THREAD: BitField<MessageFlagBits> =
        BitField::from_bits(1 << 8);
    pub const SUPPRESS_NOTIFICATIONS: BitField<MessageFlagBits> = BitField::from_bits(1 << 10);
    pub const IS_VOICE_MESSAGE: BitField<MessageFlagBits> = BitField::from_bits(1 << 12);
    pub const IS_COMPONENTS_V2: BitField<MessageFlagBits> = BitField::from_bits(1 << 15);
}

/// Interaction callback type constants.
pub mod interaction_callback_type {
    pub const PONG: u8 = 1;
    pub const CHANNEL_MESSAGE: u8 = 4;
    pub const DEFERRED_CHANNEL_MESSAGE: u8 = 5;
    pub const DEFERRED_UPDATE_MESSAGE: u8 = 6;
    pub const UPDATE_MESSAGE: u8 = 7;
    pub const AUTOCOMPLETE_RESULT: u8 = 8;
    pub const MODAL: u8 = 9;
    pub const PREMIUM_REQUIRED: u8 = 10;
    pub const LAUNCH_ACTIVITY: u8 = 12;
}

#[cfg(test)]
mod tests {
    use serde::de::{value::Error as ValueError, value::StringDeserializer};
    use serde::Deserialize;

    use super::{BitField, BitFieldFlags};

    #[derive(Copy, Clone, Debug, PartialEq, Eq)]
    struct TestFlags;

    impl BitFieldFlags for TestFlags {
        const BITS: &'static [(u64, &'static str)] = &[(1, "A"), (2, "B"), (8, "C")];
    }

    type TestBitField = BitField<TestFlags>;

    #[test]
    fn constructors_and_basic_queries_preserve_bits() {
        let empty = TestBitField::default();
        assert!(empty.is_empty());
        assert_eq!(empty.bits(), 0);

        let field = TestBitField::from_bits(1 | 8);
        assert_eq!(field.bits(), 9);
        assert!(field.contains(1));
        assert!(field.is_set(8));
        assert!(!field.contains(2));
        assert_eq!(field.to_api_string(), "9");
        assert_eq!(field.to_string(), "9");

        let added = empty.add(2).merge(field);
        assert_eq!(added.bits(), 11);

        let removed = added.remove(2);
        assert_eq!(removed, field);

        let from_raw = TestBitField::from(11);
        assert_eq!(from_raw.bits(), 11);

        let into_raw: u64 = from_raw.into();
        assert_eq!(into_raw, 11);
    }

    #[test]
    fn helpers_only_consider_declared_bits() {
        let mixed = TestBitField::from_bits(1 | 8 | 4);
        let all = TestBitField::all();

        assert_eq!(all.bits(), 11);
        assert_eq!(mixed.flag_names(), vec!["A", "C"]);
        assert_eq!(
            TestBitField::from_bits(1 | 8).missing(TestBitField::from_bits(1 | 2)),
            TestBitField::from_bits(2)
        );
        assert!(!mixed.any(0));
        assert!(mixed.any(2 | 8));
        assert!(mixed.has_all(1 | 8));
        assert!(!mixed.has_all(1 | 2 | 8));
    }

    #[test]
    fn operator_overloads_match_raw_bit_operations() {
        let left = TestBitField::from_bits(1 | 8);
        let right = TestBitField::from_bits(2 | 8);

        assert_eq!((left | right).bits(), 11);
        assert_eq!((left | 2).bits(), 11);
        assert_eq!((left & right).bits(), 8);
        assert_eq!((left & 8).bits(), 8);
        assert_eq!((left ^ right).bits(), 3);
        assert_eq!((left - right).bits(), 1);
        assert_eq!((left - 8).bits(), 1);
        assert_eq!((!left).bits(), !9);

        let mut assign = left;
        assign |= right;
        assert_eq!(assign.bits(), 11);
        assign &= TestBitField::from_bits(9);
        assert_eq!(assign.bits(), 9);
        assign ^= TestBitField::from_bits(1);
        assert_eq!(assign.bits(), 8);
        assign |= 2;
        assert_eq!(assign.bits(), 10);
        assign &= 8;
        assert_eq!(assign.bits(), 8);
    }

    #[test]
    fn serde_accepts_numeric_and_string_shapes() {
        let field = TestBitField::from_bits(9);
        assert_eq!(serde_json::to_string(&field).unwrap(), "\"9\"");

        let from_number: TestBitField = serde_json::from_str("9").unwrap();
        let from_string: TestBitField = serde_json::from_str("\"9\"").unwrap();
        let from_string_deserializer =
            TestBitField::deserialize(StringDeserializer::<ValueError>::new(String::from("9")))
                .unwrap();

        assert_eq!(from_number, field);
        assert_eq!(from_string, field);
        assert_eq!(from_string_deserializer, field);
    }

    #[test]
    fn serde_rejects_invalid_values_and_currently_wraps_negative_i64() {
        let negative: TestBitField = serde_json::from_str("-1").unwrap();
        assert_eq!(negative.bits(), u64::MAX);

        let invalid_value = serde_json::from_str::<TestBitField>("\"abc\"").unwrap_err();
        assert!(invalid_value.to_string().contains("invalid bitfield value"));

        let invalid_type = serde_json::from_str::<TestBitField>("true").unwrap_err();
        assert!(invalid_type
            .to_string()
            .contains("a bitfield encoded as a string or integer"));
    }
}
