use std::sync::Arc;
use std::time::Duration;

#[cfg(feature = "cache")]
use std::collections::{HashMap, HashSet, VecDeque};
#[cfg(feature = "cache")]
use std::time::Instant;
#[cfg(feature = "cache")]
use tokio::sync::RwLock;

#[cfg(feature = "gateway")]
use async_trait::async_trait;

use crate::error::DiscordError;
use crate::event::ScheduledEvent;
use crate::http::DiscordHttpClient;
use crate::model::{
    Channel, Guild, Member, Message, Presence, Role, Snowflake, SoundboardSound, StageInstance,
    Sticker, User, VoiceState,
};
use crate::types::Emoji;

#[cfg(feature = "gateway")]
use crate::manager::CachedManager;

#[cfg(feature = "cache")]
#[derive(Clone, Default)]
struct CacheStore {
    guilds: HashMap<Snowflake, Guild>,
    channels: HashMap<Snowflake, Channel>,
    users: HashMap<Snowflake, User>,
    members: HashMap<(Snowflake, Snowflake), Member>,
    messages: HashMap<(Snowflake, Snowflake), Message>,
    roles: HashMap<(Snowflake, Snowflake), Role>,
    presences: HashMap<(Snowflake, Snowflake), Presence>,
    voice_states: HashMap<(Snowflake, Snowflake), VoiceState>,
    soundboard_sounds: HashMap<(Snowflake, Snowflake), SoundboardSound>,
    emojis: HashMap<(Snowflake, Snowflake), Emoji>,
    stickers: HashMap<(Snowflake, Snowflake), Sticker>,
    scheduled_events: HashMap<(Snowflake, Snowflake), ScheduledEvent>,
    stage_instances: HashMap<(Snowflake, Snowflake), StageInstance>,
    guild_order: VecDeque<Snowflake>,
    channel_order: VecDeque<Snowflake>,
    user_order: VecDeque<Snowflake>,
    member_order: VecDeque<(Snowflake, Snowflake)>,
    message_order: VecDeque<(Snowflake, Snowflake)>,
    role_order: VecDeque<(Snowflake, Snowflake)>,
    presence_order: VecDeque<(Snowflake, Snowflake)>,
    voice_state_order: VecDeque<(Snowflake, Snowflake)>,
    soundboard_sound_order: VecDeque<(Snowflake, Snowflake)>,
    emoji_order: VecDeque<(Snowflake, Snowflake)>,
    sticker_order: VecDeque<(Snowflake, Snowflake)>,
    scheduled_event_order: VecDeque<(Snowflake, Snowflake)>,
    stage_instance_order: VecDeque<(Snowflake, Snowflake)>,
    member_seen: HashMap<(Snowflake, Snowflake), Instant>,
    message_seen: HashMap<(Snowflake, Snowflake), Instant>,
    presence_seen: HashMap<(Snowflake, Snowflake), Instant>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CacheConfig {
    pub max_messages_per_channel: Option<usize>,
    pub max_total_messages: Option<usize>,
    pub max_presences: Option<usize>,
    pub max_members_per_guild: Option<usize>,
    pub max_guilds: Option<usize>,
    pub max_channels: Option<usize>,
    pub max_users: Option<usize>,
    pub max_roles: Option<usize>,
    pub max_voice_states: Option<usize>,
    pub max_soundboard_sounds: Option<usize>,
    pub max_emojis: Option<usize>,
    pub max_stickers: Option<usize>,
    pub max_scheduled_events: Option<usize>,
    pub max_stage_instances: Option<usize>,
    pub message_ttl: Option<Duration>,
    pub presence_ttl: Option<Duration>,
    pub member_ttl: Option<Duration>,
    pub cache_emojis: bool,
    pub cache_stickers: bool,
    pub cache_scheduled_events: bool,
    pub cache_stage_instances: bool,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            max_messages_per_channel: None,
            max_total_messages: None,
            max_presences: None,
            max_members_per_guild: None,
            max_guilds: None,
            max_channels: None,
            max_users: None,
            max_roles: None,
            max_voice_states: None,
            max_soundboard_sounds: None,
            max_emojis: None,
            max_stickers: None,
            max_scheduled_events: None,
            max_stage_instances: None,
            message_ttl: None,
            presence_ttl: None,
            member_ttl: None,
            cache_emojis: true,
            cache_stickers: true,
            cache_scheduled_events: true,
            cache_stage_instances: true,
        }
    }
}

impl CacheConfig {
    pub fn unbounded() -> Self {
        Self::default()
    }

    pub fn max_messages_per_channel(mut self, max: usize) -> Self {
        self.max_messages_per_channel = Some(max);
        self
    }

    pub fn max_total_messages(mut self, max: usize) -> Self {
        self.max_total_messages = Some(max);
        self
    }

    pub fn max_presences(mut self, max: usize) -> Self {
        self.max_presences = Some(max);
        self
    }

    pub fn max_members_per_guild(mut self, max: usize) -> Self {
        self.max_members_per_guild = Some(max);
        self
    }

    pub fn max_guilds(mut self, max: usize) -> Self {
        self.max_guilds = Some(max);
        self
    }

    pub fn max_channels(mut self, max: usize) -> Self {
        self.max_channels = Some(max);
        self
    }

    pub fn max_users(mut self, max: usize) -> Self {
        self.max_users = Some(max);
        self
    }

    pub fn max_roles(mut self, max: usize) -> Self {
        self.max_roles = Some(max);
        self
    }

    pub fn max_voice_states(mut self, max: usize) -> Self {
        self.max_voice_states = Some(max);
        self
    }

    pub fn max_soundboard_sounds(mut self, max: usize) -> Self {
        self.max_soundboard_sounds = Some(max);
        self
    }

    pub fn max_emojis(mut self, max: usize) -> Self {
        self.max_emojis = Some(max);
        self
    }

    pub fn max_stickers(mut self, max: usize) -> Self {
        self.max_stickers = Some(max);
        self
    }

    pub fn max_scheduled_events(mut self, max: usize) -> Self {
        self.max_scheduled_events = Some(max);
        self
    }

    pub fn max_stage_instances(mut self, max: usize) -> Self {
        self.max_stage_instances = Some(max);
        self
    }

    pub fn message_ttl(mut self, ttl: Duration) -> Self {
        self.message_ttl = Some(ttl);
        self
    }

    pub fn presence_ttl(mut self, ttl: Duration) -> Self {
        self.presence_ttl = Some(ttl);
        self
    }

    pub fn member_ttl(mut self, ttl: Duration) -> Self {
        self.member_ttl = Some(ttl);
        self
    }

    pub fn cache_emojis(mut self, enabled: bool) -> Self {
        self.cache_emojis = enabled;
        self
    }

    pub fn cache_stickers(mut self, enabled: bool) -> Self {
        self.cache_stickers = enabled;
        self
    }

    pub fn cache_scheduled_events(mut self, enabled: bool) -> Self {
        self.cache_scheduled_events = enabled;
        self
    }

    pub fn cache_stage_instances(mut self, enabled: bool) -> Self {
        self.cache_stage_instances = enabled;
        self
    }
}

#[cfg(feature = "cache")]
fn remember_key<K>(order: &mut VecDeque<K>, key: K)
where
    K: Clone + Eq,
{
    if let Some(index) = order.iter().position(|stored| stored == &key) {
        order.remove(index);
    }
    order.push_back(key);
}

#[cfg(feature = "cache")]
fn ordered_overflow_keys<K>(order: &mut VecDeque<K>, len: usize, max: Option<usize>) -> Vec<K>
where
    K: Clone,
{
    let Some(max) = max else {
        return Vec::new();
    };

    (0..len.saturating_sub(max))
        .filter_map(|_| order.pop_front())
        .collect()
}

#[cfg(feature = "cache")]
fn enforce_guild_limit(store: &mut CacheStore, config: &CacheConfig) {
    let Some(max) = config.max_guilds else {
        return;
    };

    while store.guilds.len() > max {
        let Some(guild_id) = store.guild_order.pop_front() else {
            break;
        };
        if store.guilds.contains_key(&guild_id) {
            evict_guild_entries(store, &guild_id);
        }
    }
}

#[cfg(feature = "cache")]
fn enforce_channel_limit(store: &mut CacheStore, config: &CacheConfig) {
    let Some(max) = config.max_channels else {
        return;
    };

    while store.channels.len() > max {
        let Some(channel_id) = store.channel_order.pop_front() else {
            break;
        };
        if store.channels.contains_key(&channel_id) {
            evict_channel_entries(store, &channel_id);
        }
    }
}

#[cfg(feature = "cache")]
fn enforce_message_limits(store: &mut CacheStore, config: &CacheConfig, channel_id: &Snowflake) {
    if let Some(max) = config.max_messages_per_channel {
        while store
            .messages
            .keys()
            .filter(|(stored_channel_id, _)| stored_channel_id == channel_id)
            .count()
            > max
        {
            let Some(index) = store
                .message_order
                .iter()
                .position(|(stored_channel_id, _)| stored_channel_id == channel_id)
            else {
                break;
            };
            if let Some(key) = store.message_order.remove(index) {
                store.messages.remove(&key);
                store.message_seen.remove(&key);
            }
        }
    }

    if let Some(max) = config.max_total_messages {
        while store.messages.len() > max {
            let Some(key) = store.message_order.pop_front() else {
                break;
            };
            store.messages.remove(&key);
            store.message_seen.remove(&key);
        }
    }
}

#[cfg(feature = "cache")]
fn enforce_member_limit(store: &mut CacheStore, config: &CacheConfig, guild_id: &Snowflake) {
    let Some(max) = config.max_members_per_guild else {
        return;
    };
    while store
        .members
        .keys()
        .filter(|(stored_guild_id, _)| stored_guild_id == guild_id)
        .count()
        > max
    {
        let Some(index) = store
            .member_order
            .iter()
            .position(|(stored_guild_id, _)| stored_guild_id == guild_id)
        else {
            break;
        };
        if let Some(key) = store.member_order.remove(index) {
            store.members.remove(&key);
            store.member_seen.remove(&key);
        }
    }
}

#[cfg(feature = "cache")]
fn remove_message_key(store: &mut CacheStore, key: &(Snowflake, Snowflake)) {
    store.messages.remove(key);
    store.message_seen.remove(key);
    store.message_order.retain(|stored_key| stored_key != key);
}

#[cfg(feature = "cache")]
fn remove_member_key(store: &mut CacheStore, key: &(Snowflake, Snowflake)) {
    store.members.remove(key);
    store.member_seen.remove(key);
    store.member_order.retain(|stored_key| stored_key != key);
}

#[cfg(feature = "cache")]
fn remove_presence_key(store: &mut CacheStore, key: &(Snowflake, Snowflake)) {
    store.presences.remove(key);
    store.presence_seen.remove(key);
    store.presence_order.retain(|stored_key| stored_key != key);
}

#[cfg(feature = "cache")]
fn prune_expired(store: &mut CacheStore, config: &CacheConfig, now: Instant) {
    if let Some(ttl) = config.message_ttl {
        let expired: Vec<_> = store
            .message_seen
            .iter()
            .filter(|(_, seen)| now.duration_since(**seen) >= ttl)
            .map(|(key, _)| key.clone())
            .collect();
        for key in expired {
            remove_message_key(store, &key);
        }
    }

    if let Some(ttl) = config.presence_ttl {
        let expired: Vec<_> = store
            .presence_seen
            .iter()
            .filter(|(_, seen)| now.duration_since(**seen) >= ttl)
            .map(|(key, _)| key.clone())
            .collect();
        for key in expired {
            remove_presence_key(store, &key);
        }
    }

    if let Some(ttl) = config.member_ttl {
        let expired: Vec<_> = store
            .member_seen
            .iter()
            .filter(|(_, seen)| now.duration_since(**seen) >= ttl)
            .map(|(key, _)| key.clone())
            .collect();
        for key in expired {
            remove_member_key(store, &key);
        }
    }
}

