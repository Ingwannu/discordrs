use std::sync::Arc;

#[cfg(feature = "cache")]
use std::collections::{HashMap, HashSet};
#[cfg(feature = "cache")]
use tokio::sync::RwLock;

#[cfg(feature = "gateway")]
use async_trait::async_trait;

use crate::error::DiscordError;
use crate::http::DiscordHttpClient;
use crate::model::{
    Channel, Guild, Member, Message, Presence, Role, Snowflake, SoundboardSound, User, VoiceState,
};

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
}

#[cfg(feature = "cache")]
fn evict_channel_entries(store: &mut CacheStore, channel_id: &Snowflake) {
    store.channels.remove(channel_id);
    store
        .messages
        .retain(|(stored_channel_id, _), _| stored_channel_id != channel_id);
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
    store
        .channels
        .retain(|_, channel| channel.guild_id.as_ref() != Some(guild_id));
    store
        .members
        .retain(|(stored_guild_id, _), _| stored_guild_id != guild_id);
    store.messages.retain(|(stored_channel_id, _), message| {
        !removed_channel_ids.contains(stored_channel_id)
            && message.guild_id.as_ref() != Some(guild_id)
    });
    store
        .roles
        .retain(|(stored_guild_id, _), _| stored_guild_id != guild_id);
    store
        .presences
        .retain(|(stored_guild_id, _), _| stored_guild_id != guild_id);
    store
        .voice_states
        .retain(|(stored_guild_id, _), _| stored_guild_id != guild_id);
    store
        .soundboard_sounds
        .retain(|(stored_guild_id, _), _| stored_guild_id != guild_id);
}

#[derive(Clone, Default)]
pub struct CacheHandle {
    #[cfg(feature = "cache")]
    store: Arc<RwLock<CacheStore>>,
}

impl CacheHandle {
    pub fn new() -> Self {
        Self::default()
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
    }

    #[cfg(not(feature = "cache"))]
    pub async fn clear(&self) {}

    #[cfg(feature = "cache")]
    pub async fn upsert_guild(&self, guild: Guild) {
        self.store
            .write()
            .await
            .guilds
            .insert(guild.id.clone(), guild);
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
        self.store
            .write()
            .await
            .channels
            .insert(channel.id.clone(), channel);
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
        self.store.write().await.users.insert(user.id.clone(), user);
    }

    #[cfg(not(feature = "cache"))]
    pub async fn upsert_user(&self, _user: User) {}

    #[cfg(feature = "cache")]
    pub async fn remove_user(&self, user_id: &Snowflake) {
        self.store.write().await.users.remove(user_id);
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
        self.store
            .write()
            .await
            .members
            .insert((guild_id, user_id), member);
    }

    #[cfg(not(feature = "cache"))]
    pub async fn upsert_member(&self, _guild_id: Snowflake, _user_id: Snowflake, _member: Member) {}

    #[cfg(feature = "cache")]
    pub async fn remove_member(&self, guild_id: &Snowflake, user_id: &Snowflake) {
        self.store
            .write()
            .await
            .members
            .remove(&(guild_id.clone(), user_id.clone()));
    }

    #[cfg(not(feature = "cache"))]
    pub async fn remove_member(&self, _guild_id: &Snowflake, _user_id: &Snowflake) {}

    #[cfg(not(feature = "cache"))]
    pub async fn member(&self, _guild_id: &Snowflake, _user_id: &Snowflake) -> Option<Member> {
        None
    }

    #[cfg(feature = "cache")]
    pub async fn member(&self, guild_id: &Snowflake, user_id: &Snowflake) -> Option<Member> {
        self.store
            .read()
            .await
            .members
            .get(&(guild_id.clone(), user_id.clone()))
            .cloned()
    }

    #[cfg(feature = "cache")]
    pub async fn members(&self, guild_id: &Snowflake) -> Vec<Member> {
        self.store
            .read()
            .await
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
        self.store
            .write()
            .await
            .messages
            .insert((message.channel_id.clone(), message.id.clone()), message);
    }

    #[cfg(not(feature = "cache"))]
    pub async fn upsert_message(&self, _message: Message) {}

    #[cfg(feature = "cache")]
    pub async fn remove_message(&self, channel_id: &Snowflake, message_id: &Snowflake) {
        self.store
            .write()
            .await
            .messages
            .remove(&(channel_id.clone(), message_id.clone()));
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
        self.store
            .read()
            .await
            .messages
            .get(&(channel_id.clone(), message_id.clone()))
            .cloned()
    }

    #[cfg(feature = "cache")]
    pub async fn messages(&self, channel_id: &Snowflake) -> Vec<Message> {
        self.store
            .read()
            .await
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
        self.store
            .write()
            .await
            .roles
            .insert((guild_id, role.id.clone()), role);
    }

    #[cfg(not(feature = "cache"))]
    pub async fn upsert_role(&self, _guild_id: Snowflake, _role: Role) {}

    #[cfg(feature = "cache")]
    pub async fn remove_role(&self, guild_id: &Snowflake, role_id: &Snowflake) {
        self.store
            .write()
            .await
            .roles
            .remove(&(guild_id.clone(), role_id.clone()));
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
        self.store
            .write()
            .await
            .presences
            .insert((guild_id, user_id), presence);
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
        self.store
            .write()
            .await
            .presences
            .remove(&(guild_id.clone(), user_id.clone()));
    }

    #[cfg(not(feature = "cache"))]
    pub async fn remove_presence(&self, _guild_id: &Snowflake, _user_id: &Snowflake) {}

    #[cfg(feature = "cache")]
    pub async fn presence(&self, guild_id: &Snowflake, user_id: &Snowflake) -> Option<Presence> {
        self.store
            .read()
            .await
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
        self.store
            .read()
            .await
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
        self.store
            .write()
            .await
            .voice_states
            .insert((guild_id, user_id), voice_state);
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
        self.store
            .write()
            .await
            .voice_states
            .remove(&(guild_id.clone(), user_id.clone()));
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
        self.store
            .write()
            .await
            .soundboard_sounds
            .insert((guild_id, sound.sound_id.clone()), sound);
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
        for sound in sounds {
            store
                .soundboard_sounds
                .insert((guild_id.clone(), sound.sound_id.clone()), sound);
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
        self.store
            .write()
            .await
            .soundboard_sounds
            .remove(&(guild_id.clone(), sound_id.clone()));
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

    #[cfg(feature = "gateway")]
    use crate::manager::CachedManager;
    use crate::model::{Channel, Guild, Message, Presence, Role, Snowflake, User, VoiceState};

    use super::{
        CacheHandle, ChannelManager, GuildManager, MemberManager, MessageManager, RoleManager,
        UserManager,
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
