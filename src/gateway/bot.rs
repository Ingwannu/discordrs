use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use serde_json::Value;
#[cfg(feature = "sharding")]
use std::sync::Mutex as StdMutex;
#[cfg(feature = "sharding")]
use tokio::sync::watch;
use tokio::sync::{mpsc, RwLock};
#[cfg(feature = "sharding")]
use tokio::task::JoinHandle;
#[cfg(feature = "sharding")]
use tokio::time::{sleep, timeout, Duration};
use tracing::{info, warn};

use crate::cache::{
    CacheHandle, ChannelManager, GuildManager, MemberManager, MessageManager, RoleManager,
};
#[cfg(feature = "collectors")]
use crate::collector::CollectorHub;
use crate::error::DiscordError;
use crate::event::{decode_event, Event};
use crate::http::DiscordHttpClient;
use crate::model::Interaction;
#[cfg(feature = "sharding")]
use crate::sharding::{
    ShardInfo, ShardIpcMessage, ShardRuntimeChannels, ShardRuntimeState, ShardRuntimeStatus,
    ShardSupervisorEvent, ShardingManager,
};
use crate::types::invalid_data_error;
#[cfg(feature = "voice")]
use crate::voice::{AudioTrack, VoiceConnectionConfig, VoiceConnectionState, VoiceManager};
#[cfg(feature = "voice")]
use crate::voice_runtime::{
    connect as connect_voice_runtime_impl, VoiceRuntimeConfig, VoiceRuntimeHandle,
};
use crate::ws::GatewayConnectionConfig;

#[cfg(feature = "sharding")]
use super::client::SupervisorCallback;
use super::client::{voice_state_update_payload, EventCallback, GatewayClient, GatewayCommand};

#[cfg(feature = "sharding")]
pub struct ShardSupervisor {
    manager: Arc<StdMutex<ShardingManager>>,
    tasks: Vec<(u32, JoinHandle<Result<(), DiscordError>>)>,
}

#[cfg(feature = "sharding")]
impl ShardSupervisor {
    const SHUTDOWN_TIMEOUT: Duration = Duration::from_millis(15_000);

    pub fn manager(&self) -> Arc<StdMutex<ShardingManager>> {
        Arc::clone(&self.manager)
    }

    pub fn statuses(&self) -> Vec<ShardRuntimeStatus> {
        self.manager
            .lock()
            .expect("shard manager mutex poisoned")
            .statuses()
    }

    pub fn drain_events(&self) -> Result<Vec<ShardSupervisorEvent>, DiscordError> {
        self.manager
            .lock()
            .expect("shard manager mutex poisoned")
            .drain_events()
    }

    pub fn send(&self, shard_id: u32, message: ShardIpcMessage) -> Result<(), DiscordError> {
        self.manager
            .lock()
            .expect("shard manager mutex poisoned")
            .send(shard_id, message)
    }

    pub fn reconnect(&self, shard_id: u32) -> Result<(), DiscordError> {
        self.send(shard_id, ShardIpcMessage::Reconnect)
    }

    pub fn update_presence(
        &self,
        shard_id: u32,
        status: impl Into<String>,
    ) -> Result<(), DiscordError> {
        self.send(shard_id, ShardIpcMessage::UpdatePresence(status.into()))
    }

    pub fn update_voice_state(
        &self,
        shard_id: u32,
        guild_id: impl Into<crate::model::Snowflake>,
        channel_id: Option<crate::model::Snowflake>,
        self_mute: bool,
        self_deaf: bool,
    ) -> Result<(), DiscordError> {
        self.send(
            shard_id,
            ShardIpcMessage::SendPayload(voice_state_update_payload(
                guild_id.into(),
                channel_id,
                self_mute,
                self_deaf,
            )),
        )
    }

    pub fn join_voice(
        &self,
        shard_id: u32,
        guild_id: impl Into<crate::model::Snowflake>,
        channel_id: impl Into<crate::model::Snowflake>,
        self_mute: bool,
        self_deaf: bool,
    ) -> Result<(), DiscordError> {
        self.update_voice_state(
            shard_id,
            guild_id,
            Some(channel_id.into()),
            self_mute,
            self_deaf,
        )
    }

    pub fn leave_voice(
        &self,
        shard_id: u32,
        guild_id: impl Into<crate::model::Snowflake>,
        self_mute: bool,
        self_deaf: bool,
    ) -> Result<(), DiscordError> {
        self.update_voice_state(shard_id, guild_id, None, self_mute, self_deaf)
    }

    pub fn broadcast(&self, message: ShardIpcMessage) -> Result<(), DiscordError> {
        self.manager
            .lock()
            .expect("shard manager mutex poisoned")
            .broadcast(message)
    }

    pub fn shutdown(&self) -> Result<(), DiscordError> {
        self.broadcast(ShardIpcMessage::Shutdown)
    }

    pub async fn shutdown_and_wait(self) -> Result<(), DiscordError> {
        self.shutdown()?;
        self.wait_for_shutdown(Self::SHUTDOWN_TIMEOUT).await
    }

    pub async fn wait_for_shutdown(self, timeout_duration: Duration) -> Result<(), DiscordError> {
        self.wait_with_timeout(Some(timeout_duration)).await
    }

    pub async fn wait(self) -> Result<(), DiscordError> {
        self.wait_with_timeout(None).await
    }

    async fn wait_with_timeout(
        self,
        timeout_duration: Option<Duration>,
    ) -> Result<(), DiscordError> {
        for (shard_id, task) in self.tasks {
            let mut task = task;
            let result = if let Some(timeout_duration) = timeout_duration {
                match timeout(timeout_duration, &mut task).await {
                    Ok(result) => result,
                    Err(_) => {
                        task.abort();
                        return Err(invalid_data_error(format!(
                            "timed out waiting for shard {shard_id} shutdown after {timeout_duration:?}"
                        )));
                    }
                }
            } else {
                task.await
            };

            match result {
                Ok(Ok(())) => {}
                Ok(Err(error)) => return Err(error),
                Err(error) => return Err(format!("shard task failed: {error}").into()),
            }
        }

        Ok(())
    }
}

pub struct TypeMap(HashMap<TypeId, Box<dyn Any + Send + Sync>>);

#[derive(Clone)]
pub struct ShardMessenger {
    shard_id: u32,
    command_tx: mpsc::UnboundedSender<GatewayCommand>,
}

impl ShardMessenger {
    pub fn shard_id(&self) -> u32 {
        self.shard_id
    }

    pub fn update_presence(&self, status: impl Into<String>) -> Result<(), DiscordError> {
        self.send(GatewayCommand::UpdatePresence(status.into()))
    }

    pub fn update_voice_state(
        &self,
        guild_id: impl Into<crate::model::Snowflake>,
        channel_id: Option<crate::model::Snowflake>,
        self_mute: bool,
        self_deaf: bool,
    ) -> Result<(), DiscordError> {
        self.send(GatewayCommand::SendPayload(voice_state_update_payload(
            guild_id.into(),
            channel_id,
            self_mute,
            self_deaf,
        )))
    }