#[cfg(feature = "cache")]
fn evict_channel_entries(store: &mut CacheStore, channel_id: &Snowflake) {
    store.channels.remove(channel_id);
    store
        .channel_order
        .retain(|stored_id| stored_id != channel_id);
    store
        .messages
        .retain(|(stored_channel_id, _), _| stored_channel_id != channel_id);
    store
        .message_seen
        .retain(|(stored_channel_id, _), _| stored_channel_id != channel_id);
    store
        .message_order
        .retain(|(stored_channel_id, _)| stored_channel_id != channel_id);
}

#[cfg(feature = "cache")]
fn evict_guild_entries(store: &mut CacheStore, guild_id: &Snowflake) {
    let removed_channel_ids: HashSet<_> = store
        .channels
        .iter()
        .filter(|(_, channel)| channel.guild_id.as_ref() == Some(guild_id))
        .map(|(channel_id, _)| channel_id.clone())
        .collect();

    store.guilds.remove(guild_id);
    store.guild_order.retain(|stored_id| stored_id != guild_id);
    store
        .channels
        .retain(|_, channel| channel.guild_id.as_ref() != Some(guild_id));
    store.channel_order.retain(|channel_id| {
        store
            .channels
            .get(channel_id)
            .is_some_and(|channel| channel.guild_id.as_ref() != Some(guild_id))
    });
    store
        .members
        .retain(|(stored_guild_id, _), _| stored_guild_id != guild_id);
    store
        .member_seen
        .retain(|(stored_guild_id, _), _| stored_guild_id != guild_id);
    store
        .member_order
        .retain(|(stored_guild_id, _)| stored_guild_id != guild_id);
    store.messages.retain(|(stored_channel_id, _), message| {
        !removed_channel_ids.contains(stored_channel_id)
            && message.guild_id.as_ref() != Some(guild_id)
    });
    let remaining_message_keys: HashSet<_> = store.messages.keys().cloned().collect();
    store
        .message_seen
        .retain(|key, _| remaining_message_keys.contains(key));
    store
        .message_order
        .retain(|(stored_channel_id, message_id)| {
            !removed_channel_ids.contains(stored_channel_id)
                && remaining_message_keys.contains(&(stored_channel_id.clone(), message_id.clone()))
        });
    store
        .roles
        .retain(|(stored_guild_id, _), _| stored_guild_id != guild_id);
    store
        .role_order
        .retain(|(stored_guild_id, _)| stored_guild_id != guild_id);
    store
        .presences
        .retain(|(stored_guild_id, _), _| stored_guild_id != guild_id);
    store
        .presence_seen
        .retain(|(stored_guild_id, _), _| stored_guild_id != guild_id);
    store
        .presence_order
        .retain(|(stored_guild_id, _)| stored_guild_id != guild_id);
    store
        .voice_states
        .retain(|(stored_guild_id, _), _| stored_guild_id != guild_id);
    store
        .voice_state_order
        .retain(|(stored_guild_id, _)| stored_guild_id != guild_id);
    store
        .soundboard_sounds
        .retain(|(stored_guild_id, _), _| stored_guild_id != guild_id);
    store
        .soundboard_sound_order
        .retain(|(stored_guild_id, _)| stored_guild_id != guild_id);
    store
        .emojis
        .retain(|(stored_guild_id, _), _| stored_guild_id != guild_id);
    store
        .emoji_order
        .retain(|(stored_guild_id, _)| stored_guild_id != guild_id);
    store
        .stickers
        .retain(|(stored_guild_id, _), _| stored_guild_id != guild_id);
    store
        .sticker_order
        .retain(|(stored_guild_id, _)| stored_guild_id != guild_id);
    store
        .scheduled_events
        .retain(|(stored_guild_id, _), _| stored_guild_id != guild_id);
    store
        .scheduled_event_order
        .retain(|(stored_guild_id, _)| stored_guild_id != guild_id);
    store
        .stage_instances
        .retain(|(stored_guild_id, _), _| stored_guild_id != guild_id);
    store
        .stage_instance_order
        .retain(|(stored_guild_id, _)| stored_guild_id != guild_id);
}

#[derive(Clone, Default)]
pub struct CacheHandle {
    #[cfg(feature = "cache")]
    store: Arc<RwLock<CacheStore>>,
    config: CacheConfig,
}

impl CacheHandle {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_config(config: CacheConfig) -> Self {
        Self {
            #[cfg(feature = "cache")]
            store: Arc::new(RwLock::new(CacheStore::default())),
            config,
        }
    }

    pub fn config(&self) -> &CacheConfig {
        &self.config
    }

    #[cfg(feature = "cache")]
    pub async fn clear(&self) {
        let mut store = self.store.write().await;
        store.guilds.clear();
        store.channels.clear();
        store.users.clear();
        store.members.clear();
        store.messages.clear();
        store.roles.clear();
        store.presences.clear();
        store.voice_states.clear();
        store.soundboard_sounds.clear();
        store.emojis.clear();
        store.stickers.clear();
        store.scheduled_events.clear();
        store.stage_instances.clear();
        store.guild_order.clear();
        store.channel_order.clear();
        store.user_order.clear();
        store.member_order.clear();
        store.message_order.clear();
        store.role_order.clear();
        store.presence_order.clear();
        store.voice_state_order.clear();
        store.soundboard_sound_order.clear();
        store.emoji_order.clear();
        store.sticker_order.clear();
        store.scheduled_event_order.clear();
        store.stage_instance_order.clear();
        store.member_seen.clear();
        store.message_seen.clear();
        store.presence_seen.clear();
    }

    #[cfg(not(feature = "cache"))]
    pub async fn clear(&self) {}

    #[cfg(feature = "cache")]
    pub async fn purge_expired(&self) {
        let mut store = self.store.write().await;
        prune_expired(&mut store, &self.config, Instant::now());
    }

    #[cfg(not(feature = "cache"))]
    pub async fn purge_expired(&self) {}

    #[cfg(feature = "cache")]
    pub async fn upsert_guild(&self, guild: Guild) {
        let mut store = self.store.write().await;
        let guild_id = guild.id.clone();
        store.guilds.insert(guild_id.clone(), guild);
        remember_key(&mut store.guild_order, guild_id);
        enforce_guild_limit(&mut store, &self.config);
    }

    #[cfg(not(feature = "cache"))]
    pub async fn upsert_guild(&self, _guild: Guild) {}

    #[cfg(feature = "cache")]
    pub async fn remove_guild(&self, guild_id: &Snowflake) {
        let mut store = self.store.write().await;
        evict_guild_entries(&mut store, guild_id);
    }

    #[cfg(not(feature = "cache"))]
    pub async fn remove_guild(&self, _guild_id: &Snowflake) {}

    #[cfg(feature = "cache")]
    pub async fn guild(&self, guild_id: &Snowflake) -> Option<Guild> {
        self.store.read().await.guilds.get(guild_id).cloned()
    }

    #[cfg(not(feature = "cache"))]
    pub async fn guild(&self, _guild_id: &Snowflake) -> Option<Guild> {
        None
    }

    #[cfg(feature = "cache")]
    pub async fn guilds(&self) -> Vec<Guild> {
        self.store.read().await.guilds.values().cloned().collect()
    }

    #[cfg(not(feature = "cache"))]
    pub async fn guilds(&self) -> Vec<Guild> {
        Vec::new()
    }

    pub async fn contains_guild(&self, guild_id: &Snowflake) -> bool {
        self.guild(guild_id).await.is_some()
    }

    #[cfg(feature = "cache")]
    pub async fn upsert_channel(&self, channel: Channel) {
        let mut store = self.store.write().await;
        let channel_id = channel.id.clone();
        store.channels.insert(channel_id.clone(), channel);
        remember_key(&mut store.channel_order, channel_id);
        enforce_channel_limit(&mut store, &self.config);
    }

    #[cfg(not(feature = "cache"))]
    pub async fn upsert_channel(&self, _channel: Channel) {}

    #[cfg(feature = "cache")]
    pub async fn remove_channel(&self, channel_id: &Snowflake) {
        let mut store = self.store.write().await;
        evict_channel_entries(&mut store, channel_id);
    }

    #[cfg(not(feature = "cache"))]
    pub async fn remove_channel(&self, _channel_id: &Snowflake) {}

    #[cfg(not(feature = "cache"))]
    pub async fn channel(&self, _channel_id: &Snowflake) -> Option<Channel> {
        None
    }

    #[cfg(feature = "cache")]
    pub async fn channel(&self, channel_id: &Snowflake) -> Option<Channel> {
        self.store.read().await.channels.get(channel_id).cloned()
    }

    #[cfg(feature = "cache")]
    pub async fn channels(&self) -> Vec<Channel> {
        self.store.read().await.channels.values().cloned().collect()
    }

    #[cfg(not(feature = "cache"))]
    pub async fn channels(&self) -> Vec<Channel> {
        Vec::new()
    }

    pub async fn contains_channel(&self, channel_id: &Snowflake) -> bool {
        self.channel(channel_id).await.is_some()
    }

    #[cfg(feature = "cache")]
    pub async fn upsert_user(&self, user: User) {
        let mut store = self.store.write().await;
        let user_id = user.id.clone();
        store.users.insert(user_id.clone(), user);
        remember_key(&mut store.user_order, user_id);
        let len = store.users.len();
        for key in ordered_overflow_keys(&mut store.user_order, len, self.config.max_users) {
            store.users.remove(&key);
        }
    }

    #[cfg(not(feature = "cache"))]
    pub async fn upsert_user(&self, _user: User) {}

    #[cfg(feature = "cache")]
    pub async fn remove_user(&self, user_id: &Snowflake) {
        let mut store = self.store.write().await;
        store.users.remove(user_id);
        store.user_order.retain(|stored_id| stored_id != user_id);
    }

    #[cfg(not(feature = "cache"))]
    pub async fn remove_user(&self, _user_id: &Snowflake) {}

    #[cfg(feature = "cache")]
    pub async fn user(&self, user_id: &Snowflake) -> Option<User> {
        self.store.read().await.users.get(user_id).cloned()
    }

    #[cfg(not(feature = "cache"))]
    pub async fn user(&self, _user_id: &Snowflake) -> Option<User> {
        None
    }

    #[cfg(feature = "cache")]
    pub async fn users(&self) -> Vec<User> {
        self.store.read().await.users.values().cloned().collect()
    }

    #[cfg(not(feature = "cache"))]
    pub async fn users(&self) -> Vec<User> {
        Vec::new()
    }

    pub async fn contains_user(&self, user_id: &Snowflake) -> bool {
        self.user(user_id).await.is_some()
    }

    #[cfg(feature = "cache")]
    pub async fn upsert_member(&self, guild_id: Snowflake, user_id: Snowflake, member: Member) {
        let mut store = self.store.write().await;
        let key = (guild_id.clone(), user_id);
        store.members.insert(key.clone(), member);
        store.member_seen.insert(key.clone(), Instant::now());
        remember_key(&mut store.member_order, key);
        prune_expired(&mut store, &self.config, Instant::now());
        enforce_member_limit(&mut store, &self.config, &guild_id);
    }

