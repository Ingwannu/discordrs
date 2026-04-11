use std::collections::HashMap;
use std::sync::mpsc::{channel, Receiver, Sender, TryRecvError};
use std::sync::{Arc, Mutex};

use serde_json::Value;

use crate::error::DiscordError;
use crate::model::Snowflake;
use crate::types::invalid_data_error;
use crate::ws::GatewayConnectionConfig;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ShardInfo {
    pub id: u32,
    pub total: u32,
}

impl ShardInfo {
    pub fn identify_payload(&self) -> [u32; 2] {
        [self.id, self.total]
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ShardConfig {
    pub total_shards: u32,
    pub gateway: GatewayConnectionConfig,
}

impl ShardConfig {
    pub fn new(total_shards: u32) -> Self {
        Self {
            total_shards: total_shards.max(1),
            gateway: GatewayConnectionConfig::default(),
        }
    }

    pub fn gateway(mut self, gateway: GatewayConnectionConfig) -> Self {
        self.gateway = gateway;
        self
    }

    pub fn shard_info(&self, shard_id: u32) -> Option<ShardInfo> {
        (shard_id < self.total_shards).then_some(ShardInfo {
            id: shard_id,
            total: self.total_shards,
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ShardIpcMessage {
    Shutdown,
    Reconnect,
    UpdatePresence(String),
    SendPayload(Value),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ShardRuntimeState {
    Idle,
    Queued,
    Starting,
    Running,
    Reconnecting,
    Stopped,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ShardSupervisorEvent {
    StateChanged {
        shard_id: u32,
        state: ShardRuntimeState,
    },
    SessionEstablished {
        shard_id: u32,
        session_id: String,
    },
    GatewayError {
        shard_id: u32,
        message: String,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ShardRuntimeStatus {
    pub info: ShardInfo,
    pub state: ShardRuntimeState,
    pub session_id: Option<String>,
    pub last_error: Option<String>,
}

impl ShardRuntimeStatus {
    fn new(info: ShardInfo) -> Self {
        Self {
            info,
            state: ShardRuntimeState::Idle,
            session_id: None,
            last_error: None,
        }
    }
}

#[derive(Clone)]
pub struct ShardRuntimeHandle {
    command_tx: Sender<ShardIpcMessage>,
    event_rx: Arc<Mutex<Receiver<ShardSupervisorEvent>>>,
    status: Arc<Mutex<ShardRuntimeStatus>>,
}

impl ShardRuntimeHandle {
    pub fn info(&self) -> ShardInfo {
        self.status
            .lock()
            .expect("shard status mutex poisoned")
            .info
            .clone()
    }

    pub fn status(&self) -> ShardRuntimeStatus {
        self.status
            .lock()
            .expect("shard status mutex poisoned")
            .clone()
    }

    pub fn state(&self) -> ShardRuntimeState {
        self.status().state
    }

    pub fn session_id(&self) -> Option<String> {
        self.status().session_id
    }

    pub fn last_error(&self) -> Option<String> {
        self.status().last_error
    }

    pub fn command_sender(&self) -> Sender<ShardIpcMessage> {
        self.command_tx.clone()
    }

    pub fn send(&self, message: ShardIpcMessage) -> Result<(), DiscordError> {
        self.command_tx.send(message).map_err(|error| {
            invalid_data_error(format!("failed to send shard ipc message: {error}"))
        })
    }

    pub fn try_recv_event(&self) -> Result<Option<ShardSupervisorEvent>, DiscordError> {
        match self
            .event_rx
            .lock()
            .expect("shard event receiver mutex poisoned")
            .try_recv()
        {
            Ok(event) => {
                self.apply_event(&event);
                Ok(Some(event))
            }
            Err(TryRecvError::Empty) => Ok(None),
            Err(TryRecvError::Disconnected) => Err(invalid_data_error(format!(
                "shard {} event channel disconnected",
                self.info().id
            ))),
        }
    }

    fn apply_event(&self, event: &ShardSupervisorEvent) {
        let mut status = self.status.lock().expect("shard status mutex poisoned");

        match event {
            ShardSupervisorEvent::StateChanged { state, .. } => status.state = state.clone(),
            ShardSupervisorEvent::SessionEstablished { session_id, .. } => {
                status.session_id = Some(session_id.clone());
                status.last_error = None;
            }
            ShardSupervisorEvent::GatewayError { message, .. } => {
                status.last_error = Some(message.clone());
            }
        }
    }
}

pub struct ShardRuntimeChannels {
    pub info: ShardInfo,
    pub command_rx: Receiver<ShardIpcMessage>,
    pub event_tx: Sender<ShardSupervisorEvent>,
    status: Arc<Mutex<ShardRuntimeStatus>>,
}

#[derive(Clone)]
pub struct ShardRuntimePublisher {
    pub info: ShardInfo,
    pub event_tx: Sender<ShardSupervisorEvent>,
    status: Arc<Mutex<ShardRuntimeStatus>>,
}

impl ShardRuntimePublisher {
    pub fn publish(&self, event: ShardSupervisorEvent) -> Result<(), DiscordError> {
        self.apply_event(&event);
        self.event_tx.send(event).map_err(|error| {
            invalid_data_error(format!(
                "failed to publish shard supervisor event for shard {}: {error}",
                self.info.id
            ))
        })
    }

    fn apply_event(&self, event: &ShardSupervisorEvent) {
        let mut status = self.status.lock().expect("shard status mutex poisoned");

        match event {
            ShardSupervisorEvent::StateChanged { state, .. } => status.state = state.clone(),
            ShardSupervisorEvent::SessionEstablished { session_id, .. } => {
                status.session_id = Some(session_id.clone());
                status.last_error = None;
            }
            ShardSupervisorEvent::GatewayError { message, .. } => {
                status.last_error = Some(message.clone());
            }
        }
    }
}

impl ShardRuntimeChannels {
    pub fn status(&self) -> ShardRuntimeStatus {
        self.status
            .lock()
            .expect("shard status mutex poisoned")
            .clone()
    }

    pub fn state(&self) -> ShardRuntimeState {
        self.status().state
    }

    pub fn set_state(&self, state: ShardRuntimeState) {
        self.status
            .lock()
            .expect("shard status mutex poisoned")
            .state = state;
    }

    pub fn publish(&self, event: ShardSupervisorEvent) -> Result<(), DiscordError> {
        self.publisher().publish(event)
    }

    pub fn publisher(&self) -> ShardRuntimePublisher {
        ShardRuntimePublisher {
            info: self.info.clone(),
            event_tx: self.event_tx.clone(),
            status: Arc::clone(&self.status),
        }
    }

    pub fn split(self) -> (Receiver<ShardIpcMessage>, ShardRuntimePublisher) {
        let publisher = ShardRuntimePublisher {
            info: self.info,
            event_tx: self.event_tx,
            status: self.status,
        };
        (self.command_rx, publisher)
    }
}

pub struct ShardingManager {
    config: ShardConfig,
    runtimes: HashMap<u32, ShardRuntimeHandle>,
}

impl ShardingManager {
    pub fn new(config: ShardConfig) -> Self {
        Self {
            config,
            runtimes: HashMap::new(),
        }
    }

    pub fn shard_infos(&self) -> Vec<ShardInfo> {
        (0..self.config.total_shards)
            .map(|id| ShardInfo {
                id,
                total: self.config.total_shards,
            })
            .collect()
    }

    pub fn shard_for_guild(&self, guild_id: &Snowflake) -> Option<ShardInfo> {
        let raw_id = guild_id.as_u64()?;
        let shard_id = ((raw_id >> 22) % u64::from(self.config.total_shards)) as u32;
        self.config.shard_info(shard_id)
    }

    pub fn gateway_config(&self, shard_id: u32) -> Option<GatewayConnectionConfig> {
        self.config
            .shard_info(shard_id)
            .map(|info| self.config.gateway.clone().shard(info.id, info.total))
    }

    pub fn attach_runtime(
        &mut self,
        shard_id: u32,
        command_tx: Sender<ShardIpcMessage>,
        event_rx: Receiver<ShardSupervisorEvent>,
    ) -> Result<ShardRuntimeHandle, DiscordError> {
        let info = self.shard_info_or_error(shard_id)?;
        let handle = ShardRuntimeHandle {
            command_tx,
            event_rx: Arc::new(Mutex::new(event_rx)),
            status: Arc::new(Mutex::new(ShardRuntimeStatus::new(info))),
        };

        self.runtimes.insert(shard_id, handle.clone());
        Ok(handle)
    }

    pub fn prepare_runtime(&mut self, shard_id: u32) -> Result<ShardRuntimeChannels, DiscordError> {
        let (command_tx, command_rx) = channel();
        let (event_tx, event_rx) = channel();
        let handle = self.attach_runtime(shard_id, command_tx, event_rx)?;

        Ok(ShardRuntimeChannels {
            info: handle.info(),
            command_rx,
            event_tx,
            status: Arc::clone(&handle.status),
        })
    }

    pub fn handle(&self, shard_id: u32) -> Option<ShardRuntimeHandle> {
        self.runtimes.get(&shard_id).cloned()
    }

    pub fn state(&self, shard_id: u32) -> Option<ShardRuntimeState> {
        self.runtimes.get(&shard_id).map(ShardRuntimeHandle::state)
    }

    pub fn status(&self, shard_id: u32) -> Option<ShardRuntimeStatus> {
        self.runtimes.get(&shard_id).map(ShardRuntimeHandle::status)
    }

    pub fn states(&self) -> Vec<(ShardInfo, ShardRuntimeState)> {
        self.runtimes
            .values()
            .map(|handle| (handle.info(), handle.state()))
            .collect()
    }

    pub fn statuses(&self) -> Vec<ShardRuntimeStatus> {
        self.runtimes
            .values()
            .map(ShardRuntimeHandle::status)
            .collect()
    }

    pub fn runtime_count(&self) -> usize {
        self.runtimes.len()
    }

    pub fn remove_runtime(&mut self, shard_id: u32) -> Option<ShardRuntimeHandle> {
        self.runtimes.remove(&shard_id)
    }

    pub fn poll_event(&self, shard_id: u32) -> Result<Option<ShardSupervisorEvent>, DiscordError> {
        let Some(handle) = self.runtimes.get(&shard_id) else {
            return Err(invalid_data_error(format!(
                "missing shard runtime for shard {shard_id}"
            )));
        };
        handle.try_recv_event()
    }

    pub fn drain_events(&self) -> Result<Vec<ShardSupervisorEvent>, DiscordError> {
        let mut events = Vec::new();
        for handle in self.runtimes.values() {
            while let Some(event) = handle.try_recv_event()? {
                events.push(event);
            }
        }
        Ok(events)
    }

    pub fn register_ipc(&mut self, shard_id: u32, sender: Sender<ShardIpcMessage>) {
        if let Some(handle) = self.runtimes.get_mut(&shard_id) {
            handle.command_tx = sender;
            return;
        }

        if let Some(info) = self.config.shard_info(shard_id) {
            let (_, event_rx) = channel();
            self.runtimes.insert(
                shard_id,
                ShardRuntimeHandle {
                    command_tx: sender,
                    event_rx: Arc::new(Mutex::new(event_rx)),
                    status: Arc::new(Mutex::new(ShardRuntimeStatus::new(info))),
                },
            );
        }
    }

    pub fn send(&self, shard_id: u32, message: ShardIpcMessage) -> Result<(), DiscordError> {
        let Some(handle) = self.runtimes.get(&shard_id) else {
            return Err(invalid_data_error(format!(
                "missing shard ipc channel for shard {shard_id}"
            )));
        };

        handle.send(message)
    }

    pub fn broadcast(&self, message: ShardIpcMessage) -> Result<(), DiscordError> {
        for handle in self.runtimes.values() {
            handle.send(message.clone())?;
        }

        Ok(())
    }

    fn shard_info_or_error(&self, shard_id: u32) -> Result<ShardInfo, DiscordError> {
        self.config
            .shard_info(shard_id)
            .ok_or_else(|| invalid_data_error(format!("invalid shard id {shard_id}")))
    }
}

#[cfg(test)]
mod tests {
    use std::sync::mpsc::channel;

    use super::{
        ShardConfig, ShardIpcMessage, ShardRuntimeState, ShardSupervisorEvent, ShardingManager,
    };
    use crate::model::Snowflake;

    #[test]
    fn shard_for_guild_uses_discord_formula() {
        let manager = ShardingManager::new(ShardConfig::new(4));
        let shard = manager.shard_for_guild(&Snowflake::from("81384788765712384"));

        assert_eq!(shard.unwrap().total, 4);
    }

    #[test]
    fn gateway_config_applies_shard_pair() {
        let manager = ShardingManager::new(ShardConfig::new(2));
        let config = manager.gateway_config(1).unwrap();

        assert_eq!(
            config.normalized_url(),
            "wss://gateway.discord.gg/?v=10&encoding=json&shard=1,2"
        );
    }

    #[test]
    fn prepare_runtime_exposes_command_and_event_channels() {
        let mut manager = ShardingManager::new(ShardConfig::new(2));
        let channels = manager.prepare_runtime(1).unwrap();

        manager.send(1, ShardIpcMessage::Reconnect).unwrap();
        assert_eq!(
            channels.command_rx.recv().unwrap(),
            ShardIpcMessage::Reconnect
        );

        channels
            .publish(ShardSupervisorEvent::StateChanged {
                shard_id: 1,
                state: ShardRuntimeState::Running,
            })
            .unwrap();
        channels
            .publish(ShardSupervisorEvent::SessionEstablished {
                shard_id: 1,
                session_id: "session-1".to_string(),
            })
            .unwrap();

        let event = manager.poll_event(1).unwrap().unwrap();
        assert_eq!(
            event,
            ShardSupervisorEvent::StateChanged {
                shard_id: 1,
                state: ShardRuntimeState::Running,
            }
        );
        assert_eq!(manager.state(1), Some(ShardRuntimeState::Running));
        assert_eq!(
            manager.status(1).unwrap().session_id.as_deref(),
            Some("session-1")
        );
    }

    #[test]
    fn drain_events_collects_across_shards() {
        let mut manager = ShardingManager::new(ShardConfig::new(2));
        let shard_zero = manager.prepare_runtime(0).unwrap();
        let shard_one = manager.prepare_runtime(1).unwrap();

        shard_zero
            .event_tx
            .send(ShardSupervisorEvent::SessionEstablished {
                shard_id: 0,
                session_id: "session-a".to_string(),
            })
            .unwrap();
        shard_one
            .event_tx
            .send(ShardSupervisorEvent::GatewayError {
                shard_id: 1,
                message: "boom".to_string(),
            })
            .unwrap();

        let events = manager.drain_events().unwrap();
        assert_eq!(events.len(), 2);
    }

    #[test]
    fn attach_runtime_supports_external_supervisor_channels() {
        let mut manager = ShardingManager::new(ShardConfig::new(1));
        let (command_tx, command_rx) = channel();
        let (event_tx, event_rx) = channel();

        let handle = manager.attach_runtime(0, command_tx, event_rx).unwrap();

        handle.send(ShardIpcMessage::Shutdown).unwrap();
        assert_eq!(command_rx.recv().unwrap(), ShardIpcMessage::Shutdown);

        event_tx
            .send(ShardSupervisorEvent::GatewayError {
                shard_id: 0,
                message: "gateway closed".to_string(),
            })
            .unwrap();

        assert_eq!(
            handle.try_recv_event().unwrap(),
            Some(ShardSupervisorEvent::GatewayError {
                shard_id: 0,
                message: "gateway closed".to_string(),
            })
        );
        assert_eq!(
            manager.status(0).unwrap().last_error.as_deref(),
            Some("gateway closed")
        );
        assert_eq!(manager.runtime_count(), 1);

        manager.remove_runtime(0).unwrap();
        assert_eq!(manager.runtime_count(), 0);
    }
}