    pub fn join_voice(
        &self,
        guild_id: impl Into<crate::model::Snowflake>,
        channel_id: impl Into<crate::model::Snowflake>,
        self_mute: bool,
        self_deaf: bool,
    ) -> Result<(), DiscordError> {
        self.update_voice_state(guild_id, Some(channel_id.into()), self_mute, self_deaf)
    }

    pub fn leave_voice(
        &self,
        guild_id: impl Into<crate::model::Snowflake>,
        self_mute: bool,
        self_deaf: bool,
    ) -> Result<(), DiscordError> {
        self.update_voice_state(guild_id, None, self_mute, self_deaf)
    }

    pub fn reconnect(&self) -> Result<(), DiscordError> {
        self.send(GatewayCommand::Reconnect)
    }

    pub fn shutdown(&self) -> Result<(), DiscordError> {
        self.send(GatewayCommand::Shutdown)
    }

    fn send(&self, command: GatewayCommand) -> Result<(), DiscordError> {
        self.command_tx
            .send(command)
            .map_err(|error| invalid_data_error(format!("failed to send gateway command: {error}")))
    }
}

impl TypeMap {
    pub fn new() -> Self {
        Self(HashMap::new())
    }

    pub fn insert<T: Send + Sync + 'static>(&mut self, val: T) {
        self.0.insert(TypeId::of::<T>(), Box::new(val));
    }

    pub fn get<T: Send + Sync + 'static>(&self) -> Option<&T> {
        self.0
            .get(&TypeId::of::<T>())
            .and_then(|b| b.downcast_ref())
    }

    pub fn get_mut<T: Send + Sync + 'static>(&mut self) -> Option<&mut T> {
        self.0
            .get_mut(&TypeId::of::<T>())
            .and_then(|b| b.downcast_mut())
    }
}

impl Default for TypeMap {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone)]
pub struct Context {
    pub http: Arc<DiscordHttpClient>,
    pub data: Arc<RwLock<TypeMap>>,
    pub cache: CacheHandle,
    pub shard_id: u32,
    pub shard_count: u32,
    gateway_commands: Arc<RwLock<HashMap<u32, ShardMessenger>>>,
    #[cfg(feature = "voice")]
    voice: Arc<RwLock<VoiceManager>>,
    #[cfg(feature = "collectors")]
    collectors: CollectorHub,
}

impl Context {
    pub fn new(http: Arc<DiscordHttpClient>, data: Arc<RwLock<TypeMap>>) -> Self {
        Self {
            http,
            data,
            cache: CacheHandle::new(),
            shard_id: 0,
            shard_count: 1,
            gateway_commands: Arc::new(RwLock::new(HashMap::new())),
            #[cfg(feature = "voice")]
            voice: Arc::new(RwLock::new(VoiceManager::new())),
            #[cfg(feature = "collectors")]
            collectors: CollectorHub::new(),
        }
    }

    pub fn rest(&self) -> Arc<DiscordHttpClient> {
        Arc::clone(&self.http)
    }

    pub fn shard_pair(&self) -> (u32, u32) {
        (self.shard_id, self.shard_count)
    }

    #[cfg(feature = "sharding")]
    pub fn shard_info(&self) -> ShardInfo {
        ShardInfo {
            id: self.shard_id,
            total: self.shard_count,
        }
    }

    pub async fn insert_data<T: Send + Sync + 'static>(&self, value: T) {
        self.data.write().await.insert(value);
    }

    pub async fn with_data<R>(&self, map: impl FnOnce(&TypeMap) -> Option<R>) -> Option<R> {
        let data = self.data.read().await;
        map(&data)
    }

    pub async fn get_data_cloned<T>(&self) -> Option<T>
    where
        T: Send + Sync + Clone + 'static,
    {
        let data = self.data.read().await;
        data.get::<T>().cloned()
    }

    pub fn guilds(&self) -> GuildManager {
        GuildManager::new(Arc::clone(&self.http), self.cache.clone())
    }

    pub fn channels(&self) -> ChannelManager {
        ChannelManager::new(Arc::clone(&self.http), self.cache.clone())
    }

    pub fn members(&self) -> MemberManager {
        MemberManager::new(Arc::clone(&self.http), self.cache.clone())
    }

    pub fn messages(&self) -> MessageManager {
        MessageManager::new(Arc::clone(&self.http), self.cache.clone())
    }

    pub fn roles(&self) -> RoleManager {
        RoleManager::new(Arc::clone(&self.http), self.cache.clone())
    }

    pub async fn shard_messenger(&self) -> Option<ShardMessenger> {
        self.gateway_commands
            .read()
            .await
            .get(&self.shard_id)
            .cloned()
    }

    pub async fn update_presence(&self, status: impl Into<String>) -> Result<(), DiscordError> {
        let messenger = self
            .shard_messenger()
            .await
            .ok_or_else(|| invalid_data_error("missing shard messenger"))?;
        messenger.update_presence(status)
    }

    pub async fn reconnect_shard(&self) -> Result<(), DiscordError> {
        let messenger = self
            .shard_messenger()
            .await
            .ok_or_else(|| invalid_data_error("missing shard messenger"))?;
        messenger.reconnect()
    }

    pub async fn shutdown_shard(&self) -> Result<(), DiscordError> {
        let messenger = self
            .shard_messenger()
            .await
            .ok_or_else(|| invalid_data_error("missing shard messenger"))?;
        messenger.shutdown()
    }

    pub async fn update_voice_state(
        &self,
        guild_id: impl Into<crate::model::Snowflake>,
        channel_id: Option<crate::model::Snowflake>,
        self_mute: bool,
        self_deaf: bool,
    ) -> Result<(), DiscordError> {
        let messenger = self
            .shard_messenger()
            .await
            .ok_or_else(|| invalid_data_error("missing shard messenger"))?;
        messenger.update_voice_state(guild_id, channel_id, self_mute, self_deaf)
    }

    pub async fn join_voice(
        &self,
        guild_id: impl Into<crate::model::Snowflake>,
        channel_id: impl Into<crate::model::Snowflake>,
        self_mute: bool,
        self_deaf: bool,
    ) -> Result<(), DiscordError> {
        let messenger = self
            .shard_messenger()
            .await
            .ok_or_else(|| invalid_data_error("missing shard messenger"))?;
        messenger.join_voice(guild_id, channel_id, self_mute, self_deaf)
    }

    pub async fn leave_voice(
        &self,
        guild_id: impl Into<crate::model::Snowflake>,
        self_mute: bool,
        self_deaf: bool,
    ) -> Result<(), DiscordError> {
        let messenger = self
            .shard_messenger()
            .await
            .ok_or_else(|| invalid_data_error("missing shard messenger"))?;
        messenger.leave_voice(guild_id, self_mute, self_deaf)
    }

    #[cfg(feature = "voice")]
    pub fn voice(&self) -> Arc<RwLock<VoiceManager>> {
        Arc::clone(&self.voice)
    }

    #[cfg(feature = "voice")]
    pub async fn connect_voice(&self, config: VoiceConnectionConfig) -> VoiceConnectionState {
        let mut voice = self.voice.write().await;
        voice.connect(config)
    }

    #[cfg(feature = "voice")]
    pub async fn disconnect_voice(
        &self,
        guild_id: impl Into<crate::model::Snowflake>,
    ) -> Option<VoiceConnectionState> {
        let mut voice = self.voice.write().await;
        voice.disconnect(guild_id)
    }

    #[cfg(feature = "voice")]
    pub async fn enqueue_voice_track(
        &self,
        guild_id: impl Into<crate::model::Snowflake>,
        track: AudioTrack,
    ) -> Option<usize> {
        let mut voice = self.voice.write().await;
        voice.enqueue(guild_id, track)
    }

    #[cfg(feature = "voice")]
    pub async fn voice_runtime_config(
        &self,
        guild_id: impl Into<crate::model::Snowflake>,
        user_id: impl Into<crate::model::Snowflake>,
    ) -> Option<VoiceRuntimeConfig> {
        let voice = self.voice.read().await;
        voice.runtime_config(guild_id, user_id)
    }

    #[cfg(feature = "voice")]
    pub async fn connect_voice_runtime(
        &self,
        guild_id: impl Into<crate::model::Snowflake>,
        user_id: impl Into<crate::model::Snowflake>,
    ) -> Result<VoiceRuntimeHandle, DiscordError> {
        let config = self
            .voice_runtime_config(guild_id, user_id)
            .await
            .ok_or_else(|| {
                invalid_data_error("voice runtime requires endpoint, session_id, and token")
            })?;
        connect_voice_runtime_impl(config).await
    }

    #[cfg(feature = "collectors")]
    pub fn collectors(&self) -> &CollectorHub {
        &self.collectors
    }
}