    #[cfg(not(feature = "cache"))]
    pub async fn upsert_member(&self, _guild_id: Snowflake, _user_id: Snowflake, _member: Member) {}

    #[cfg(feature = "cache")]
    pub async fn remove_member(&self, guild_id: &Snowflake, user_id: &Snowflake) {
        let mut store = self.store.write().await;
        let key = (guild_id.clone(), user_id.clone());
        remove_member_key(&mut store, &key);
    }

    #[cfg(not(feature = "cache"))]
    pub async fn remove_member(&self, _guild_id: &Snowflake, _user_id: &Snowflake) {}

    #[cfg(not(feature = "cache"))]
    pub async fn member(&self, _guild_id: &Snowflake, _user_id: &Snowflake) -> Option<Member> {
        None
    }

    #[cfg(feature = "cache")]
    pub async fn member(&self, guild_id: &Snowflake, user_id: &Snowflake) -> Option<Member> {
        let mut store = self.store.write().await;
        prune_expired(&mut store, &self.config, Instant::now());
        store
            .members
            .get(&(guild_id.clone(), user_id.clone()))
            .cloned()
    }

    #[cfg(feature = "cache")]
    pub async fn members(&self, guild_id: &Snowflake) -> Vec<Member> {
        let mut store = self.store.write().await;
        prune_expired(&mut store, &self.config, Instant::now());
        store
            .members
            .iter()
            .filter(|((stored_guild_id, _), _)| stored_guild_id == guild_id)
            .map(|(_, member)| member.clone())
            .collect()
    }

    #[cfg(not(feature = "cache"))]
    pub async fn members(&self, _guild_id: &Snowflake) -> Vec<Member> {
        Vec::new()
    }

    pub async fn contains_member(&self, guild_id: &Snowflake, user_id: &Snowflake) -> bool {
        self.member(guild_id, user_id).await.is_some()
    }

    #[cfg(feature = "cache")]
    pub async fn upsert_message(&self, message: Message) {
        let channel_id = message.channel_id.clone();
        let message_id = message.id.clone();
        let mut store = self.store.write().await;
        let key = (channel_id.clone(), message_id);
        store.messages.insert(key.clone(), message);
        store.message_seen.insert(key.clone(), Instant::now());
        remember_key(&mut store.message_order, key);
        prune_expired(&mut store, &self.config, Instant::now());
        enforce_message_limits(&mut store, &self.config, &channel_id);
    }

    #[cfg(not(feature = "cache"))]
    pub async fn upsert_message(&self, _message: Message) {}

    #[cfg(feature = "cache")]
    pub async fn remove_message(&self, channel_id: &Snowflake, message_id: &Snowflake) {
        let mut store = self.store.write().await;
        let key = (channel_id.clone(), message_id.clone());
        remove_message_key(&mut store, &key);
    }

    #[cfg(not(feature = "cache"))]
    pub async fn remove_message(&self, _channel_id: &Snowflake, _message_id: &Snowflake) {}

    #[cfg(feature = "cache")]
    pub async fn remove_messages_bulk(&self, channel_id: &Snowflake, message_ids: &[Snowflake]) {
        let mut store = self.store.write().await;
        store
            .messages
            .retain(|(stored_channel_id, stored_message_id), _| {
                stored_channel_id != channel_id || !message_ids.contains(stored_message_id)
            });
        store
            .message_seen
            .retain(|(stored_channel_id, stored_message_id), _| {
                stored_channel_id != channel_id || !message_ids.contains(stored_message_id)
            });
        store
            .message_order
            .retain(|(stored_channel_id, stored_message_id)| {
                stored_channel_id != channel_id || !message_ids.contains(stored_message_id)
            });
    }

    #[cfg(not(feature = "cache"))]
    pub async fn remove_messages_bulk(&self, _channel_id: &Snowflake, _message_ids: &[Snowflake]) {}

    #[cfg(not(feature = "cache"))]
    pub async fn message(
        &self,
        _channel_id: &Snowflake,
        _message_id: &Snowflake,
    ) -> Option<Message> {
        None
    }

    #[cfg(feature = "cache")]
    pub async fn message(&self, channel_id: &Snowflake, message_id: &Snowflake) -> Option<Message> {
        let mut store = self.store.write().await;
        prune_expired(&mut store, &self.config, Instant::now());
        store
            .messages
            .get(&(channel_id.clone(), message_id.clone()))
            .cloned()
    }

    #[cfg(feature = "cache")]
    pub async fn messages(&self, channel_id: &Snowflake) -> Vec<Message> {
        let mut store = self.store.write().await;
        prune_expired(&mut store, &self.config, Instant::now());
        store
            .messages
            .iter()
            .filter(|((stored_channel_id, _), _)| stored_channel_id == channel_id)
            .map(|(_, message)| message.clone())
            .collect()
    }

    #[cfg(not(feature = "cache"))]
    pub async fn messages(&self, _channel_id: &Snowflake) -> Vec<Message> {
        Vec::new()
    }

    pub async fn contains_message(&self, channel_id: &Snowflake, message_id: &Snowflake) -> bool {
        self.message(channel_id, message_id).await.is_some()
    }

    #[cfg(feature = "cache")]
    pub async fn upsert_role(&self, guild_id: Snowflake, role: Role) {
        let mut store = self.store.write().await;
        let key = (guild_id, role.id.clone());
        store.roles.insert(key.clone(), role);
        remember_key(&mut store.role_order, key);
        let len = store.roles.len();
        for key in ordered_overflow_keys(&mut store.role_order, len, self.config.max_roles) {
            store.roles.remove(&key);
        }
    }

    #[cfg(not(feature = "cache"))]
    pub async fn upsert_role(&self, _guild_id: Snowflake, _role: Role) {}

    #[cfg(feature = "cache")]
    pub async fn remove_role(&self, guild_id: &Snowflake, role_id: &Snowflake) {
        let mut store = self.store.write().await;
        let key = (guild_id.clone(), role_id.clone());
        store.roles.remove(&key);
        store.role_order.retain(|stored_key| stored_key != &key);
    }

    #[cfg(not(feature = "cache"))]
    pub async fn remove_role(&self, _guild_id: &Snowflake, _role_id: &Snowflake) {}

    #[cfg(not(feature = "cache"))]
    pub async fn roles(&self, _guild_id: &Snowflake) -> Vec<Role> {
        Vec::new()
    }

    #[cfg(feature = "cache")]
    pub async fn roles(&self, guild_id: &Snowflake) -> Vec<Role> {
        self.store
            .read()
            .await
            .roles
            .iter()
            .filter(|((stored_guild_id, _), _)| stored_guild_id == guild_id)
            .map(|(_, role)| role.clone())
            .collect()
    }

    #[cfg(feature = "cache")]
    pub async fn role(&self, guild_id: &Snowflake, role_id: &Snowflake) -> Option<Role> {
        self.store
            .read()
            .await
            .roles
            .get(&(guild_id.clone(), role_id.clone()))
            .cloned()
    }

    #[cfg(not(feature = "cache"))]
    pub async fn role(&self, _guild_id: &Snowflake, _role_id: &Snowflake) -> Option<Role> {
        None
    }

    pub async fn contains_role(&self, guild_id: &Snowflake, role_id: &Snowflake) -> bool {
        self.role(guild_id, role_id).await.is_some()
    }

    #[cfg(feature = "cache")]
    pub async fn upsert_presence(
        &self,
        guild_id: Snowflake,
        user_id: Snowflake,
        presence: Presence,
    ) {
        let mut store = self.store.write().await;
        let key = (guild_id, user_id);
        store.presences.insert(key.clone(), presence);
        store.presence_seen.insert(key.clone(), Instant::now());
        remember_key(&mut store.presence_order, key);
        prune_expired(&mut store, &self.config, Instant::now());
        if let Some(max) = self.config.max_presences {
            while store.presences.len() > max {
                let Some(key) = store.presence_order.pop_front() else {
                    break;
                };
                store.presences.remove(&key);
                store.presence_seen.remove(&key);
            }
        }
    }

    #[cfg(not(feature = "cache"))]
    pub async fn upsert_presence(
        &self,
        _guild_id: Snowflake,
        _user_id: Snowflake,
        _presence: Presence,
    ) {
    }

    #[cfg(feature = "cache")]
    pub async fn remove_presence(&self, guild_id: &Snowflake, user_id: &Snowflake) {
        let mut store = self.store.write().await;
        let key = (guild_id.clone(), user_id.clone());
        remove_presence_key(&mut store, &key);
    }

    #[cfg(not(feature = "cache"))]
    pub async fn remove_presence(&self, _guild_id: &Snowflake, _user_id: &Snowflake) {}

    #[cfg(feature = "cache")]
    pub async fn presence(&self, guild_id: &Snowflake, user_id: &Snowflake) -> Option<Presence> {
        let mut store = self.store.write().await;
        prune_expired(&mut store, &self.config, Instant::now());
        store
            .presences
            .get(&(guild_id.clone(), user_id.clone()))
            .cloned()
    }

    #[cfg(not(feature = "cache"))]
    pub async fn presence(&self, _guild_id: &Snowflake, _user_id: &Snowflake) -> Option<Presence> {
        None
    }

    #[cfg(feature = "cache")]
    pub async fn presences(&self, guild_id: &Snowflake) -> Vec<Presence> {
        let mut store = self.store.write().await;
        prune_expired(&mut store, &self.config, Instant::now());
        store
            .presences
            .iter()
            .filter(|((stored_guild_id, _), _)| stored_guild_id == guild_id)
            .map(|(_, presence)| presence.clone())
            .collect()
    }

    #[cfg(not(feature = "cache"))]
    pub async fn presences(&self, _guild_id: &Snowflake) -> Vec<Presence> {
        Vec::new()
    }

    pub async fn contains_presence(&self, guild_id: &Snowflake, user_id: &Snowflake) -> bool {
        self.presence(guild_id, user_id).await.is_some()
    }

    #[cfg(feature = "cache")]
    pub async fn upsert_voice_state(
        &self,
        guild_id: Snowflake,
        user_id: Snowflake,
        voice_state: VoiceState,
    ) {
        let mut store = self.store.write().await;
        let key = (guild_id, user_id);
        store.voice_states.insert(key.clone(), voice_state);
        remember_key(&mut store.voice_state_order, key);
        let len = store.voice_states.len();
        for key in ordered_overflow_keys(
            &mut store.voice_state_order,
            len,
            self.config.max_voice_states,
        ) {
            store.voice_states.remove(&key);
        }
    }

    #[cfg(not(feature = "cache"))]
    pub async fn upsert_voice_state(
        &self,
        _guild_id: Snowflake,
        _user_id: Snowflake,
        _voice_state: VoiceState,
    ) {
    }

    #[cfg(feature = "cache")]
    pub async fn remove_voice_state(&self, guild_id: &Snowflake, user_id: &Snowflake) {
        let mut store = self.store.write().await;
        let key = (guild_id.clone(), user_id.clone());
        store.voice_states.remove(&key);
        store
            .voice_state_order
            .retain(|stored_key| stored_key != &key);
    }

    #[cfg(not(feature = "cache"))]
    pub async fn remove_voice_state(&self, _guild_id: &Snowflake, _user_id: &Snowflake) {}

    #[cfg(feature = "cache")]
    pub async fn voice_state(
        &self,
        guild_id: &Snowflake,
        user_id: &Snowflake,
    ) -> Option<VoiceState> {
        self.store
            .read()
            .await
            .voice_states
            .get(&(guild_id.clone(), user_id.clone()))
            .cloned()
    }

    #[cfg(not(feature = "cache"))]
    pub async fn voice_state(
        &self,
        _guild_id: &Snowflake,
        _user_id: &Snowflake,
    ) -> Option<VoiceState> {
        None
    }

    #[cfg(feature = "cache")]
    pub async fn voice_states(&self, guild_id: &Snowflake) -> Vec<VoiceState> {
        self.store
            .read()
            .await
            .voice_states
            .iter()
            .filter(|((stored_guild_id, _), _)| stored_guild_id == guild_id)
            .map(|(_, voice_state)| voice_state.clone())
            .collect()
    }

    #[cfg(not(feature = "cache"))]
    pub async fn voice_states(&self, _guild_id: &Snowflake) -> Vec<VoiceState> {
        Vec::new()
    }

    pub async fn contains_voice_state(&self, guild_id: &Snowflake, user_id: &Snowflake) -> bool {
        self.voice_state(guild_id, user_id).await.is_some()
    }

    #[cfg(feature = "cache")]
    pub async fn upsert_soundboard_sound(&self, guild_id: Snowflake, sound: SoundboardSound) {
        let mut store = self.store.write().await;
        let key = (guild_id, sound.sound_id.clone());
        store.soundboard_sounds.insert(key.clone(), sound);
        remember_key(&mut store.soundboard_sound_order, key);
        let len = store.soundboard_sounds.len();
        for key in ordered_overflow_keys(
            &mut store.soundboard_sound_order,
            len,
            self.config.max_soundboard_sounds,
        ) {
            store.soundboard_sounds.remove(&key);
        }
    }

    #[cfg(not(feature = "cache"))]
    pub async fn upsert_soundboard_sound(&self, _guild_id: Snowflake, _sound: SoundboardSound) {}

    #[cfg(feature = "cache")]
    pub async fn replace_soundboard_sounds(
        &self,
        guild_id: Snowflake,
        sounds: Vec<SoundboardSound>,
    ) {
        let mut store = self.store.write().await;
        store
            .soundboard_sounds
            .retain(|(stored_guild_id, _), _| stored_guild_id != &guild_id);
        store
            .soundboard_sound_order
            .retain(|(stored_guild_id, _)| stored_guild_id != &guild_id);
        for sound in sounds {
            let key = (guild_id.clone(), sound.sound_id.clone());
            store.soundboard_sounds.insert(key.clone(), sound);
            remember_key(&mut store.soundboard_sound_order, key);
        }
        let len = store.soundboard_sounds.len();
        for key in ordered_overflow_keys(
            &mut store.soundboard_sound_order,
            len,
            self.config.max_soundboard_sounds,
        ) {
            store.soundboard_sounds.remove(&key);
        }
    }

    #[cfg(not(feature = "cache"))]
    pub async fn replace_soundboard_sounds(
        &self,
        _guild_id: Snowflake,
        _sounds: Vec<SoundboardSound>,
    ) {
    }

    #[cfg(feature = "cache")]
    pub async fn remove_soundboard_sound(&self, guild_id: &Snowflake, sound_id: &Snowflake) {
        let mut store = self.store.write().await;
        let key = (guild_id.clone(), sound_id.clone());
        store.soundboard_sounds.remove(&key);
        store
            .soundboard_sound_order
            .retain(|stored_key| stored_key != &key);
    }

    #[cfg(not(feature = "cache"))]
    pub async fn remove_soundboard_sound(&self, _guild_id: &Snowflake, _sound_id: &Snowflake) {}

    #[cfg(feature = "cache")]
    pub async fn soundboard_sound(
        &self,
        guild_id: &Snowflake,
        sound_id: &Snowflake,
    ) -> Option<SoundboardSound> {
        self.store
            .read()
            .await
            .soundboard_sounds
            .get(&(guild_id.clone(), sound_id.clone()))
            .cloned()
    }

    #[cfg(not(feature = "cache"))]
    pub async fn soundboard_sound(
        &self,
        _guild_id: &Snowflake,
        _sound_id: &Snowflake,
    ) -> Option<SoundboardSound> {
        None
    }

    #[cfg(feature = "cache")]
    pub async fn soundboard_sounds(&self, guild_id: &Snowflake) -> Vec<SoundboardSound> {
        self.store
            .read()
            .await
            .soundboard_sounds
            .iter()
            .filter(|((stored_guild_id, _), _)| stored_guild_id == guild_id)
            .map(|(_, sound)| sound.clone())
            .collect()
    }

    #[cfg(not(feature = "cache"))]
    pub async fn soundboard_sounds(&self, _guild_id: &Snowflake) -> Vec<SoundboardSound> {
        Vec::new()
    }

    pub async fn contains_soundboard_sound(
        &self,
        guild_id: &Snowflake,
        sound_id: &Snowflake,
    ) -> bool {
        self.soundboard_sound(guild_id, sound_id).await.is_some()
    }

    #[cfg(feature = "cache")]
    pub async fn replace_emojis(&self, guild_id: Snowflake, emojis: Vec<Emoji>) {
        if !self.config.cache_emojis {
            return;
        }
        let mut store = self.store.write().await;
        store
            .emojis
            .retain(|(stored_guild_id, _), _| stored_guild_id != &guild_id);
        store
            .emoji_order
            .retain(|(stored_guild_id, _)| stored_guild_id != &guild_id);
        for emoji in emojis {
            if let Some(emoji_id) = emoji.id.clone() {
                let key = (guild_id.clone(), Snowflake::from(emoji_id.as_str()));
                store.emojis.insert(key.clone(), emoji);
                remember_key(&mut store.emoji_order, key);
            }
        }
        let len = store.emojis.len();
        for key in ordered_overflow_keys(&mut store.emoji_order, len, self.config.max_emojis) {
            store.emojis.remove(&key);
        }
    }

    #[cfg(not(feature = "cache"))]
    pub async fn replace_emojis(&self, _guild_id: Snowflake, _emojis: Vec<Emoji>) {}

    #[cfg(feature = "cache")]
    pub async fn emoji(&self, guild_id: &Snowflake, emoji_id: &Snowflake) -> Option<Emoji> {
        self.store
            .read()
            .await
            .emojis
            .get(&(guild_id.clone(), emoji_id.clone()))
            .cloned()
    }

    #[cfg(not(feature = "cache"))]
    pub async fn emoji(&self, _guild_id: &Snowflake, _emoji_id: &Snowflake) -> Option<Emoji> {
        None
    }

    #[cfg(feature = "cache")]
    pub async fn emojis(&self, guild_id: &Snowflake) -> Vec<Emoji> {
        self.store
            .read()
            .await
            .emojis
            .iter()
            .filter(|((stored_guild_id, _), _)| stored_guild_id == guild_id)
            .map(|(_, emoji)| emoji.clone())
            .collect()
    }

    #[cfg(not(feature = "cache"))]
    pub async fn emojis(&self, _guild_id: &Snowflake) -> Vec<Emoji> {
        Vec::new()
    }

    #[cfg(feature = "cache")]
    pub async fn replace_stickers(&self, guild_id: Snowflake, stickers: Vec<Sticker>) {
        if !self.config.cache_stickers {
            return;
        }
        let mut store = self.store.write().await;
        store
            .stickers
            .retain(|(stored_guild_id, _), _| stored_guild_id != &guild_id);
        store
            .sticker_order
            .retain(|(stored_guild_id, _)| stored_guild_id != &guild_id);
        for sticker in stickers {
            let key = (guild_id.clone(), sticker.id.clone());
            store.stickers.insert(key.clone(), sticker);
            remember_key(&mut store.sticker_order, key);
        }
        let len = store.stickers.len();
        for key in ordered_overflow_keys(&mut store.sticker_order, len, self.config.max_stickers) {
            store.stickers.remove(&key);
        }
    }

    #[cfg(not(feature = "cache"))]
    pub async fn replace_stickers(&self, _guild_id: Snowflake, _stickers: Vec<Sticker>) {}

    #[cfg(feature = "cache")]
    pub async fn sticker(&self, guild_id: &Snowflake, sticker_id: &Snowflake) -> Option<Sticker> {
        self.store
            .read()
            .await
            .stickers
            .get(&(guild_id.clone(), sticker_id.clone()))
            .cloned()
    }

    #[cfg(not(feature = "cache"))]
    pub async fn sticker(&self, _guild_id: &Snowflake, _sticker_id: &Snowflake) -> Option<Sticker> {
        None
    }

    #[cfg(feature = "cache")]
    pub async fn stickers(&self, guild_id: &Snowflake) -> Vec<Sticker> {
        self.store
            .read()
            .await
            .stickers
            .iter()
            .filter(|((stored_guild_id, _), _)| stored_guild_id == guild_id)
            .map(|(_, sticker)| sticker.clone())
            .collect()
    }

    #[cfg(not(feature = "cache"))]
    pub async fn stickers(&self, _guild_id: &Snowflake) -> Vec<Sticker> {
        Vec::new()
    }

    #[cfg(feature = "cache")]
    pub async fn upsert_scheduled_event(&self, event: ScheduledEvent) {
        if !self.config.cache_scheduled_events {
            return;
        }
        let (Some(guild_id), Some(event_id)) = (event.guild_id.clone(), event.id.clone()) else {
            return;
        };
        let mut store = self.store.write().await;
        let key = (guild_id, event_id);
        store.scheduled_events.insert(key.clone(), event);
        remember_key(&mut store.scheduled_event_order, key);
        let len = store.scheduled_events.len();
        for key in ordered_overflow_keys(
            &mut store.scheduled_event_order,
            len,
            self.config.max_scheduled_events,
        ) {
            store.scheduled_events.remove(&key);
        }
    }

    #[cfg(not(feature = "cache"))]
    pub async fn upsert_scheduled_event(&self, _event: ScheduledEvent) {}

    #[cfg(feature = "cache")]
    pub async fn remove_scheduled_event(&self, guild_id: &Snowflake, event_id: &Snowflake) {
        let mut store = self.store.write().await;
        let key = (guild_id.clone(), event_id.clone());
        store.scheduled_events.remove(&key);
        store
            .scheduled_event_order
            .retain(|stored_key| stored_key != &key);
    }

    #[cfg(not(feature = "cache"))]
    pub async fn remove_scheduled_event(&self, _guild_id: &Snowflake, _event_id: &Snowflake) {}

    #[cfg(feature = "cache")]
    pub async fn scheduled_event(
        &self,
        guild_id: &Snowflake,
        event_id: &Snowflake,
    ) -> Option<ScheduledEvent> {
        self.store
            .read()
            .await
            .scheduled_events
            .get(&(guild_id.clone(), event_id.clone()))
            .cloned()
    }

    #[cfg(not(feature = "cache"))]
    pub async fn scheduled_event(
        &self,
        _guild_id: &Snowflake,
        _event_id: &Snowflake,
    ) -> Option<ScheduledEvent> {
        None
    }

    #[cfg(feature = "cache")]
    pub async fn scheduled_events(&self, guild_id: &Snowflake) -> Vec<ScheduledEvent> {
        self.store
            .read()
            .await
            .scheduled_events
            .iter()
            .filter(|((stored_guild_id, _), _)| stored_guild_id == guild_id)
            .map(|(_, event)| event.clone())
            .collect()
    }

    #[cfg(not(feature = "cache"))]
    pub async fn scheduled_events(&self, _guild_id: &Snowflake) -> Vec<ScheduledEvent> {
        Vec::new()
    }