#[async_trait]
pub trait EventHandler: Send + Sync + 'static {
    async fn handle_event(&self, ctx: Context, event: Event) {
        match event {
            Event::Ready(event) => self.ready(ctx, event.data).await,
            Event::GuildCreate(event) => self.guild_create(ctx, event.guild).await,
            Event::GuildUpdate(event) => self.guild_update(ctx, event.guild).await,
            Event::GuildDelete(event) => self.guild_delete(ctx, event.data).await,
            Event::ChannelCreate(event) => self.channel_create(ctx, event.channel).await,
            Event::ChannelUpdate(event) => self.channel_update(ctx, event.channel).await,
            Event::ChannelDelete(event) => self.channel_delete(ctx, event.channel).await,
            Event::MemberAdd(event) => self.member_add(ctx, event.guild_id, event.member).await,
            Event::MemberUpdate(event) => {
                self.member_update(ctx, event.guild_id, event.member).await
            }
            Event::MemberRemove(event) => {
                self.member_remove(ctx, event.data.guild_id, event.data.user)
                    .await
            }
            Event::RoleCreate(event) => self.role_create(ctx, event.guild_id, event.role).await,
            Event::RoleUpdate(event) => self.role_update(ctx, event.guild_id, event.role).await,
            Event::RoleDelete(event) => {
                self.role_delete(ctx, event.data.guild_id, event.data.role_id)
                    .await
            }
            Event::MessageCreate(event) => self.message_create(ctx, event.message).await,
            Event::MessageUpdate(event) => self.message_update(ctx, event.message).await,
            Event::MessageDelete(event) => {
                self.message_delete(ctx, event.data.channel_id, event.data.id)
                    .await
            }
            Event::MessageDeleteBulk(event) => self.message_delete_bulk(ctx, event).await,
            Event::ChannelPinsUpdate(event) => self.channel_pins_update(ctx, event).await,
            Event::GuildBanAdd(event) => self.guild_ban_add(ctx, event).await,
            Event::GuildBanRemove(event) => self.guild_ban_remove(ctx, event).await,
            Event::GuildEmojisUpdate(event) => self.guild_emojis_update(ctx, event).await,
            Event::GuildIntegrationsUpdate(event) => {
                self.guild_integrations_update(ctx, event).await
            }
            Event::WebhooksUpdate(event) => self.webhooks_update(ctx, event).await,
            Event::InviteCreate(event) => self.invite_create(ctx, event).await,
            Event::InviteDelete(event) => self.invite_delete(ctx, event).await,
            Event::VoiceStateUpdate(event) => self.voice_state_update(ctx, event.state).await,
            Event::VoiceServerUpdate(event) => self.voice_server_update(ctx, event.data).await,
            Event::MessageReactionAdd(event) => self.reaction_add(ctx, event).await,
            Event::MessageReactionRemove(event) => self.reaction_remove(ctx, event).await,
            Event::MessageReactionRemoveAll(event) => self.reaction_remove_all(ctx, event).await,
            Event::TypingStart(event) => self.typing_start(ctx, event).await,
            Event::PresenceUpdate(event) => self.presence_update(ctx, event).await,
            Event::InteractionCreate(event) => {
                self.interaction_create(ctx, event.interaction).await
            }
            Event::Unknown { kind, raw } => self.raw_event(ctx, kind, raw).await,
        }
    }

    async fn ready(&self, _ctx: Context, _ready_data: crate::event::ReadyPayload) {}
    async fn guild_create(&self, _ctx: Context, _guild: crate::model::Guild) {}
    async fn guild_update(&self, _ctx: Context, _guild: crate::model::Guild) {}
    async fn guild_delete(&self, _ctx: Context, _data: crate::event::GuildDeletePayload) {}
    async fn channel_create(&self, _ctx: Context, _channel: crate::model::Channel) {}
    async fn channel_update(&self, _ctx: Context, _channel: crate::model::Channel) {}
    async fn channel_delete(&self, _ctx: Context, _channel: crate::model::Channel) {}
    async fn member_add(
        &self,
        _ctx: Context,
        _guild_id: crate::model::Snowflake,
        _member: crate::model::Member,
    ) {
    }
    async fn member_update(
        &self,
        _ctx: Context,
        _guild_id: crate::model::Snowflake,
        _member: crate::model::Member,
    ) {
    }
    async fn member_remove(
        &self,
        _ctx: Context,
        _guild_id: crate::model::Snowflake,
        _user: crate::model::User,
    ) {
    }
    async fn role_create(
        &self,
        _ctx: Context,
        _guild_id: crate::model::Snowflake,
        _role: crate::model::Role,
    ) {
    }
    async fn role_update(
        &self,
        _ctx: Context,
        _guild_id: crate::model::Snowflake,
        _role: crate::model::Role,
    ) {
    }
    async fn role_delete(
        &self,
        _ctx: Context,
        _guild_id: crate::model::Snowflake,
        _role_id: crate::model::Snowflake,
    ) {
    }
    async fn message_create(&self, _ctx: Context, _message: crate::model::Message) {}
    async fn message_update(&self, _ctx: Context, _message: crate::model::Message) {}
    async fn message_delete(
        &self,
        _ctx: Context,
        _channel_id: crate::model::Snowflake,
        _message_id: crate::model::Snowflake,
    ) {
    }
    async fn message_delete_bulk(
        &self,
        _ctx: Context,
        _event: crate::event::BulkMessageDeleteEvent,
    ) {
    }
    async fn channel_pins_update(
        &self,
        _ctx: Context,
        _event: crate::event::ChannelPinsUpdateEvent,
    ) {
    }
    async fn guild_ban_add(&self, _ctx: Context, _event: crate::event::GuildBanEvent) {}
    async fn guild_ban_remove(&self, _ctx: Context, _event: crate::event::GuildBanEvent) {}
    async fn guild_emojis_update(
        &self,
        _ctx: Context,
        _event: crate::event::GuildEmojisUpdateEvent,
    ) {
    }
    async fn guild_integrations_update(
        &self,
        _ctx: Context,
        _event: crate::event::GuildIntegrationsUpdateEvent,
    ) {
    }
    async fn webhooks_update(&self, _ctx: Context, _event: crate::event::WebhooksUpdateEvent) {}
    async fn invite_create(&self, _ctx: Context, _event: crate::event::InviteEvent) {}
    async fn invite_delete(&self, _ctx: Context, _event: crate::event::InviteEvent) {}
    async fn voice_state_update(&self, _ctx: Context, _state: crate::model::VoiceState) {}
    async fn voice_server_update(&self, _ctx: Context, _data: crate::model::VoiceServerUpdate) {}
    async fn reaction_add(&self, _ctx: Context, _data: crate::event::ReactionEvent) {}
    async fn reaction_remove(&self, _ctx: Context, _data: crate::event::ReactionEvent) {}
    async fn reaction_remove_all(
        &self,
        _ctx: Context,
        _event: crate::event::ReactionRemoveAllEvent,
    ) {
    }
    async fn typing_start(&self, _ctx: Context, _data: crate::event::TypingStartEvent) {}
    async fn presence_update(&self, _ctx: Context, _data: crate::event::PresenceUpdateEvent) {}
    async fn interaction_create(&self, _ctx: Context, _interaction: crate::model::Interaction) {}
    async fn raw_event(&self, _ctx: Context, _event_name: String, _data: Value) {}
}