    #[cfg(feature = "cache")]
    pub async fn upsert_stage_instance(&self, stage_instance: StageInstance) {
        if !self.config.cache_stage_instances {
            return;
        }
        let mut store = self.store.write().await;
        let key = (stage_instance.guild_id.clone(), stage_instance.id.clone());
        store.stage_instances.insert(key.clone(), stage_instance);
        remember_key(&mut store.stage_instance_order, key);
        let len = store.stage_instances.len();
        for key in ordered_overflow_keys(
            &mut store.stage_instance_order,
            len,
            self.config.max_stage_instances,
        ) {
            store.stage_instances.remove(&key);
        }
    }

    #[cfg(not(feature = "cache"))]
    pub async fn upsert_stage_instance(&self, _stage_instance: StageInstance) {}

    #[cfg(feature = "cache")]
    pub async fn remove_stage_instance(&self, guild_id: &Snowflake, stage_instance_id: &Snowflake) {
        let mut store = self.store.write().await;
        let key = (guild_id.clone(), stage_instance_id.clone());
        store.stage_instances.remove(&key);
        store
            .stage_instance_order
            .retain(|stored_key| stored_key != &key);
    }

    #[cfg(not(feature = "cache"))]
    pub async fn remove_stage_instance(
        &self,
        _guild_id: &Snowflake,
        _stage_instance_id: &Snowflake,
    ) {
    }

    #[cfg(feature = "cache")]
    pub async fn stage_instance(
        &self,
        guild_id: &Snowflake,
        stage_instance_id: &Snowflake,
    ) -> Option<StageInstance> {
        self.store
            .read()
            .await
            .stage_instances
            .get(&(guild_id.clone(), stage_instance_id.clone()))
            .cloned()
    }

    #[cfg(not(feature = "cache"))]
    pub async fn stage_instance(
        &self,
        _guild_id: &Snowflake,
        _stage_instance_id: &Snowflake,
    ) -> Option<StageInstance> {
        None
    }

    #[cfg(feature = "cache")]
    pub async fn stage_instances(&self, guild_id: &Snowflake) -> Vec<StageInstance> {
        self.store
            .read()
            .await
            .stage_instances
            .iter()
            .filter(|((stored_guild_id, _), _)| stored_guild_id == guild_id)
            .map(|(_, stage_instance)| stage_instance.clone())
            .collect()
    }

    #[cfg(not(feature = "cache"))]
    pub async fn stage_instances(&self, _guild_id: &Snowflake) -> Vec<StageInstance> {
        Vec::new()
    }
}

#[derive(Clone)]
pub struct UserManager {
    http: Arc<DiscordHttpClient>,
    cache: CacheHandle,
}

impl UserManager {
    #[cfg(feature = "gateway")]
    pub(crate) fn new(http: Arc<DiscordHttpClient>, cache: CacheHandle) -> Self {
        Self { http, cache }
    }

    pub async fn get(&self, user_id: impl Into<Snowflake>) -> Result<User, DiscordError> {
        let user_id = user_id.into();
        if let Some(user) = self.cache.user(&user_id).await {
            return Ok(user);
        }
        self.http.get_user(user_id).await
    }

    pub async fn cached(&self, user_id: impl Into<Snowflake>) -> Option<User> {
        self.cache.user(&user_id.into()).await
    }

    pub async fn contains(&self, user_id: impl Into<Snowflake>) -> bool {
        self.cache.contains_user(&user_id.into()).await
    }

    pub async fn list_cached(&self) -> Vec<User> {
        self.cache.users().await
    }
}

#[derive(Clone)]
pub struct GuildManager {
    http: Arc<DiscordHttpClient>,
    cache: CacheHandle,
}

impl GuildManager {
    #[cfg(feature = "gateway")]
    pub(crate) fn new(http: Arc<DiscordHttpClient>, cache: CacheHandle) -> Self {
        Self { http, cache }
    }

    pub async fn get(&self, guild_id: impl Into<Snowflake>) -> Result<Guild, DiscordError> {
        let guild_id = guild_id.into();
        if let Some(guild) = self.cache.guild(&guild_id).await {
            return Ok(guild);
        }
        self.http.get_guild(guild_id).await
    }

    pub async fn cached(&self, guild_id: impl Into<Snowflake>) -> Option<Guild> {
        self.cache.guild(&guild_id.into()).await
    }

    pub async fn contains(&self, guild_id: impl Into<Snowflake>) -> bool {
        self.cache.contains_guild(&guild_id.into()).await
    }

    pub async fn list_cached(&self) -> Vec<Guild> {
        self.cache.guilds().await
    }
}

#[derive(Clone)]
pub struct ChannelManager {
    http: Arc<DiscordHttpClient>,
    cache: CacheHandle,
}

impl ChannelManager {
    #[cfg(feature = "gateway")]
    pub(crate) fn new(http: Arc<DiscordHttpClient>, cache: CacheHandle) -> Self {
        Self { http, cache }
    }

    pub async fn get(&self, channel_id: impl Into<Snowflake>) -> Result<Channel, DiscordError> {
        let channel_id = channel_id.into();
        if let Some(channel) = self.cache.channel(&channel_id).await {
            return Ok(channel);
        }
        self.http.get_channel(channel_id).await
    }

    pub async fn cached(&self, channel_id: impl Into<Snowflake>) -> Option<Channel> {
        self.cache.channel(&channel_id.into()).await
    }

    pub async fn contains(&self, channel_id: impl Into<Snowflake>) -> bool {
        self.cache.contains_channel(&channel_id.into()).await
    }

    pub async fn list_cached(&self) -> Vec<Channel> {
        self.cache.channels().await
    }
}

#[derive(Clone)]
pub struct MemberManager {
    http: Arc<DiscordHttpClient>,
    cache: CacheHandle,
}

impl MemberManager {
    #[cfg(feature = "gateway")]
    pub(crate) fn new(http: Arc<DiscordHttpClient>, cache: CacheHandle) -> Self {
        Self { http, cache }
    }

    pub async fn get(
        &self,
        guild_id: impl Into<Snowflake>,
        user_id: impl Into<Snowflake>,
    ) -> Result<Member, DiscordError> {
        let guild_id = guild_id.into();
        let user_id = user_id.into();
        if let Some(member) = self.cache.member(&guild_id, &user_id).await {
            return Ok(member);
        }
        self.http.get_member(guild_id, user_id).await
    }

    pub async fn cached(
        &self,
        guild_id: impl Into<Snowflake>,
        user_id: impl Into<Snowflake>,
    ) -> Option<Member> {
        self.cache.member(&guild_id.into(), &user_id.into()).await
    }

    pub async fn contains(
        &self,
        guild_id: impl Into<Snowflake>,
        user_id: impl Into<Snowflake>,
    ) -> bool {
        self.cache
            .contains_member(&guild_id.into(), &user_id.into())
            .await
    }

    pub async fn list_cached(&self, guild_id: impl Into<Snowflake>) -> Vec<Member> {
        self.cache.members(&guild_id.into()).await
    }
}

#[derive(Clone)]
pub struct MessageManager {
    http: Arc<DiscordHttpClient>,
    cache: CacheHandle,
}

impl MessageManager {
    #[cfg(feature = "gateway")]
    pub(crate) fn new(http: Arc<DiscordHttpClient>, cache: CacheHandle) -> Self {
        Self { http, cache }
    }

    pub async fn get(
        &self,
        channel_id: impl Into<Snowflake>,
        message_id: impl Into<Snowflake>,
    ) -> Result<Message, DiscordError> {
        let channel_id = channel_id.into();
        let message_id = message_id.into();
        if let Some(message) = self.cache.message(&channel_id, &message_id).await {
            return Ok(message);
        }
        self.http.get_message(channel_id, message_id).await
    }

    pub async fn cached(
        &self,
        channel_id: impl Into<Snowflake>,
        message_id: impl Into<Snowflake>,
    ) -> Option<Message> {
        self.cache
            .message(&channel_id.into(), &message_id.into())
            .await
    }

    pub async fn contains(
        &self,
        channel_id: impl Into<Snowflake>,
        message_id: impl Into<Snowflake>,
    ) -> bool {
        self.cache
            .contains_message(&channel_id.into(), &message_id.into())
            .await
    }

    pub async fn list_cached(&self, channel_id: impl Into<Snowflake>) -> Vec<Message> {
        self.cache.messages(&channel_id.into()).await
    }
}

#[derive(Clone)]
pub struct RoleManager {
    http: Arc<DiscordHttpClient>,
    cache: CacheHandle,
}

impl RoleManager {
    #[cfg(feature = "gateway")]
    pub(crate) fn new(http: Arc<DiscordHttpClient>, cache: CacheHandle) -> Self {
        Self { http, cache }
    }

    pub async fn list(&self, guild_id: impl Into<Snowflake>) -> Result<Vec<Role>, DiscordError> {
        let guild_id = guild_id.into();
        let cached = self.cache.roles(&guild_id).await;
        if !cached.is_empty() {
            return Ok(cached);
        }
        self.http.list_roles(guild_id).await
    }

    pub async fn cached(
        &self,
        guild_id: impl Into<Snowflake>,
        role_id: impl Into<Snowflake>,
    ) -> Option<Role> {
        self.cache.role(&guild_id.into(), &role_id.into()).await
    }

    pub async fn contains(
        &self,
        guild_id: impl Into<Snowflake>,
        role_id: impl Into<Snowflake>,
    ) -> bool {
        self.cache
            .contains_role(&guild_id.into(), &role_id.into())
            .await
    }

    pub async fn list_cached(&self, guild_id: impl Into<Snowflake>) -> Vec<Role> {
        self.cache.roles(&guild_id.into()).await
    }
}

#[cfg(feature = "gateway")]
#[async_trait]
impl CachedManager<Guild> for GuildManager {
    async fn get(&self, id: impl Into<Snowflake> + Send) -> Result<Guild, DiscordError> {
        let id = id.into();
        if let Some(guild) = self.cache.guild(&id).await {
            return Ok(guild);
        }
        self.http.get_guild(id).await
    }

    async fn cached(&self, id: impl Into<Snowflake> + Send) -> Option<Guild> {
        self.cache.guild(&id.into()).await
    }

    async fn contains(&self, id: impl Into<Snowflake> + Send) -> bool {
        self.cache.contains_guild(&id.into()).await
    }

    async fn list_cached(&self) -> Vec<Guild> {
        self.cache.guilds().await
    }
}

#[cfg(feature = "gateway")]
#[async_trait]
impl CachedManager<Channel> for ChannelManager {
    async fn get(&self, id: impl Into<Snowflake> + Send) -> Result<Channel, DiscordError> {
        let id = id.into();
        if let Some(channel) = self.cache.channel(&id).await {
            return Ok(channel);
        }
        self.http.get_channel(id).await
    }

    async fn cached(&self, id: impl Into<Snowflake> + Send) -> Option<Channel> {
        self.cache.channel(&id.into()).await
    }

    async fn contains(&self, id: impl Into<Snowflake> + Send) -> bool {
        self.cache.contains_channel(&id.into()).await
    }

    async fn list_cached(&self) -> Vec<Channel> {
        self.cache.channels().await
    }
}

#[cfg(feature = "gateway")]
#[async_trait]
impl CachedManager<User> for UserManager {
    async fn get(&self, id: impl Into<Snowflake> + Send) -> Result<User, DiscordError> {
        let id = id.into();
        if let Some(user) = self.cache.user(&id).await {
            return Ok(user);
        }
        self.http.get_user(id).await
    }

    async fn cached(&self, id: impl Into<Snowflake> + Send) -> Option<User> {
        self.cache.user(&id.into()).await
    }

    async fn contains(&self, id: impl Into<Snowflake> + Send) -> bool {
        self.cache.contains_user(&id.into()).await
    }

    async fn list_cached(&self) -> Vec<User> {
        self.cache.users().await
    }
}

#[cfg(all(test, feature = "cache"))]
mod tests {
    use std::sync::Arc;
    use std::time::Duration;

    use crate::event::ScheduledEvent;
    #[cfg(feature = "gateway")]
    use crate::manager::CachedManager;
    use crate::model::{
        Channel, Guild, Message, Presence, Role, Snowflake, SoundboardSound, StageInstance,
        Sticker, User, VoiceState,
    };
    use crate::types::Emoji;

    use super::{
        CacheConfig, CacheHandle, ChannelManager, GuildManager, MemberManager, MessageManager,
        RoleManager, UserManager,
    };
    use crate::http::DiscordHttpClient;

    #[tokio::test]
    async fn cache_handle_tracks_create_and_delete_flows() {
        let cache = CacheHandle::new();
        let guild_id = Snowflake::from("1");
        let other_guild_id = Snowflake::from("2");
        let channel_id = Snowflake::from("10");
        let other_channel_id = Snowflake::from("20");
        let dm_channel_id = Snowflake::from("30");
        let user_id = Snowflake::from("11");
        let other_user_id = Snowflake::from("21");
        let message_id = Snowflake::from("12");
        let orphan_channel_id = Snowflake::from("13");
        let orphan_message_id = Snowflake::from("14");
        let other_message_id = Snowflake::from("22");
        let dm_message_id = Snowflake::from("31");
        let role_id = Snowflake::from("15");
        let other_role_id = Snowflake::from("23");

        cache
            .upsert_guild(Guild {
                id: guild_id.clone(),
                name: "discordrs".to_string(),
                ..Guild::default()
            })
            .await;
        cache
            .upsert_guild(Guild {
                id: other_guild_id.clone(),
                name: "other".to_string(),
                ..Guild::default()
            })
            .await;
        cache
            .upsert_role(
                guild_id.clone(),
                Role {
                    id: role_id.clone(),
                    name: "admin".to_string(),
                    ..Role::default()
                },
            )
            .await;
        cache
            .upsert_role(
                other_guild_id.clone(),
                Role {
                    id: other_role_id.clone(),
                    name: "member".to_string(),
                    ..Role::default()
                },
            )
            .await;
        cache
            .upsert_channel(Channel {
                id: channel_id.clone(),
                guild_id: Some(guild_id.clone()),
                kind: 0,
                name: Some("general".to_string()),
                ..Channel::default()
            })
            .await;
        cache
            .upsert_channel(Channel {
                id: other_channel_id.clone(),
                guild_id: Some(other_guild_id.clone()),
                kind: 0,
                name: Some("other-general".to_string()),
                ..Channel::default()
            })
            .await;
        cache
            .upsert_channel(Channel {
                id: dm_channel_id.clone(),
                kind: 1,
                name: Some("dm".to_string()),
                ..Channel::default()
            })
            .await;
        cache
            .upsert_member(
                guild_id.clone(),
                user_id.clone(),
                crate::model::Member {
                    user: Some(User {
                        id: user_id.clone(),
                        username: "discordrs".to_string(),
                        ..User::default()
                    }),
                    ..crate::model::Member::default()
                },
            )
            .await;
        cache
            .upsert_user(User {
                id: user_id.clone(),
                username: "discordrs".to_string(),
                ..User::default()
            })
            .await;
        cache
            .upsert_member(
                other_guild_id.clone(),
                other_user_id.clone(),
                crate::model::Member {
                    user: Some(User {
                        id: other_user_id.clone(),
                        username: "other".to_string(),
                        ..User::default()
                    }),
                    ..crate::model::Member::default()
                },
            )
            .await;
        cache
            .upsert_user(User {
                id: other_user_id.clone(),
                username: "other".to_string(),
                ..User::default()
            })
            .await;
        cache
            .upsert_message(Message {
                id: message_id.clone(),
                channel_id: channel_id.clone(),
                guild_id: Some(guild_id.clone()),
                content: "hello".to_string(),
                ..Message::default()
            })
            .await;
        cache
            .upsert_message(Message {
                id: orphan_message_id.clone(),
                channel_id: orphan_channel_id.clone(),
                guild_id: Some(guild_id.clone()),
                content: "orphan".to_string(),
                ..Message::default()
            })
            .await;
        cache
            .upsert_message(Message {
                id: other_message_id.clone(),
                channel_id: other_channel_id.clone(),
                guild_id: Some(other_guild_id.clone()),
                content: "other".to_string(),
                ..Message::default()
            })
            .await;
        cache
            .upsert_message(Message {
                id: dm_message_id.clone(),
                channel_id: dm_channel_id.clone(),
                content: "dm".to_string(),
                ..Message::default()
            })
            .await;
        cache
            .upsert_presence(
                guild_id.clone(),
                user_id.clone(),
                Presence {
                    user_id: Some(user_id.clone()),
                    status: Some("online".to_string()),
                    ..Presence::default()
                },
            )
            .await;
        cache
            .upsert_presence(
                other_guild_id.clone(),
                other_user_id.clone(),
                Presence {
                    user_id: Some(other_user_id.clone()),
                    status: Some("idle".to_string()),
                    ..Presence::default()
                },
            )
            .await;
        cache
            .upsert_voice_state(
                guild_id.clone(),
                user_id.clone(),
                VoiceState {
                    guild_id: Some(guild_id.clone()),
                    channel_id: Some(channel_id.clone()),
                    user_id: Some(user_id.clone()),
                    ..VoiceState::default()
                },
            )
            .await;
        cache
            .upsert_voice_state(
                other_guild_id.clone(),
                other_user_id.clone(),
                VoiceState {
                    guild_id: Some(other_guild_id.clone()),
                    channel_id: Some(other_channel_id.clone()),
                    user_id: Some(other_user_id.clone()),
                    ..VoiceState::default()
                },
            )
            .await;

        assert!(cache.guild(&guild_id).await.is_some());
        assert!(cache.channel(&channel_id).await.is_some());
        assert!(cache.user(&user_id).await.is_some());
        assert!(cache.member(&guild_id, &user_id).await.is_some());
        assert!(cache.message(&channel_id, &message_id).await.is_some());
        assert!(cache.presence(&guild_id, &user_id).await.is_some());
        assert!(cache.voice_state(&guild_id, &user_id).await.is_some());
        assert!(cache
            .message(&orphan_channel_id, &orphan_message_id)
            .await
            .is_some());
        assert_eq!(cache.roles(&guild_id).await.len(), 1);

        cache.remove_guild(&guild_id).await;
        assert!(cache.guild(&guild_id).await.is_none());
        assert!(cache.channel(&channel_id).await.is_none());
        assert!(cache.user(&user_id).await.is_some());
        assert!(cache.member(&guild_id, &user_id).await.is_none());
        assert!(cache.message(&channel_id, &message_id).await.is_none());
        assert!(cache.presence(&guild_id, &user_id).await.is_none());
        assert!(cache.voice_state(&guild_id, &user_id).await.is_none());
        assert!(cache
            .message(&orphan_channel_id, &orphan_message_id)
            .await
            .is_none());
        assert!(cache.roles(&guild_id).await.is_empty());
        assert!(cache.guild(&other_guild_id).await.is_some());
        assert!(cache.channel(&other_channel_id).await.is_some());
        assert!(cache.channel(&dm_channel_id).await.is_some());
        assert!(cache.user(&other_user_id).await.is_some());
        assert!(cache
            .member(&other_guild_id, &other_user_id)
            .await
            .is_some());
        assert!(cache
            .message(&other_channel_id, &other_message_id)
            .await
            .is_some());
        assert!(cache
            .message(&dm_channel_id, &dm_message_id)
            .await
            .is_some());
        assert!(cache.role(&other_guild_id, &other_role_id).await.is_some());
        assert!(cache
            .presence(&other_guild_id, &other_user_id)
            .await
            .is_some());
        assert!(cache
            .voice_state(&other_guild_id, &other_user_id)
            .await
            .is_some());
    }

    #[tokio::test]
    async fn cache_handle_exposes_contains_and_list_helpers() {
        let cache = CacheHandle::new();
        let guild_id = Snowflake::from("1");
        let channel_id = Snowflake::from("2");
        let message_id = Snowflake::from("3");
        let user_id = Snowflake::from("4");
        let role_id = Snowflake::from("5");

        cache
            .upsert_guild(Guild {
                id: guild_id.clone(),
                name: "discordrs".to_string(),
                ..Guild::default()
            })
            .await;
        cache
            .upsert_channel(Channel {
                id: channel_id.clone(),
                guild_id: Some(guild_id.clone()),
                kind: 0,
                name: Some("general".to_string()),
                ..Channel::default()
            })
            .await;
        cache
            .upsert_member(
                guild_id.clone(),
                user_id.clone(),
                crate::model::Member {
                    user: Some(User {
                        id: user_id.clone(),
                        username: "discordrs".to_string(),
                        ..User::default()
                    }),
                    ..crate::model::Member::default()
                },
            )
            .await;
        cache
            .upsert_user(User {
                id: user_id.clone(),
                username: "discordrs".to_string(),
                ..User::default()
            })
            .await;
        cache
            .upsert_message(Message {
                id: message_id.clone(),
                channel_id: channel_id.clone(),
                guild_id: Some(guild_id.clone()),
                content: "hello".to_string(),
                ..Message::default()
            })
            .await;
        cache
            .upsert_role(
                guild_id.clone(),
                Role {
                    id: role_id.clone(),
                    name: "admin".to_string(),
                    ..Role::default()
                },
            )
            .await;
        cache
            .upsert_presence(
                guild_id.clone(),
                user_id.clone(),
                Presence {
                    user_id: Some(user_id.clone()),
                    status: Some("online".to_string()),
                    ..Presence::default()
                },
            )
            .await;
        cache
            .upsert_voice_state(
                guild_id.clone(),
                user_id.clone(),
                VoiceState {
                    guild_id: Some(guild_id.clone()),
                    channel_id: Some(channel_id.clone()),
                    user_id: Some(user_id.clone()),
                    ..VoiceState::default()
                },
            )
            .await;

        assert!(cache.contains_guild(&guild_id).await);
        assert!(cache.contains_channel(&channel_id).await);
        assert!(cache.contains_user(&user_id).await);
        assert!(cache.contains_member(&guild_id, &user_id).await);
        assert!(cache.contains_message(&channel_id, &message_id).await);
        assert!(cache.contains_role(&guild_id, &role_id).await);
        assert!(cache.contains_presence(&guild_id, &user_id).await);
        assert!(cache.contains_voice_state(&guild_id, &user_id).await);
        assert_eq!(cache.guilds().await.len(), 1);
        assert_eq!(cache.channels().await.len(), 1);
        assert_eq!(cache.users().await.len(), 1);
        assert_eq!(cache.members(&guild_id).await.len(), 1);
        assert_eq!(cache.messages(&channel_id).await.len(), 1);
        assert_eq!(cache.roles(&guild_id).await.len(), 1);
        assert_eq!(cache.presences(&guild_id).await.len(), 1);
        assert_eq!(cache.voice_states(&guild_id).await.len(), 1);

        cache.clear().await;
        assert!(cache.guilds().await.is_empty());
        assert!(cache.channels().await.is_empty());
        assert!(cache.users().await.is_empty());
        assert!(cache.presences(&guild_id).await.is_empty());
        assert!(cache.voice_states(&guild_id).await.is_empty());
    }