pub struct ClientBuilder {
    token: String,
    intents: u64,
    handler: Option<Arc<dyn EventHandler>>,
    data: TypeMap,
    application_id: Option<u64>,
    gateway_config: GatewayConnectionConfig,
    shard: Option<(u32, u32)>,
}

impl ClientBuilder {
    pub fn event_handler<H: EventHandler>(mut self, handler: H) -> Self {
        self.handler = Some(Arc::new(handler));
        self
    }

    pub fn application_id(mut self, id: u64) -> Self {
        self.application_id = Some(id);
        self
    }

    pub fn type_map_insert<T: Send + Sync + 'static>(mut self, val: T) -> Self {
        self.data.insert(val);
        self
    }

    pub fn gateway_config(mut self, gateway_config: GatewayConnectionConfig) -> Self {
        self.gateway_config = gateway_config;
        self
    }

    pub fn shard(mut self, shard_id: u32, shard_count: u32) -> Self {
        self.shard = Some((shard_id, shard_count.max(1)));
        self
    }

    /// Returns just the REST client without starting a gateway connection.
    pub fn rest_only(self) -> Arc<DiscordHttpClient> {
        let application_id = self.application_id.unwrap_or(0);
        Arc::new(DiscordHttpClient::new(self.token, application_id))
    }

    pub async fn start(self) -> Result<(), DiscordError> {
        let ClientBuilder {
            token,
            intents,
            handler,
            data,
            application_id,
            gateway_config,
            shard,
        } = self;
        let handler = handler.ok_or("event_handler is required")?;
        let application_id = application_id.unwrap_or(0);
        let shard = shard.unwrap_or((0, 1));
        let runtime = SharedRuntime::new(&token, application_id, data);
        #[cfg(feature = "sharding")]
        {
            start_gateway_shard(
                token,
                intents,
                handler,
                runtime,
                gateway_config,
                shard,
                ShardStartControl {
                    supervisor_channels: None,
                    boot_gate: None,
                },
            )
            .await
        }
        #[cfg(not(feature = "sharding"))]
        {
            start_gateway_shard(token, intents, handler, runtime, gateway_config, shard).await
        }
    }

    pub async fn start_shards(self, shard_count: u32) -> Result<(), DiscordError> {
        #[cfg(feature = "sharding")]
        {
            self.spawn_shards(shard_count).await?.wait().await
        }

        #[cfg(not(feature = "sharding"))]
        {
            let _ = shard_count;
            Err("sharding feature is required to start multiple shards".into())
        }
    }

    pub async fn start_auto_shards(self) -> Result<(), DiscordError> {
        #[cfg(feature = "sharding")]
        {
            self.spawn_auto_shards().await?.wait().await
        }

        #[cfg(not(feature = "sharding"))]
        {
            Err("sharding feature is required to auto-start shards".into())
        }
    }

    #[cfg(feature = "sharding")]
    pub async fn spawn_shards(self, shard_count: u32) -> Result<ShardSupervisor, DiscordError> {
        let ClientBuilder {
            token,
            intents,
            handler,
            data,
            application_id,
            gateway_config,
            shard: _,
        } = self;
        let handler = handler.ok_or("event_handler is required")?;
        let application_id = application_id.unwrap_or(0);
        let total_shards = shard_count.max(1);
        let runtime = SharedRuntime::new(&token, application_id, data);
        spawn_shard_supervisor(SpawnShardSupervisorConfig {
            token,
            intents,
            handler,
            runtime,
            gateway_config,
            total_shards,
            boot_window_size: 1,
            initial_delay: None,
        })
        .await
    }

    #[cfg(feature = "sharding")]
    pub async fn spawn_auto_shards(self) -> Result<ShardSupervisor, DiscordError> {
        let ClientBuilder {
            token,
            intents,
            handler,
            data,
            application_id,
            gateway_config,
            shard: _,
        } = self;
        let handler = handler.ok_or("event_handler is required")?;
        let application_id = application_id.unwrap_or(0);
        let metadata_http = DiscordHttpClient::new(&token, application_id);
        let gateway_bot = metadata_http.get_gateway_bot().await?;
        let auto_shard_plan = auto_shard_plan(&gateway_bot);
        let runtime = SharedRuntime::new(&token, application_id, data);
        let gateway_config = gateway_config.with_base_url(gateway_bot.url);

        spawn_shard_supervisor(SpawnShardSupervisorConfig {
            token,
            intents,
            handler,
            runtime,
            gateway_config,
            total_shards: auto_shard_plan.total_shards,
            boot_window_size: auto_shard_plan.boot_window_size,
            initial_delay: auto_shard_plan.initial_delay,
        })
        .await
    }
}

pub struct Client;

impl Client {
    pub fn builder(
        token: impl Into<String>,
        intents: impl Into<crate::bitfield::Intents>,
    ) -> ClientBuilder {
        ClientBuilder {
            token: token.into(),
            intents: intents.into().bits(),
            handler: None,
            data: TypeMap::new(),
            application_id: None,
            gateway_config: GatewayConnectionConfig::default(),
            shard: None,
        }
    }