    #[tokio::test]
    async fn cache_config_enforces_message_presence_and_member_size_limits() {
        let cache = CacheHandle::with_config(
            CacheConfig::unbounded()
                .max_messages_per_channel(2)
                .max_total_messages(3)
                .max_presences(2)
                .max_members_per_guild(2),
        );
        assert_eq!(cache.config().max_total_messages, Some(3));
        let guild_id = Snowflake::from("1");
        let channel_id = Snowflake::from("10");
        let other_channel_id = Snowflake::from("20");

        for id in ["100", "101", "102"] {
            cache
                .upsert_message(Message {
                    id: Snowflake::from(id),
                    channel_id: channel_id.clone(),
                    guild_id: Some(guild_id.clone()),
                    content: id.to_string(),
                    ..Message::default()
                })
                .await;
        }
        assert!(cache
            .message(&channel_id, &Snowflake::from("100"))
            .await
            .is_none());
        assert_eq!(cache.messages(&channel_id).await.len(), 2);

        for id in ["200", "201"] {
            cache
                .upsert_message(Message {
                    id: Snowflake::from(id),
                    channel_id: other_channel_id.clone(),
                    guild_id: Some(guild_id.clone()),
                    content: id.to_string(),
                    ..Message::default()
                })
                .await;
        }
        let total_messages =
            cache.messages(&channel_id).await.len() + cache.messages(&other_channel_id).await.len();
        assert_eq!(total_messages, 3);

        for id in ["300", "301", "302"] {
            let user_id = Snowflake::from(id);
            cache
                .upsert_presence(
                    guild_id.clone(),
                    user_id.clone(),
                    Presence {
                        user_id: Some(user_id),
                        status: Some("online".to_string()),
                        ..Presence::default()
                    },
                )
                .await;
        }
        assert_eq!(cache.presences(&guild_id).await.len(), 2);
        assert!(cache
            .presence(&guild_id, &Snowflake::from("300"))
            .await
            .is_none());

        for id in ["400", "401", "402"] {
            let user_id = Snowflake::from(id);
            cache
                .upsert_member(
                    guild_id.clone(),
                    user_id.clone(),
                    crate::model::Member {
                        user: Some(User {
                            id: user_id,
                            username: id.to_string(),
                            ..User::default()
                        }),
                        ..crate::model::Member::default()
                    },
                )
                .await;
        }
        assert_eq!(cache.members(&guild_id).await.len(), 2);
        assert!(cache
            .member(&guild_id, &Snowflake::from("400"))
            .await
            .is_none());
    }

    #[tokio::test]
    async fn cache_config_ttl_expires_message_presence_and_member_entries() {
        let cache = CacheHandle::with_config(
            CacheConfig::unbounded()
                .message_ttl(Duration::ZERO)
                .presence_ttl(Duration::ZERO)
                .member_ttl(Duration::ZERO),
        );
        let guild_id = Snowflake::from("1");
        let channel_id = Snowflake::from("10");
        let user_id = Snowflake::from("20");
        let message_id = Snowflake::from("30");

        cache
            .upsert_message(Message {
                id: message_id.clone(),
                channel_id: channel_id.clone(),
                guild_id: Some(guild_id.clone()),
                content: "expired".to_string(),
                ..Message::default()
            })
            .await;
        cache
            .upsert_presence(
                guild_id.clone(),
                user_id.clone(),
                Presence {
                    user_id: Some(user_id.clone()),
                    status: Some("online".to_string()),
                    ..Presence::default()
                },
            )
            .await;
        cache
            .upsert_member(
                guild_id.clone(),
                user_id.clone(),
                crate::model::Member {
                    user: Some(User {
                        id: user_id.clone(),
                        username: "expired".to_string(),
                        ..User::default()
                    }),
                    ..crate::model::Member::default()
                },
            )
            .await;

        cache.purge_expired().await;

        assert!(cache.message(&channel_id, &message_id).await.is_none());
        assert!(cache.presence(&guild_id, &user_id).await.is_none());
        assert!(cache.member(&guild_id, &user_id).await.is_none());
        assert!(cache.messages(&channel_id).await.is_empty());
        assert!(cache.presences(&guild_id).await.is_empty());
        assert!(cache.members(&guild_id).await.is_empty());
    }

    #[tokio::test]
    async fn cache_config_enforces_core_and_metadata_size_limits() {
        let cache = CacheHandle::with_config(
            CacheConfig::unbounded()
                .max_guilds(1)
                .max_channels(1)
                .max_users(1)
                .max_roles(1)
                .max_voice_states(1)
                .max_soundboard_sounds(1)
                .max_emojis(1)
                .max_stickers(1)
                .max_scheduled_events(1)
                .max_stage_instances(1),
        );
        let guild_id = Snowflake::from("1");

        for id in ["1", "2"] {
            cache
                .upsert_guild(Guild {
                    id: Snowflake::from(id),
                    name: format!("guild-{id}"),
                    ..Guild::default()
                })
                .await;
        }
        assert!(cache.guild(&Snowflake::from("1")).await.is_none());
        assert!(cache.guild(&Snowflake::from("2")).await.is_some());

        for id in ["10", "11"] {
            cache
                .upsert_channel(Channel {
                    id: Snowflake::from(id),
                    guild_id: Some(guild_id.clone()),
                    kind: 0,
                    ..Channel::default()
                })
                .await;
        }
        assert!(cache.channel(&Snowflake::from("10")).await.is_none());
        assert!(cache.channel(&Snowflake::from("11")).await.is_some());

        for id in ["20", "21"] {
            cache
                .upsert_user(User {
                    id: Snowflake::from(id),
                    username: format!("user-{id}"),
                    ..User::default()
                })
                .await;
        }
        assert!(cache.user(&Snowflake::from("20")).await.is_none());
        assert!(cache.user(&Snowflake::from("21")).await.is_some());

        for id in ["30", "31"] {
            cache
                .upsert_role(
                    guild_id.clone(),
                    Role {
                        id: Snowflake::from(id),
                        name: format!("role-{id}"),
                        ..Role::default()
                    },
                )
                .await;
        }
        assert!(cache
            .role(&guild_id, &Snowflake::from("30"))
            .await
            .is_none());
        assert!(cache
            .role(&guild_id, &Snowflake::from("31"))
            .await
            .is_some());

        for id in ["40", "41"] {
            cache
                .upsert_voice_state(
                    guild_id.clone(),
                    Snowflake::from(id),
                    VoiceState {
                        user_id: Some(Snowflake::from(id)),
                        guild_id: Some(guild_id.clone()),
                        ..VoiceState::default()
                    },
                )
                .await;
        }
        assert!(cache
            .voice_state(&guild_id, &Snowflake::from("40"))
            .await
            .is_none());
        assert!(cache
            .voice_state(&guild_id, &Snowflake::from("41"))
            .await
            .is_some());

        for id in ["50", "51"] {
            cache
                .upsert_soundboard_sound(
                    guild_id.clone(),
                    SoundboardSound {
                        name: format!("sound-{id}"),
                        sound_id: Snowflake::from(id),
                        volume: 1.0,
                        ..SoundboardSound::default()
                    },
                )
                .await;
        }
        assert!(cache
            .soundboard_sound(&guild_id, &Snowflake::from("50"))
            .await
            .is_none());
        assert!(cache
            .soundboard_sound(&guild_id, &Snowflake::from("51"))
            .await
            .is_some());

        cache
            .replace_emojis(
                guild_id.clone(),
                vec![
                    Emoji::custom("first", "60", false),
                    Emoji::custom("second", "61", false),
                ],
            )
            .await;
        assert!(cache
            .emoji(&guild_id, &Snowflake::from("60"))
            .await
            .is_none());
        assert!(cache
            .emoji(&guild_id, &Snowflake::from("61"))
            .await
            .is_some());

        cache
            .replace_stickers(
                guild_id.clone(),
                vec![
                    Sticker {
                        id: Snowflake::from("70"),
                        name: "first".to_string(),
                        ..Sticker::default()
                    },
                    Sticker {
                        id: Snowflake::from("71"),
                        name: "second".to_string(),
                        ..Sticker::default()
                    },
                ],
            )
            .await;
        assert!(cache
            .sticker(&guild_id, &Snowflake::from("70"))
            .await
            .is_none());
        assert!(cache
            .sticker(&guild_id, &Snowflake::from("71"))
            .await
            .is_some());

        for id in ["80", "81"] {
            cache
                .upsert_scheduled_event(ScheduledEvent {
                    id: Some(Snowflake::from(id)),
                    guild_id: Some(guild_id.clone()),
                    name: Some(format!("event-{id}")),
                    ..ScheduledEvent::default()
                })
                .await;
        }
        assert!(cache
            .scheduled_event(&guild_id, &Snowflake::from("80"))
            .await
            .is_none());
        assert!(cache
            .scheduled_event(&guild_id, &Snowflake::from("81"))
            .await
            .is_some());

        for id in ["90", "91"] {
            cache
                .upsert_stage_instance(StageInstance {
                    id: Snowflake::from(id),
                    guild_id: guild_id.clone(),
                    channel_id: Snowflake::from("11"),
                    topic: format!("stage-{id}"),
                    privacy_level: 2,
                    ..StageInstance::default()
                })
                .await;
        }
        assert!(cache
            .stage_instance(&guild_id, &Snowflake::from("90"))
            .await
            .is_none());
        assert!(cache
            .stage_instance(&guild_id, &Snowflake::from("91"))
            .await
            .is_some());
    }

    #[tokio::test]
    async fn remove_channel_cascades_messages_and_bulk_delete_only_targets_selected_ids() {
        let cache = CacheHandle::new();
        let channel_id = Snowflake::from("2");
        let other_channel_id = Snowflake::from("3");
        let first_message_id = Snowflake::from("10");
        let second_message_id = Snowflake::from("11");
        let untouched_message_id = Snowflake::from("12");

        cache
            .upsert_channel(Channel {
                id: channel_id.clone(),
                kind: 0,
                name: Some("general".to_string()),
                ..Channel::default()
            })
            .await;
        cache
            .upsert_channel(Channel {
                id: other_channel_id.clone(),
                kind: 0,
                name: Some("random".to_string()),
                ..Channel::default()
            })
            .await;
        for (message_id, stored_channel_id) in [
            (first_message_id.clone(), channel_id.clone()),
            (second_message_id.clone(), channel_id.clone()),
            (untouched_message_id.clone(), other_channel_id.clone()),
        ] {
            cache
                .upsert_message(Message {
                    id: message_id,
                    channel_id: stored_channel_id,
                    content: "hello".to_string(),
                    ..Message::default()
                })
                .await;
        }

        cache
            .remove_messages_bulk(&channel_id, std::slice::from_ref(&first_message_id))
            .await;
        assert!(cache
            .message(&channel_id, &first_message_id)
            .await
            .is_none());
        assert!(cache
            .message(&channel_id, &second_message_id)
            .await
            .is_some());
        assert!(cache
            .message(&other_channel_id, &untouched_message_id)
            .await
            .is_some());

        cache.remove_channel(&channel_id).await;
        assert!(cache.channel(&channel_id).await.is_none());
        assert!(cache
            .message(&channel_id, &second_message_id)
            .await
            .is_none());
        assert!(cache
            .message(&other_channel_id, &untouched_message_id)
            .await
            .is_some());
    }