    pub fn rest(token: impl Into<String>, application_id: u64) -> DiscordHttpClient {
        DiscordHttpClient::new(token, application_id)
    }
}

pub type BotClient = Client;
pub type BotClientBuilder = ClientBuilder;

#[cfg(feature = "sharding")]
const SHARD_BOOT_DELAY: Duration = Duration::from_millis(5_000);

#[derive(Clone)]
struct SharedRuntime {
    http: Arc<DiscordHttpClient>,
    data: Arc<RwLock<TypeMap>>,
    cache: CacheHandle,
    gateway_commands: Arc<RwLock<HashMap<u32, ShardMessenger>>>,
    #[cfg(feature = "voice")]
    voice: Arc<RwLock<VoiceManager>>,
    #[cfg(feature = "collectors")]
    collectors: CollectorHub,
}

impl SharedRuntime {
    fn new(token: &str, application_id: u64, data: TypeMap) -> Self {
        Self {
            http: Arc::new(DiscordHttpClient::new(token, application_id)),
            data: Arc::new(RwLock::new(data)),
            cache: CacheHandle::new(),
            gateway_commands: Arc::new(RwLock::new(HashMap::new())),
            #[cfg(feature = "voice")]
            voice: Arc::new(RwLock::new(VoiceManager::new())),
            #[cfg(feature = "collectors")]
            collectors: CollectorHub::new(),
        }
    }

    fn context(&self, shard: (u32, u32)) -> Context {
        let mut context = Context::new(Arc::clone(&self.http), Arc::clone(&self.data));
        context.cache = self.cache.clone();
        context.shard_id = shard.0;
        context.shard_count = shard.1;
        context.gateway_commands = Arc::clone(&self.gateway_commands);
        #[cfg(feature = "voice")]
        {
            context.voice = Arc::clone(&self.voice);
        }
        #[cfg(feature = "collectors")]
        {
            context.collectors = self.collectors.clone();
        }
        context
    }
}

#[cfg(feature = "sharding")]
struct ShardStartControl {
    supervisor_channels: Option<ShardRuntimeChannels>,
    boot_gate: Option<watch::Receiver<bool>>,
}

#[cfg(feature = "sharding")]
struct SpawnShardSupervisorConfig {
    token: String,
    intents: u64,
    handler: Arc<dyn EventHandler>,
    runtime: SharedRuntime,
    gateway_config: GatewayConnectionConfig,
    total_shards: u32,
    boot_window_size: u32,
    initial_delay: Option<Duration>,
}

#[cfg(feature = "sharding")]
struct AutoShardPlan {
    total_shards: u32,
    boot_window_size: u32,
    initial_delay: Option<Duration>,
}

#[cfg(feature = "sharding")]
fn auto_shard_plan(gateway_bot: &crate::model::GatewayBot) -> AutoShardPlan {
    AutoShardPlan {
        total_shards: gateway_bot.shards.max(1),
        boot_window_size: gateway_bot.session_start_limit.max_concurrency.max(1),
        initial_delay: if gateway_bot.session_start_limit.remaining == 0
            && gateway_bot.session_start_limit.reset_after > 0
        {
            Some(Duration::from_millis(
                gateway_bot.session_start_limit.reset_after,
            ))
        } else {
            None
        },
    }
}

#[cfg(feature = "sharding")]
async fn spawn_shard_supervisor(
    config: SpawnShardSupervisorConfig,
) -> Result<ShardSupervisor, DiscordError> {
    let SpawnShardSupervisorConfig {
        token,
        intents,
        handler,
        runtime,
        gateway_config,
        total_shards,
        boot_window_size,
        initial_delay,
    } = config;

    if let Some(initial_delay) = initial_delay {
        sleep(initial_delay).await;
    }

    let manager = Arc::new(StdMutex::new(ShardingManager::new(
        crate::sharding::ShardConfig::new(total_shards).gateway(gateway_config.clone()),
    )));
    let mut tasks = Vec::new();
    let mut queued_shards = Vec::new();

    for shard_id in 0..total_shards {
        let handler = Arc::clone(&handler);
        let runtime = runtime.clone();
        let token = token.clone();
        let gateway_config = gateway_config.clone().shard(shard_id, total_shards);
        let supervisor_channels = manager
            .lock()
            .expect("shard manager mutex poisoned")
            .prepare_runtime(shard_id)?;
        let (boot_tx, boot_rx) = watch::channel(false);

        tasks.push((
            shard_id,
            tokio::spawn(async move {
                start_gateway_shard(
                    token,
                    intents,
                    handler,
                    runtime,
                    gateway_config,
                    (shard_id, total_shards),
                    ShardStartControl {
                        supervisor_channels: Some(supervisor_channels),
                        boot_gate: Some(boot_rx),
                    },
                )
                .await
            }),
        ));
        queued_shards.push((shard_id, boot_tx));
    }

    let wave_size = boot_window_size.max(1) as usize;
    for (wave_index, wave) in queued_shards.chunks(wave_size).enumerate() {
        for (_, boot_tx) in wave {
            let _ = boot_tx.send(true);
        }
        if wave_index + 1 < queued_shards.len().div_ceil(wave_size) {
            sleep(SHARD_BOOT_DELAY).await;
        }
    }

    Ok(ShardSupervisor { manager, tasks })
}

async fn start_gateway_shard(
    token: String,
    intents: u64,
    handler: Arc<dyn EventHandler>,
    runtime: SharedRuntime,
    gateway_config: GatewayConnectionConfig,
    shard: (u32, u32),
    #[cfg(feature = "sharding")] shard_control: ShardStartControl,
) -> Result<(), DiscordError> {
    #[cfg(feature = "sharding")]
    if let Some(mut boot_gate) = shard_control.boot_gate {
        if let Some(supervisor_channels) = shard_control.supervisor_channels.as_ref() {
            let _ = supervisor_channels.publish(ShardSupervisorEvent::StateChanged {
                shard_id: shard.0,
                state: ShardRuntimeState::Queued,
            });
        }

        while !*boot_gate.borrow() {
            if boot_gate.changed().await.is_err() {
                if let Some(supervisor_channels) = shard_control.supervisor_channels.as_ref() {
                    let _ = supervisor_channels.publish(ShardSupervisorEvent::StateChanged {
                        shard_id: shard.0,
                        state: ShardRuntimeState::Stopped,
                    });
                }
                return Ok(());
            }
        }
    }

    let ctx = runtime.context(shard);
    let http_for_app_id = Arc::clone(&runtime.http);
    let cache_for_events = runtime.cache.clone();
    let gateway_commands_for_runtime = Arc::clone(&runtime.gateway_commands);
    #[cfg(feature = "voice")]
    let voice_for_events = Arc::clone(&runtime.voice);
    #[cfg(feature = "collectors")]
    let collectors_for_events = runtime.collectors.clone();
    let (gateway_command_tx, gateway_command_rx) = mpsc::unbounded_channel();

    runtime.gateway_commands.write().await.insert(
        shard.0,
        ShardMessenger {
            shard_id: shard.0,
            command_tx: gateway_command_tx.clone(),
        },
    );

    let callback: EventCallback = Arc::new(move |event_name: String, data: Value| {
        let handler = Arc::clone(&handler);
        let ctx = ctx.clone();
        let http_ref = Arc::clone(&http_for_app_id);
        let cache = cache_for_events.clone();
        #[cfg(feature = "voice")]
        let voice = Arc::clone(&voice_for_events);
        #[cfg(feature = "collectors")]
        let collectors = collectors_for_events.clone();

        tokio::spawn(async move {
            if event_name == "READY" && http_ref.application_id() == 0 {
                if let Some(app_id) = data
                    .pointer("/application/id")
                    .and_then(|value| value.as_str())
                    .and_then(|value| value.parse::<u64>().ok())
                {
                    http_ref.set_application_id(app_id);
                    info!("Set application_id from READY: {app_id}");
                }
            }

            let event = match decode_event(&event_name, data.clone()) {
                Ok(event) => event,
                Err(error) => {
                    warn!("Failed to decode event {event_name}: {error}");
                    Event::Unknown {
                        kind: event_name,
                        raw: data,
                    }
                }
            };

            apply_cache_updates(&cache, &event).await;
            #[cfg(feature = "voice")]
            apply_voice_updates(&voice, &event).await;
            #[cfg(feature = "collectors")]
            collectors.publish(event.clone());
            handler.handle_event(ctx, event).await;
        });
    });

    let mut gateway = GatewayClient::new(token, intents)
        .gateway_config(gateway_config)
        .control(gateway_command_rx);
    if shard.1 > 1 {
        gateway = gateway.shard(shard.0, shard.1);
    }
    #[cfg(feature = "sharding")]
    if let Some(supervisor_channels) = shard_control.supervisor_channels {
        let (command_rx, publisher) = supervisor_channels.split();
        forward_shard_commands(command_rx, gateway_command_tx);
        gateway = gateway.supervisor(shard_supervisor_callback(publisher));
    }
    let result = gateway.run(callback).await;
    gateway_commands_for_runtime.write().await.remove(&shard.0);
    result
}

async fn apply_cache_updates(cache: &CacheHandle, event: &Event) {
    match event {
        Event::Ready(_) => {
            cache.clear().await;
        }
        Event::GuildCreate(event) | Event::GuildUpdate(event) => {
            cache.upsert_guild(event.guild.clone()).await;
            for role in &event.guild.roles {
                cache
                    .upsert_role(event.guild.id.clone(), role.clone())
                    .await;
            }
        }
        Event::GuildDelete(event) => {
            cache.remove_guild(&event.data.id).await;
        }
        Event::ChannelCreate(event) | Event::ChannelUpdate(event) => {
            cache.upsert_channel(event.channel.clone()).await;
        }
        Event::ChannelDelete(event) => {
            cache.remove_channel(&event.channel.id).await;
        }
        Event::MemberAdd(event) | Event::MemberUpdate(event) => {
            if let Some(user) = event.member.user.as_ref() {
                cache
                    .upsert_member(
                        event.guild_id.clone(),
                        user.id.clone(),
                        event.member.clone(),
                    )
                    .await;
            }
        }
        Event::MemberRemove(event) => {
            cache
                .remove_member(&event.data.guild_id, &event.data.user.id)
                .await;
        }
        Event::RoleCreate(event) | Event::RoleUpdate(event) => {
            cache
                .upsert_role(event.guild_id.clone(), event.role.clone())
                .await;
        }
        Event::RoleDelete(event) => {
            cache
                .remove_role(&event.data.guild_id, &event.data.role_id)
                .await;
        }
        Event::MessageCreate(event) => {
            cache.upsert_message(event.message.clone()).await;
        }
        Event::MessageUpdate(event) => {
            if let Some(cached_message) = cache
                .message(&event.message.channel_id, &event.message.id)
                .await
            {
                cache
                    .upsert_message(merge_message_update(
                        cached_message,
                        event.message.clone(),
                        &event.raw,
                    ))
                    .await;
            }
        }
        Event::MessageDelete(event) => {
            cache
                .remove_message(&event.data.channel_id, &event.data.id)
                .await;
        }
        Event::MessageDeleteBulk(event) => {
            cache
                .remove_messages_bulk(&event.channel_id, &event.ids)
                .await;
        }
        Event::InteractionCreate(event) => {
            if let Interaction::Component(component) = &event.interaction {
                if let Some(channel_id) = component.context.channel_id.clone() {
                    cache
                        .upsert_channel(crate::model::Channel {
                            id: channel_id,
                            guild_id: component.context.guild_id.clone(),
                            kind: 0,
                            ..crate::model::Channel::default()
                        })
                        .await;
                }
            }
        }
        Event::VoiceStateUpdate(_) | Event::VoiceServerUpdate(_) => {}
        Event::ChannelPinsUpdate(_) => {}
        Event::GuildBanAdd(_) | Event::GuildBanRemove(_) => {}
        Event::GuildEmojisUpdate(_) => {}
        Event::GuildIntegrationsUpdate(_) => {}
        Event::WebhooksUpdate(_) => {}
        Event::InviteCreate(_) | Event::InviteDelete(_) => {}
        Event::MessageReactionAdd(_) | Event::MessageReactionRemove(_) => {}
        Event::MessageReactionRemoveAll(_) => {}
        Event::TypingStart(_) => {}
        Event::PresenceUpdate(_) => {}
        Event::Unknown { .. } => {}
    }
}

fn merge_message_update(
    mut cached: crate::model::Message,
    partial: crate::model::Message,
    raw: &Value,
) -> crate::model::Message {
    cached.id = partial.id.clone();
    cached.channel_id = partial.channel_id.clone();

    if raw.get("guild_id").is_some() {
        cached.guild_id = partial.guild_id;
    }
    if raw.get("author").is_some() {
        cached.author = partial.author;
    }
    if raw.get("member").is_some() {
        cached.member = partial.member;
    }
    if raw.get("content").is_some() {
        cached.content = partial.content;
    }
    if raw.get("timestamp").is_some() {
        cached.timestamp = partial.timestamp;
    }
    if raw.get("edited_timestamp").is_some() {
        cached.edited_timestamp = partial.edited_timestamp;
    }
    if raw.get("mentions").is_some() {
        cached.mentions = partial.mentions;
    }
    if raw.get("attachments").is_some() {
        cached.attachments = partial.attachments;
    }
    if raw.get("type").is_some() {
        cached.kind = partial.kind;
    }
    if raw.get("pinned").is_some() {
        cached.pinned = partial.pinned;
    }
    if raw.get("tts").is_some() {
        cached.tts = partial.tts;
    }
    if raw.get("flags").is_some() {
        cached.flags = partial.flags;
    }
    if raw.get("webhook_id").is_some() {
        cached.webhook_id = partial.webhook_id;
    }
    if raw.get("embeds").is_some() {
        cached.embeds = partial.embeds;
    }
    if raw.get("reactions").is_some() {
        cached.reactions = partial.reactions;
    }

    cached
}