    #[tokio::test]
    async fn cache_handle_removes_individual_entries_without_touching_other_guild_data() {
        let cache = CacheHandle::new();
        let guild_id = Snowflake::from("1");
        let other_guild_id = Snowflake::from("2");
        let channel_id = Snowflake::from("10");
        let other_channel_id = Snowflake::from("20");
        let user_id = Snowflake::from("11");
        let other_user_id = Snowflake::from("21");
        let message_id = Snowflake::from("12");
        let other_message_id = Snowflake::from("22");
        let role_id = Snowflake::from("13");
        let other_role_id = Snowflake::from("23");

        for (id, name) in [
            (guild_id.clone(), "discordrs"),
            (other_guild_id.clone(), "other"),
        ] {
            cache
                .upsert_guild(Guild {
                    id,
                    name: name.to_string(),
                    ..Guild::default()
                })
                .await;
        }

        for (id, guild, name) in [
            (channel_id.clone(), Some(guild_id.clone()), "general"),
            (
                other_channel_id.clone(),
                Some(other_guild_id.clone()),
                "other-general",
            ),
        ] {
            cache
                .upsert_channel(Channel {
                    id,
                    guild_id: guild,
                    kind: 0,
                    name: Some(name.to_string()),
                    ..Channel::default()
                })
                .await;
        }

        for (guild, user, username) in [
            (guild_id.clone(), user_id.clone(), "discordrs"),
            (other_guild_id.clone(), other_user_id.clone(), "other"),
        ] {
            cache
                .upsert_member(
                    guild,
                    user.clone(),
                    crate::model::Member {
                        user: Some(User {
                            id: user,
                            username: username.to_string(),
                            ..User::default()
                        }),
                        ..crate::model::Member::default()
                    },
                )
                .await;
        }

        for (message_id, channel_id, guild_id, content) in [
            (
                message_id.clone(),
                channel_id.clone(),
                Some(guild_id.clone()),
                "hello",
            ),
            (
                other_message_id.clone(),
                other_channel_id.clone(),
                Some(other_guild_id.clone()),
                "other",
            ),
        ] {
            cache
                .upsert_message(Message {
                    id: message_id,
                    channel_id,
                    guild_id,
                    content: content.to_string(),
                    ..Message::default()
                })
                .await;
        }

        for (guild_id, role_id, name) in [
            (guild_id.clone(), role_id.clone(), "admin"),
            (other_guild_id.clone(), other_role_id.clone(), "member"),
        ] {
            cache
                .upsert_role(
                    guild_id,
                    Role {
                        id: role_id,
                        name: name.to_string(),
                        ..Role::default()
                    },
                )
                .await;
        }

        assert_eq!(cache.members(&guild_id).await.len(), 1);
        assert_eq!(cache.messages(&channel_id).await.len(), 1);
        assert_eq!(cache.roles(&guild_id).await.len(), 1);

        cache.remove_member(&guild_id, &user_id).await;
        cache.remove_message(&channel_id, &message_id).await;
        cache.remove_role(&guild_id, &role_id).await;

        assert!(cache.member(&guild_id, &user_id).await.is_none());
        assert!(cache.message(&channel_id, &message_id).await.is_none());
        assert!(cache.role(&guild_id, &role_id).await.is_none());
        assert!(!cache.contains_member(&guild_id, &user_id).await);
        assert!(!cache.contains_message(&channel_id, &message_id).await);
        assert!(!cache.contains_role(&guild_id, &role_id).await);
        assert!(cache.members(&guild_id).await.is_empty());
        assert!(cache.messages(&channel_id).await.is_empty());
        assert!(cache.roles(&guild_id).await.is_empty());

        assert!(cache
            .member(&other_guild_id, &other_user_id)
            .await
            .is_some());
        assert!(cache
            .message(&other_channel_id, &other_message_id)
            .await
            .is_some());
        assert!(cache.role(&other_guild_id, &other_role_id).await.is_some());
    }

    #[cfg(feature = "gateway")]
    #[tokio::test]
    async fn managers_return_cached_values_without_hitting_http() {
        let cache = CacheHandle::new();
        let http = Arc::new(DiscordHttpClient::new("token", 1));
        let guild_id = Snowflake::from("100");
        let channel_id = Snowflake::from("200");
        let user_id = Snowflake::from("300");
        let message_id = Snowflake::from("400");
        let role_id = Snowflake::from("500");

        let guild = Guild {
            id: guild_id.clone(),
            name: "discordrs".to_string(),
            ..Guild::default()
        };
        let channel = Channel {
            id: channel_id.clone(),
            guild_id: Some(guild_id.clone()),
            kind: 0,
            name: Some("general".to_string()),
            ..Channel::default()
        };
        let member = crate::model::Member {
            user: Some(User {
                id: user_id.clone(),
                username: "discordrs".to_string(),
                ..User::default()
            }),
            ..crate::model::Member::default()
        };
        let user = User {
            id: user_id.clone(),
            username: "discordrs".to_string(),
            ..User::default()
        };
        let message = Message {
            id: message_id.clone(),
            channel_id: channel_id.clone(),
            guild_id: Some(guild_id.clone()),
            content: "cached".to_string(),
            ..Message::default()
        };
        let role = Role {
            id: role_id.clone(),
            name: "admin".to_string(),
            ..Role::default()
        };

        cache.upsert_guild(guild.clone()).await;
        cache.upsert_channel(channel.clone()).await;
        cache.upsert_user(user.clone()).await;
        cache
            .upsert_member(guild_id.clone(), user_id.clone(), member.clone())
            .await;
        cache.upsert_message(message.clone()).await;
        cache.upsert_role(guild_id.clone(), role.clone()).await;

        let guild_manager = GuildManager::new(Arc::clone(&http), cache.clone());
        let channel_manager = ChannelManager::new(Arc::clone(&http), cache.clone());
        let user_manager = UserManager::new(Arc::clone(&http), cache.clone());
        let member_manager = MemberManager::new(Arc::clone(&http), cache.clone());
        let message_manager = MessageManager::new(Arc::clone(&http), cache.clone());
        let role_manager = RoleManager::new(http, cache.clone());

        assert_eq!(
            guild_manager.get(guild_id.clone()).await.unwrap().name,
            "discordrs"
        );
        assert_eq!(
            channel_manager
                .get(channel_id.clone())
                .await
                .unwrap()
                .name
                .as_deref(),
            Some("general")
        );
        assert_eq!(
            member_manager
                .get(guild_id.clone(), user_id.clone())
                .await
                .unwrap()
                .user
                .as_ref()
                .map(|user| user.username.as_str()),
            Some("discordrs")
        );
        assert_eq!(
            user_manager.get(user_id.clone()).await.unwrap().username,
            "discordrs"
        );
        assert_eq!(
            message_manager
                .get(channel_id.clone(), message_id.clone())
                .await
                .unwrap()
                .content,
            "cached"
        );
        assert_eq!(role_manager.list(guild_id.clone()).await.unwrap().len(), 1);

        assert!(guild_manager.contains(guild_id.clone()).await);
        assert!(channel_manager.contains(channel_id.clone()).await);
        assert!(user_manager.contains(user_id.clone()).await);
        assert!(
            member_manager
                .contains(guild_id.clone(), user_id.clone())
                .await
        );
        assert!(
            message_manager
                .contains(channel_id.clone(), message_id.clone())
                .await
        );
        assert!(
            role_manager
                .contains(guild_id.clone(), role_id.clone())
                .await
        );

        assert_eq!(
            guild_manager.cached(guild_id.clone()).await.unwrap().id,
            guild_id
        );
        assert_eq!(
            channel_manager.cached(channel_id.clone()).await.unwrap().id,
            channel_id
        );
        assert_eq!(
            user_manager.cached(user_id.clone()).await.unwrap().id,
            user_id
        );
        assert_eq!(
            member_manager
                .cached(guild_id.clone(), user_id.clone())
                .await
                .unwrap()
                .user
                .as_ref()
                .map(|user| user.id.clone()),
            Some(user_id.clone())
        );
        assert_eq!(
            message_manager
                .cached(channel_id.clone(), message_id.clone())
                .await
                .unwrap()
                .id,
            message_id
        );
        assert_eq!(
            role_manager
                .cached(guild_id.clone(), role_id.clone())
                .await
                .unwrap()
                .id,
            role_id
        );

        assert_eq!(guild_manager.list_cached().await.len(), 1);
        assert_eq!(channel_manager.list_cached().await.len(), 1);
        assert_eq!(user_manager.list_cached().await.len(), 1);
        assert_eq!(member_manager.list_cached(guild_id.clone()).await.len(), 1);
        assert_eq!(
            message_manager.list_cached(channel_id.clone()).await.len(),
            1
        );
        assert_eq!(role_manager.list_cached(guild_id.clone()).await.len(), 1);
    }

    #[cfg(feature = "gateway")]
    #[tokio::test]
    async fn cached_manager_trait_impls_delegate_to_cache_for_hits() {
        let cache = CacheHandle::new();
        let http = Arc::new(DiscordHttpClient::new("token", 1));
        let guild_id = Snowflake::from("701");
        let channel_id = Snowflake::from("702");
        let user_id = Snowflake::from("703");

        cache
            .upsert_guild(Guild {
                id: guild_id.clone(),
                name: "guild".to_string(),
                ..Guild::default()
            })
            .await;
        cache
            .upsert_channel(Channel {
                id: channel_id.clone(),
                guild_id: Some(guild_id.clone()),
                kind: 0,
                name: Some("cached-channel".to_string()),
                ..Channel::default()
            })
            .await;
        cache
            .upsert_user(User {
                id: user_id.clone(),
                username: "cached-user".to_string(),
                ..User::default()
            })
            .await;

        let guild_manager = GuildManager::new(Arc::clone(&http), cache.clone());
        let channel_manager = ChannelManager::new(Arc::clone(&http), cache.clone());
        let user_manager = UserManager::new(http, cache);

        assert_eq!(
            <GuildManager as CachedManager<Guild>>::get(&guild_manager, guild_id.clone())
                .await
                .unwrap()
                .name,
            "guild"
        );
        assert_eq!(
            <GuildManager as CachedManager<Guild>>::cached(&guild_manager, guild_id.clone())
                .await
                .unwrap()
                .id,
            guild_id
        );
        assert!(
            <GuildManager as CachedManager<Guild>>::contains(&guild_manager, guild_id.clone())
                .await
        );
        assert_eq!(
            <GuildManager as CachedManager<Guild>>::list_cached(&guild_manager)
                .await
                .len(),
            1
        );

        assert_eq!(
            <ChannelManager as CachedManager<Channel>>::get(&channel_manager, channel_id.clone())
                .await
                .unwrap()
                .name
                .as_deref(),
            Some("cached-channel")
        );
        assert_eq!(
            <ChannelManager as CachedManager<Channel>>::cached(
                &channel_manager,
                channel_id.clone()
            )
            .await
            .unwrap()
            .id,
            channel_id
        );
        assert!(
            <ChannelManager as CachedManager<Channel>>::contains(
                &channel_manager,
                channel_id.clone()
            )
            .await
        );
        assert_eq!(
            <ChannelManager as CachedManager<Channel>>::list_cached(&channel_manager)
                .await
                .len(),
            1
        );

        assert_eq!(
            <UserManager as CachedManager<User>>::get(&user_manager, user_id.clone())
                .await
                .unwrap()
                .username,
            "cached-user"
        );
        assert_eq!(
            <UserManager as CachedManager<User>>::cached(&user_manager, user_id.clone())
                .await
                .unwrap()
                .id,
            user_id
        );
        assert!(
            <UserManager as CachedManager<User>>::contains(&user_manager, user_id.clone()).await
        );
        assert_eq!(
            <UserManager as CachedManager<User>>::list_cached(&user_manager)
                .await
                .len(),
            1
        );
    }
}