#[cfg(feature = "voice")]
async fn apply_voice_updates(voice: &Arc<RwLock<VoiceManager>>, event: &Event) {
    let mut voice = voice.write().await;
    match event {
        Event::VoiceStateUpdate(event) => {
            let _ = voice.update_voice_state(&event.state);
        }
        Event::VoiceServerUpdate(event) => {
            let _ = voice.update_server(event.data.clone());
        }
        _ => {}
    }
}

#[cfg(feature = "sharding")]
fn shard_supervisor_callback(
    supervisor_channels: crate::sharding::ShardRuntimePublisher,
) -> SupervisorCallback {
    Arc::new(move |event| {
        let _ = supervisor_channels.publish(event);
    })
}

#[cfg(feature = "sharding")]
fn forward_shard_commands(
    command_rx: std::sync::mpsc::Receiver<ShardIpcMessage>,
    gateway_command_tx: mpsc::UnboundedSender<GatewayCommand>,
) {
    tokio::task::spawn_blocking(move || {
        while let Ok(command) = command_rx.recv() {
            let gateway_command = match command {
                ShardIpcMessage::Shutdown => GatewayCommand::Shutdown,
                ShardIpcMessage::Reconnect => GatewayCommand::Reconnect,
                ShardIpcMessage::UpdatePresence(status) => GatewayCommand::UpdatePresence(status),
                ShardIpcMessage::SendPayload(payload) => GatewayCommand::SendPayload(payload),
            };

            if gateway_command_tx.send(gateway_command).is_err() {
                break;
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    #[cfg(feature = "sharding")]
    use std::sync::Mutex as StdMutex;

    use async_trait::async_trait;
    use serde_json::{json, Value};
    use tokio::sync::{mpsc, Mutex, RwLock};
    #[cfg(feature = "sharding")]
    use tokio::task::JoinHandle;
    #[cfg(feature = "sharding")]
    use tokio::time::{sleep, Duration};

    #[cfg(feature = "sharding")]
    use super::{auto_shard_plan, ShardSupervisor};
    use super::{EventHandler, ShardMessenger};
    use crate::event::{
        decode_event, BulkMessageDeleteEvent, Event, MessageEvent, ReadyEvent, ReadyPayload,
    };
    use crate::gateway::client::GatewayCommand;
    use crate::http::DiscordHttpClient;
    #[cfg(feature = "sharding")]
    use crate::model::{GatewayBot, SessionStartLimit};
    use crate::model::{Interaction, Message, Snowflake, User};
    #[cfg(feature = "sharding")]
    use crate::sharding::{ShardConfig, ShardingManager};

    #[test]
    fn typed_event_decoder_maps_message_create() {
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

        assert_eq!(event.kind(), "MESSAGE_CREATE");
    }

    #[cfg(feature = "cache")]
    #[tokio::test]
    async fn apply_cache_updates_clears_cache_on_ready() {
        let cache = crate::cache::CacheHandle::new();
        let guild_id = Snowflake::from("1");
        let channel_id = Snowflake::from("10");
        let message_id = Snowflake::from("11");

        cache
            .upsert_guild(crate::model::Guild {
                id: guild_id.clone(),
                name: "discordrs".to_string(),
                ..crate::model::Guild::default()
            })
            .await;
        cache
            .upsert_message(Message {
                id: message_id.clone(),
                channel_id: channel_id.clone(),
                content: "cached".to_string(),
                ..Message::default()
            })
            .await;

        super::apply_cache_updates(
            &cache,
            &Event::Ready(ReadyEvent {
                data: ReadyPayload {
                    user: User {
                        id: Snowflake::from("2"),
                        username: "bot".to_string(),
                        ..User::default()
                    },
                    session_id: "session".to_string(),
                    application: None,
                    resume_gateway_url: None,
                },
                raw: json!({ "session_id": "session" }),
            }),
        )
        .await;

        assert!(cache.guild(&guild_id).await.is_none());
        assert!(cache.message(&channel_id, &message_id).await.is_none());
    }

    #[cfg(feature = "cache")]
    #[tokio::test]
    async fn apply_cache_updates_merges_partial_message_update_without_wiping_cached_fields() {
        let cache = crate::cache::CacheHandle::new();
        let channel_id = Snowflake::from("1");
        let message_id = Snowflake::from("2");

        cache
            .upsert_message(Message {
                id: message_id.clone(),
                channel_id: channel_id.clone(),
                content: "before".to_string(),
                author: Some(User {
                    id: Snowflake::from("3"),
                    username: "alice".to_string(),
                    ..User::default()
                }),
                attachments: vec![crate::model::Attachment {
                    id: Snowflake::from("4"),
                    filename: "file.txt".to_string(),
                    ..crate::model::Attachment::default()
                }],
                mentions: vec![User {
                    id: Snowflake::from("5"),
                    username: "bob".to_string(),
                    ..User::default()
                }],
                ..Message::default()
            })
            .await;

        super::apply_cache_updates(
            &cache,
            &Event::MessageUpdate(MessageEvent {
                message: Message {
                    id: message_id.clone(),
                    channel_id: channel_id.clone(),
                    content: "after".to_string(),
                    ..Message::default()
                },
                raw: json!({
                    "id": message_id.as_str(),
                    "channel_id": channel_id.as_str(),
                    "content": "after"
                }),
            }),
        )
        .await;

        let cached = cache.message(&channel_id, &message_id).await.unwrap();
        assert_eq!(cached.content, "after");
        assert_eq!(cached.author.unwrap().username, "alice");
        assert_eq!(cached.attachments.len(), 1);
        assert_eq!(cached.mentions.len(), 1);
    }

    #[cfg(feature = "cache")]
    #[tokio::test]
    async fn apply_cache_updates_ignores_partial_message_update_without_cached_message() {
        let cache = crate::cache::CacheHandle::new();
        let channel_id = Snowflake::from("1");
        let message_id = Snowflake::from("2");

        super::apply_cache_updates(
            &cache,
            &Event::MessageUpdate(MessageEvent {
                message: Message {
                    id: message_id.clone(),
                    channel_id: channel_id.clone(),
                    content: "after".to_string(),
                    ..Message::default()
                },
                raw: json!({
                    "id": message_id.as_str(),
                    "channel_id": channel_id.as_str(),
                    "content": "after"
                }),
            }),
        )
        .await;

        assert!(cache.message(&channel_id, &message_id).await.is_none());
    }

    #[cfg(feature = "cache")]
    #[tokio::test]
    async fn apply_cache_updates_evicts_bulk_deleted_messages() {
        let cache = crate::cache::CacheHandle::new();
        let channel_id = Snowflake::from("1");
        let first = Snowflake::from("2");
        let second = Snowflake::from("3");

        for message_id in [first.clone(), second.clone()] {
            cache
                .upsert_message(Message {
                    id: message_id,
                    channel_id: channel_id.clone(),
                    content: "hello".to_string(),
                    ..Message::default()
                })
                .await;
        }

        super::apply_cache_updates(
            &cache,
            &Event::MessageDeleteBulk(BulkMessageDeleteEvent {
                ids: vec![first.clone(), second.clone()],
                channel_id: channel_id.clone(),
                guild_id: None,
                raw: json!({}),
            }),
        )
        .await;

        assert!(cache.message(&channel_id, &first).await.is_none());
        assert!(cache.message(&channel_id, &second).await.is_none());
    }

    #[tokio::test]
    async fn event_handler_routes_typed_hooks_and_reserves_raw_for_unknowns() {
        struct RecordingHandler {
            hits: Arc<Mutex<Vec<String>>>,
        }

        #[async_trait]
        impl super::EventHandler for RecordingHandler {
            async fn ready(&self, _ctx: super::Context, ready_data: ReadyPayload) {
                self.hits
                    .lock()
                    .await
                    .push(format!("ready:{}", ready_data.user.username));
            }

            async fn message_create(&self, _ctx: super::Context, message: Message) {
                self.hits
                    .lock()
                    .await
                    .push(format!("message:{}", message.content));
            }

            async fn interaction_create(&self, _ctx: super::Context, interaction: Interaction) {
                let label = match interaction {
                    Interaction::Ping(_) => "interaction:ping",
                    _ => "interaction:other",
                };
                self.hits.lock().await.push(label.to_string());
            }

            async fn message_delete_bulk(
                &self,
                _ctx: super::Context,
                event: BulkMessageDeleteEvent,
            ) {
                self.hits
                    .lock()
                    .await
                    .push(format!("bulk:{}", event.ids.len()));
            }

            async fn raw_event(&self, _ctx: super::Context, event_name: String, _data: Value) {
                self.hits.lock().await.push(format!("raw:{event_name}"));
            }
        }

        let hits = Arc::new(Mutex::new(Vec::new()));
        let handler = RecordingHandler {
            hits: Arc::clone(&hits),
        };
        let context = super::Context::new(
            Arc::new(DiscordHttpClient::new("token", 0)),
            Arc::new(RwLock::new(super::TypeMap::new())),
        );

        handler
            .handle_event(
                context.clone(),
                Event::Ready(ReadyEvent {
                    data: ReadyPayload {
                        user: User {
                            id: Snowflake::from("1"),
                            username: "bot".to_string(),
                            ..User::default()
                        },
                        session_id: "session".to_string(),
                        application: None,
                        resume_gateway_url: None,
                    },
                    raw: json!({ "session_id": "session" }),
                }),
            )
            .await;
        handler
            .handle_event(
                context.clone(),
                Event::MessageCreate(MessageEvent {
                    message: Message {
                        id: Snowflake::from("2"),
                        channel_id: Snowflake::from("3"),
                        content: "hello".to_string(),
                        ..Message::default()
                    },
                    raw: json!({}),
                }),
            )
            .await;
        handler
            .handle_event(
                context.clone(),
                decode_event(
                    "INTERACTION_CREATE",
                    json!({
                        "id": "4",
                        "application_id": "5",
                        "token": "interaction-token",
                        "type": 1
                    }),
                )
                .unwrap(),
            )
            .await;
        handler
            .handle_event(
                context.clone(),
                Event::MessageDeleteBulk(BulkMessageDeleteEvent {
                    ids: vec![Snowflake::from("6"), Snowflake::from("7")],
                    channel_id: Snowflake::from("3"),
                    guild_id: None,
                    raw: json!({}),
                }),
            )
            .await;
        handler
            .handle_event(
                context,
                Event::Unknown {
                    kind: "SOMETHING_NEW".to_string(),
                    raw: json!({ "surprise": true }),
                },
            )
            .await;

        assert_eq!(
            *hits.lock().await,
            vec![
                "ready:bot".to_string(),
                "message:hello".to_string(),
                "interaction:ping".to_string(),
                "bulk:2".to_string(),
                "raw:SOMETHING_NEW".to_string(),
            ]
        );
    }

    #[test]
    fn shard_messenger_builds_voice_state_update_gateway_payload() {
        let (command_tx, mut command_rx) = mpsc::unbounded_channel();
        let messenger = ShardMessenger {
            shard_id: 0,
            command_tx,
        };

        messenger.join_voice("1", "2", false, true).unwrap();

        match command_rx.try_recv().unwrap() {
            GatewayCommand::SendPayload(payload) => {
                assert_eq!(payload["op"], serde_json::json!(4));
                assert_eq!(payload["d"]["guild_id"], serde_json::json!("1"));
                assert_eq!(payload["d"]["channel_id"], serde_json::json!("2"));
                assert_eq!(payload["d"]["self_mute"], serde_json::json!(false));
                assert_eq!(payload["d"]["self_deaf"], serde_json::json!(true));
            }
            other => panic!("unexpected gateway command: {other:?}"),
        }
    }

    #[tokio::test]
    async fn context_new_keeps_legacy_http_and_data_entry_points() {
        let http = Arc::new(crate::http::DiscordHttpClient::new("token", 0));
        let data = Arc::new(RwLock::new(super::TypeMap::new()));
        let context = super::Context::new(Arc::clone(&http), Arc::clone(&data));

        assert!(Arc::ptr_eq(&context.http, &http));
        assert!(Arc::ptr_eq(&context.data, &data));
        assert_eq!(context.shard_pair(), (0, 1));
    }

    #[cfg(feature = "sharding")]
    #[tokio::test]
    async fn shard_supervisor_wait_for_shutdown_times_out() {
        let supervisor = ShardSupervisor {
            manager: Arc::new(StdMutex::new(ShardingManager::new(ShardConfig::new(1)))),
            tasks: vec![(
                0,
                tokio::spawn(async move {
                    sleep(Duration::from_millis(50)).await;
                    Ok(())
                }) as JoinHandle<Result<(), crate::error::DiscordError>>,
            )],
        };

        let error = supervisor
            .wait_for_shutdown(Duration::from_millis(1))
            .await
            .expect_err("shutdown wait should time out for a hanging shard");

        assert!(error
            .to_string()
            .contains("timed out waiting for shard 0 shutdown"));
    }

    #[cfg(feature = "sharding")]
    #[test]
    fn auto_shard_plan_uses_gateway_bot_recommendations() {
        let plan = auto_shard_plan(&GatewayBot {
            url: "wss://gateway.discord.gg".to_string(),
            shards: 8,
            session_start_limit: SessionStartLimit {
                total: 1000,
                remaining: 5,
                reset_after: 15_000,
                max_concurrency: 4,
            },
        });

        assert_eq!(plan.total_shards, 8);
        assert_eq!(plan.boot_window_size, 4);
        assert_eq!(plan.initial_delay, None);
    }

    #[cfg(feature = "sharding")]
    #[test]
    fn auto_shard_plan_waits_for_reset_when_identify_budget_is_empty() {
        let plan = auto_shard_plan(&GatewayBot {
            url: "wss://gateway.discord.gg".to_string(),
            shards: 2,
            session_start_limit: SessionStartLimit {
                total: 1000,
                remaining: 0,
                reset_after: 1_500,
                max_concurrency: 0,
            },
        });

        assert_eq!(plan.total_shards, 2);
        assert_eq!(plan.boot_window_size, 1);
        assert_eq!(plan.initial_delay, Some(Duration::from_millis(1_500)));
    }
}
