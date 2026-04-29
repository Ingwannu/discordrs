use std::collections::{HashMap, VecDeque};

use serde::{Deserialize, Serialize};

use crate::error::DiscordError;
use crate::model::{Snowflake, VoiceServerUpdate, VoiceState};
use crate::types::invalid_data_error;
#[cfg(feature = "voice")]
use crate::voice_runtime::VoiceRuntimeConfig;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum VoiceConnectionStatus {
    Connecting,
    Connected,
    Disconnected,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(transparent)]
pub struct VoiceGatewayOpcode(u8);

impl VoiceGatewayOpcode {
    pub const IDENTIFY: Self = Self(0);
    pub const SELECT_PROTOCOL: Self = Self(1);
    pub const READY: Self = Self(2);
    pub const HEARTBEAT: Self = Self(3);
    pub const SESSION_DESCRIPTION: Self = Self(4);
    pub const SPEAKING: Self = Self(5);
    pub const HEARTBEAT_ACK: Self = Self(6);
    pub const RESUME: Self = Self(7);
    pub const HELLO: Self = Self(8);
    pub const RESUMED: Self = Self(9);
    pub const DAVE_PROTOCOL_TRANSITION_READY: Self = Self(23);
    pub const DAVE_MLS_KEY_PACKAGE: Self = Self(26);
    pub const DAVE_MLS_COMMIT_WELCOME: Self = Self(28);
    pub const DAVE_MLS_INVALID_COMMIT_WELCOME: Self = Self(31);

    pub const fn code(self) -> u8 {
        self.0
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(transparent)]
pub struct VoiceTransportProtocol(String);

impl VoiceTransportProtocol {
    pub fn new(protocol: impl Into<String>) -> Self {
        Self(protocol.into())
    }

    pub fn udp() -> Self {
        Self::new("udp")
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(transparent)]
pub struct VoiceEncryptionMode(String);

impl VoiceEncryptionMode {
    pub fn new(mode: impl Into<String>) -> Self {
        Self(mode.into())
    }

    pub fn aead_aes256_gcm_rtpsize() -> Self {
        Self::new("aead_aes256_gcm_rtpsize")
    }

    pub fn aead_xchacha20_poly1305_rtpsize() -> Self {
        Self::new("aead_xchacha20_poly1305_rtpsize")
    }
}

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(transparent)]
pub struct VoiceSpeakingFlags(u8);

impl VoiceSpeakingFlags {
    pub const MICROPHONE: Self = Self(1);
    pub const SOUNDSHARE: Self = Self(1 << 1);
    pub const PRIORITY: Self = Self(1 << 2);

    pub const fn bits(self) -> u8 {
        self.0
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct VoiceConnectionConfig {
    pub guild_id: Snowflake,
    pub channel_id: Snowflake,
    pub endpoint: Option<String>,
    pub session_id: Option<String>,
    pub token: Option<String>,
    pub self_mute: bool,
    pub self_deaf: bool,
}

impl VoiceConnectionConfig {
    pub fn new(guild_id: impl Into<Snowflake>, channel_id: impl Into<Snowflake>) -> Self {
        Self {
            guild_id: guild_id.into(),
            channel_id: channel_id.into(),
            endpoint: None,
            session_id: None,
            token: None,
            self_mute: false,
            self_deaf: false,
        }
    }

    pub fn endpoint(mut self, endpoint: impl Into<String>) -> Self {
        self.endpoint = Some(endpoint.into());
        self
    }

    pub fn session_id(mut self, session_id: impl Into<String>) -> Self {
        self.session_id = Some(session_id.into());
        self
    }

    pub fn token(mut self, token: impl Into<String>) -> Self {
        self.token = Some(token.into());
        self
    }

    pub fn self_mute(mut self, self_mute: bool) -> Self {
        self.self_mute = self_mute;
        self
    }

    pub fn self_deaf(mut self, self_deaf: bool) -> Self {
        self.self_deaf = self_deaf;
        self
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct VoiceTransportState {
    pub protocol: VoiceTransportProtocol,
    pub ip: String,
    pub port: u16,
    pub mode: VoiceEncryptionMode,
    pub ssrc: u32,
}

impl VoiceTransportState {
    pub fn udp(ip: impl Into<String>, port: u16, mode: VoiceEncryptionMode, ssrc: u32) -> Self {
        Self {
            protocol: VoiceTransportProtocol::udp(),
            ip: ip.into(),
            port,
            mode,
            ssrc,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct VoiceSpeakingState {
    pub flags: VoiceSpeakingFlags,
    pub delay: u32,
}

impl VoiceSpeakingState {
    pub fn new(flags: VoiceSpeakingFlags, delay: u32) -> Self {
        Self { flags, delay }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct VoiceConnectionState {
    pub guild_id: Snowflake,
    pub channel_id: Snowflake,
    pub endpoint: Option<String>,
    pub session_id: Option<String>,
    pub token: Option<String>,
    pub self_mute: bool,
    pub self_deaf: bool,
    pub status: VoiceConnectionStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transport: Option<VoiceTransportState>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub speaking: Option<VoiceSpeakingState>,
}

impl VoiceConnectionState {
    pub fn from_config(config: VoiceConnectionConfig) -> Self {
        let mut state = Self {
            guild_id: config.guild_id,
            channel_id: config.channel_id,
            endpoint: config.endpoint,
            session_id: config.session_id,
            token: config.token,
            self_mute: config.self_mute,
            self_deaf: config.self_deaf,
            status: VoiceConnectionStatus::Connecting,
            transport: None,
            speaking: None,
        };

        if state.is_ready() {
            state.mark_connected();
        }

        state
    }

    pub fn is_ready(&self) -> bool {
        self.endpoint.is_some() && self.session_id.is_some() && self.token.is_some()
    }

    pub fn mark_connected(&mut self) {
        self.status = VoiceConnectionStatus::Connected;
    }

    pub fn mark_disconnected(&mut self) {
        self.status = VoiceConnectionStatus::Disconnected;
    }

    fn clear_runtime_state(&mut self) {
        self.endpoint = None;
        self.session_id = None;
        self.token = None;
        self.transport = None;
        self.speaking = None;
    }

    fn transition_to_disconnected(&mut self) {
        self.clear_runtime_state();
        self.mark_disconnected();
    }

    pub fn apply_server_update(&mut self, update: &VoiceServerUpdate) {
        self.transport = None;
        self.speaking = None;
        self.endpoint = update.endpoint.clone();
        self.token = Some(update.token.clone());
        if self.endpoint.is_none() {
            self.session_id = None;
            self.token = None;
            self.mark_disconnected();
            return;
        }
        if self.is_ready() {
            self.mark_connected();
        } else {
            self.status = VoiceConnectionStatus::Connecting;
        }
    }

    pub fn apply_voice_state(&mut self, state: &VoiceState) {
        if let Some(channel_id) = state.channel_id.clone() {
            self.channel_id = channel_id;
        } else {
            self.self_mute = state.self_mute;
            self.self_deaf = state.self_deaf;
            self.transition_to_disconnected();
            return;
        }
        self.session_id = state.session_id.clone();
        self.self_mute = state.self_mute;
        self.self_deaf = state.self_deaf;
        if self.is_ready() {
            self.mark_connected();
        } else {
            self.status = VoiceConnectionStatus::Connecting;
        }
    }

    pub fn set_transport(&mut self, transport: VoiceTransportState) {
        self.transport = Some(transport);
    }

    pub fn set_speaking(&mut self, speaking: VoiceSpeakingState) {
        self.speaking = Some(speaking);
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct VoiceGatewayHello {
    pub heartbeat_interval_ms: u64,
}

impl VoiceGatewayHello {
    pub fn new(heartbeat_interval_ms: u64) -> Self {
        Self {
            heartbeat_interval_ms,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct VoiceGatewayReady {
    pub ssrc: u32,
    pub ip: String,
    pub port: u16,
    #[serde(default)]
    pub modes: Vec<VoiceEncryptionMode>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub heartbeat_interval_ms: Option<u64>,
}

impl VoiceGatewayReady {
    pub fn new(ssrc: u32, ip: impl Into<String>, port: u16) -> Self {
        Self {
            ssrc,
            ip: ip.into(),
            port,
            modes: Vec::new(),
            heartbeat_interval_ms: None,
        }
    }

    pub fn mode(mut self, mode: VoiceEncryptionMode) -> Self {
        self.modes.push(mode);
        self
    }

    pub fn heartbeat_interval_ms(mut self, heartbeat_interval_ms: u64) -> Self {
        self.heartbeat_interval_ms = Some(heartbeat_interval_ms);
        self
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct VoiceSelectProtocolData {
    pub address: String,
    pub port: u16,
    pub mode: VoiceEncryptionMode,
}

impl VoiceSelectProtocolData {
    pub fn new(address: impl Into<String>, port: u16, mode: VoiceEncryptionMode) -> Self {
        Self {
            address: address.into(),
            port,
            mode,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct VoiceSelectProtocolCommand {
    pub protocol: VoiceTransportProtocol,
    pub data: VoiceSelectProtocolData,
}

impl VoiceSelectProtocolCommand {
    pub fn new(protocol: VoiceTransportProtocol, data: VoiceSelectProtocolData) -> Self {
        Self { protocol, data }
    }

    pub fn udp(address: impl Into<String>, port: u16, mode: VoiceEncryptionMode) -> Self {
        Self::new(
            VoiceTransportProtocol::udp(),
            VoiceSelectProtocolData::new(address, port, mode),
        )
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct VoiceSpeakingCommand {
    pub speaking: VoiceSpeakingFlags,
    pub delay: u32,
    pub ssrc: u32,
}

impl VoiceSpeakingCommand {
    pub fn new(ssrc: u32) -> Self {
        Self {
            speaking: VoiceSpeakingFlags::default(),
            delay: 0,
            ssrc,
        }
    }

    pub fn speaking(mut self, speaking: VoiceSpeakingFlags) -> Self {
        self.speaking = speaking;
        self
    }

    pub fn delay(mut self, delay: u32) -> Self {
        self.delay = delay;
        self
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum VoiceGatewayCommand {
    Identify {
        server_id: Snowflake,
        user_id: Snowflake,
        session_id: String,
        token: String,
    },
    SelectProtocol(VoiceSelectProtocolCommand),
    Speaking(VoiceSpeakingCommand),
    Resume {
        server_id: Snowflake,
        session_id: String,
        token: String,
    },
    Heartbeat {
        nonce: u64,
    },
    DaveProtocolTransitionReady {
        transition_id: u64,
    },
    DaveMlsKeyPackage {
        key_package: Vec<u8>,
    },
    DaveMlsCommitWelcome {
        commit: Vec<u8>,
        welcome: Option<Vec<u8>>,
    },
    DaveMlsInvalidCommitWelcome {
        transition_id: u64,
    },
}

impl VoiceGatewayCommand {
    pub fn opcode(&self) -> VoiceGatewayOpcode {
        match self {
            VoiceGatewayCommand::Identify { .. } => VoiceGatewayOpcode::IDENTIFY,
            VoiceGatewayCommand::SelectProtocol(_) => VoiceGatewayOpcode::SELECT_PROTOCOL,
            VoiceGatewayCommand::Speaking(_) => VoiceGatewayOpcode::SPEAKING,
            VoiceGatewayCommand::Resume { .. } => VoiceGatewayOpcode::RESUME,
            VoiceGatewayCommand::Heartbeat { .. } => VoiceGatewayOpcode::HEARTBEAT,
            VoiceGatewayCommand::DaveProtocolTransitionReady { .. } => {
                VoiceGatewayOpcode::DAVE_PROTOCOL_TRANSITION_READY
            }
            VoiceGatewayCommand::DaveMlsKeyPackage { .. } => {
                VoiceGatewayOpcode::DAVE_MLS_KEY_PACKAGE
            }
            VoiceGatewayCommand::DaveMlsCommitWelcome { .. } => {
                VoiceGatewayOpcode::DAVE_MLS_COMMIT_WELCOME
            }
            VoiceGatewayCommand::DaveMlsInvalidCommitWelcome { .. } => {
                VoiceGatewayOpcode::DAVE_MLS_INVALID_COMMIT_WELCOME
            }
        }
    }

    pub fn payload(&self) -> serde_json::Value {
        let data = match self {
            VoiceGatewayCommand::Identify {
                server_id,
                user_id,
                session_id,
                token,
            } => serde_json::json!({
                "server_id": server_id,
                "user_id": user_id,
                "session_id": session_id,
                "token": token,
            }),
            VoiceGatewayCommand::SelectProtocol(command) => {
                serde_json::to_value(command).expect("voice protocol command should serialize")
            }
            VoiceGatewayCommand::Speaking(command) => {
                serde_json::to_value(command).expect("voice speaking command should serialize")
            }
            VoiceGatewayCommand::Resume {
                server_id,
                session_id,
                token,
            } => serde_json::json!({
                "server_id": server_id,
                "session_id": session_id,
                "token": token,
            }),
            VoiceGatewayCommand::Heartbeat { nonce } => serde_json::json!(*nonce),
            VoiceGatewayCommand::DaveProtocolTransitionReady { transition_id }
            | VoiceGatewayCommand::DaveMlsInvalidCommitWelcome { transition_id } => {
                serde_json::json!({
                    "transition_id": transition_id,
                })
            }
            VoiceGatewayCommand::DaveMlsKeyPackage { key_package } => {
                serde_json::json!(key_package)
            }
            VoiceGatewayCommand::DaveMlsCommitWelcome { commit, welcome } => {
                serde_json::json!({
                    "commit": commit,
                    "welcome": welcome,
                })
            }
        };

        serde_json::json!({
            "op": self.opcode().code(),
            "d": data,
        })
    }

    pub fn binary_payload(&self) -> Option<Vec<u8>> {
        match self {
            VoiceGatewayCommand::DaveMlsKeyPackage { key_package } => {
                let mut bytes = Vec::with_capacity(1 + key_package.len());
                bytes.push(self.opcode().code());
                bytes.extend_from_slice(key_package);
                Some(bytes)
            }
            VoiceGatewayCommand::DaveMlsCommitWelcome { commit, welcome } => {
                let welcome_len = welcome.as_ref().map_or(0, Vec::len);
                let mut bytes = Vec::with_capacity(1 + commit.len() + welcome_len);
                bytes.push(self.opcode().code());
                bytes.extend_from_slice(commit);
                if let Some(welcome) = welcome {
                    bytes.extend_from_slice(welcome);
                }
                Some(bytes)
            }
            _ => None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VoiceUdpDiscoveryPacket {
    pub ssrc: u32,
    pub address: String,
    pub port: u16,
}

impl VoiceUdpDiscoveryPacket {
    pub const LEN: usize = 74;
    pub const REQUEST_TYPE: u16 = 1;
    pub const RESPONSE_TYPE: u16 = 2;
    pub const BODY_LEN: u16 = 70;

    pub fn request(ssrc: u32) -> [u8; Self::LEN] {
        let mut packet = [0_u8; Self::LEN];
        packet[..2].copy_from_slice(&Self::REQUEST_TYPE.to_be_bytes());
        packet[2..4].copy_from_slice(&Self::BODY_LEN.to_be_bytes());
        packet[4..8].copy_from_slice(&ssrc.to_be_bytes());
        packet
    }

    pub fn decode(packet: &[u8]) -> Result<Self, DiscordError> {
        if packet.len() < Self::LEN {
            return Err(invalid_data_error(format!(
                "voice discovery packet must be at least {} bytes",
                Self::LEN
            )));
        }

        let packet_type = u16::from_be_bytes([packet[0], packet[1]]);
        if packet_type != Self::RESPONSE_TYPE {
            return Err(invalid_data_error(format!(
                "unexpected voice discovery packet type {packet_type}"
            )));
        }

        let packet_len = u16::from_be_bytes([packet[2], packet[3]]);
        if packet_len != Self::BODY_LEN {
            return Err(invalid_data_error(format!(
                "unexpected voice discovery packet length {packet_len}"
            )));
        }

        let ssrc = u32::from_be_bytes([packet[4], packet[5], packet[6], packet[7]]);
        let address_end = packet[8..72]
            .iter()
            .position(|byte| *byte == 0)
            .map(|offset| offset + 8)
            .unwrap_or(72);
        let address_bytes = &packet[8..address_end];
        let address = std::str::from_utf8(address_bytes)
            .map_err(|error| invalid_data_error(format!("invalid voice discovery ip: {error}")))?
            .to_string();
        let port = u16::from_be_bytes([packet[72], packet[73]]);

        Ok(Self {
            ssrc,
            address,
            port,
        })
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct AudioTrack {
    pub id: String,
    pub source: String,
    pub title: Option<String>,
}

impl AudioTrack {
    pub fn new(id: impl Into<String>, source: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            source: source.into(),
            title: None,
        }
    }

    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct AudioPlayer {
    queue: VecDeque<AudioTrack>,
    current: Option<AudioTrack>,
    volume: f32,
    paused: bool,
}

impl Default for AudioPlayer {
    fn default() -> Self {
        Self::new()
    }
}

impl AudioPlayer {
    pub fn new() -> Self {
        Self {
            queue: VecDeque::new(),
            current: None,
            volume: 1.0,
            paused: false,
        }
    }

    pub fn enqueue(&mut self, track: AudioTrack) {
        self.queue.push_back(track);
    }

    pub fn current(&self) -> Option<&AudioTrack> {
        self.current.as_ref()
    }

    pub fn queue_len(&self) -> usize {
        self.queue.len()
    }

    pub fn volume(&self) -> f32 {
        self.volume
    }

    pub fn set_volume(&mut self, volume: f32) {
        self.volume = volume.clamp(0.0, 1.0);
    }

    pub fn is_paused(&self) -> bool {
        self.paused
    }

    pub fn pause(&mut self) {
        if self.current.is_some() {
            self.paused = true;
        }
    }

    pub fn resume(&mut self) {
        self.paused = false;
    }

    pub fn clear_queue(&mut self) -> usize {
        let removed = self.queue.len();
        self.queue.clear();
        removed
    }

    pub fn skip(&mut self) -> Option<&AudioTrack> {
        self.paused = false;
        self.current = self.queue.pop_front();
        self.current.as_ref()
    }

    pub fn play_next(&mut self) -> Option<&AudioTrack> {
        self.current = self.queue.pop_front();
        self.paused = false;
        self.current.as_ref()
    }

    pub fn stop(&mut self) -> Option<AudioTrack> {
        self.paused = false;
        self.current.take()
    }

    pub fn stop_all(&mut self) -> (Option<AudioTrack>, usize) {
        let current = self.stop();
        let queued = self.clear_queue();
        (current, queued)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum VoiceEvent {
    Connecting {
        guild_id: Snowflake,
        channel_id: Snowflake,
    },
    Connected(VoiceConnectionState),
    Disconnected {
        guild_id: Snowflake,
    },
    ServerUpdated {
        guild_id: Snowflake,
    },
    SessionUpdated {
        guild_id: Snowflake,
    },
    TransportConfigured {
        guild_id: Snowflake,
        transport: VoiceTransportState,
    },
    SpeakingUpdated {
        guild_id: Snowflake,
        speaking: VoiceSpeakingState,
    },
    PlayerStarted {
        guild_id: Snowflake,
        track: AudioTrack,
    },
    PlayerStopped {
        guild_id: Snowflake,
        track: AudioTrack,
    },
    PlayerPaused {
        guild_id: Snowflake,
        track: AudioTrack,
    },
    PlayerResumed {
        guild_id: Snowflake,
        track: AudioTrack,
    },
    PlayerSkipped {
        guild_id: Snowflake,
        track: AudioTrack,
    },
}

#[derive(Clone, Debug, Default)]
pub struct VoiceManager {
    connections: HashMap<Snowflake, VoiceConnectionState>,
    players: HashMap<Snowflake, AudioPlayer>,
    events: Vec<VoiceEvent>,
}

impl VoiceManager {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn connect(&mut self, config: VoiceConnectionConfig) -> VoiceConnectionState {
        let guild_id = config.guild_id.clone();
        let channel_id = config.channel_id.clone();
        self.events.push(VoiceEvent::Connecting {
            guild_id: guild_id.clone(),
            channel_id,
        });

        let state = VoiceConnectionState::from_config(config);
        if state.status == VoiceConnectionStatus::Connected {
            self.events.push(VoiceEvent::Connected(state.clone()));
        }

        self.connections.insert(guild_id.clone(), state.clone());
        self.players.entry(guild_id).or_default();
        state
    }

    pub fn disconnect(&mut self, guild_id: impl Into<Snowflake>) -> Option<VoiceConnectionState> {
        let guild_id = guild_id.into();
        self.players.remove(&guild_id);
        let mut state = self.connections.remove(&guild_id)?;
        state.mark_disconnected();
        self.events.push(VoiceEvent::Disconnected {
            guild_id: guild_id.clone(),
        });
        Some(state)
    }

    pub fn update_server(
        &mut self,
        update: impl Into<VoiceServerUpdate>,
    ) -> Option<VoiceConnectionState> {
        let update = update.into();
        let state = self.connections.get_mut(&update.guild_id)?;
        let was_disconnected = state.status == VoiceConnectionStatus::Disconnected;
        state.apply_server_update(&update);
        let guild_id = state.guild_id.clone();
        self.events.push(VoiceEvent::ServerUpdated {
            guild_id: guild_id.clone(),
        });
        if !was_disconnected && state.status == VoiceConnectionStatus::Disconnected {
            self.events.push(VoiceEvent::Disconnected {
                guild_id: guild_id.clone(),
            });
        }
        if state.status == VoiceConnectionStatus::Connected {
            self.events.push(VoiceEvent::Connected(state.clone()));
        }
        Some(state.clone())
    }

    pub fn update_voice_state(&mut self, update: &VoiceState) -> Option<VoiceConnectionState> {
        let guild_id = update.guild_id.clone()?;
        let state = match self.connections.get_mut(&guild_id) {
            Some(state) => state,
            None => {
                let channel_id = update.channel_id.clone()?;
                let state = VoiceConnectionState::from_config(VoiceConnectionConfig::new(
                    guild_id.clone(),
                    channel_id,
                ));
                self.players.entry(guild_id.clone()).or_default();
                self.connections.insert(guild_id.clone(), state);
                self.connections
                    .get_mut(&guild_id)
                    .expect("voice connection should exist after insert")
            }
        };

        let was_disconnected = state.status == VoiceConnectionStatus::Disconnected;
        state.apply_voice_state(update);
        self.events.push(VoiceEvent::SessionUpdated {
            guild_id: guild_id.clone(),
        });
        if !was_disconnected && state.status == VoiceConnectionStatus::Disconnected {
            self.events.push(VoiceEvent::Disconnected {
                guild_id: guild_id.clone(),
            });
        }
        if state.status == VoiceConnectionStatus::Connected {
            self.events.push(VoiceEvent::Connected(state.clone()));
        }
        Some(state.clone())
    }

    pub fn configure_transport(
        &mut self,
        guild_id: impl Into<Snowflake>,
        transport: VoiceTransportState,
    ) -> Option<VoiceTransportState> {
        let guild_id = guild_id.into();
        let state = self.connections.get_mut(&guild_id)?;
        state.set_transport(transport.clone());
        self.events.push(VoiceEvent::TransportConfigured {
            guild_id,
            transport: transport.clone(),
        });
        Some(transport)
    }

    pub fn set_speaking(
        &mut self,
        guild_id: impl Into<Snowflake>,
        speaking: VoiceSpeakingState,
    ) -> Option<VoiceSpeakingState> {
        let guild_id = guild_id.into();
        let state = self.connections.get_mut(&guild_id)?;
        state.set_speaking(speaking.clone());
        self.events.push(VoiceEvent::SpeakingUpdated {
            guild_id,
            speaking: speaking.clone(),
        });
        Some(speaking)
    }

    pub fn identify_command(
        &self,
        guild_id: impl Into<Snowflake>,
        user_id: impl Into<Snowflake>,
    ) -> Option<VoiceGatewayCommand> {
        let guild_id = guild_id.into();
        let user_id = user_id.into();
        let connection = self.connections.get(&guild_id)?;
        Some(VoiceGatewayCommand::Identify {
            server_id: guild_id,
            user_id,
            session_id: connection.session_id.clone()?,
            token: connection.token.clone()?,
        })
    }

    pub fn runtime_config(
        &self,
        guild_id: impl Into<Snowflake>,
        user_id: impl Into<Snowflake>,
    ) -> Option<VoiceRuntimeConfig> {
        let guild_id = guild_id.into();
        let user_id = user_id.into();
        let connection = self.connections.get(&guild_id)?;
        Some(VoiceRuntimeConfig::new(
            guild_id,
            user_id,
            connection.session_id.clone()?,
            connection.token.clone()?,
            connection.endpoint.clone()?,
        ))
    }

    pub fn resume_command(&self, guild_id: impl Into<Snowflake>) -> Option<VoiceGatewayCommand> {
        let guild_id = guild_id.into();
        let connection = self.connections.get(&guild_id)?;
        Some(VoiceGatewayCommand::Resume {
            server_id: guild_id,
            session_id: connection.session_id.clone()?,
            token: connection.token.clone()?,
        })
    }

    pub fn select_protocol_command(
        &self,
        guild_id: impl Into<Snowflake>,
    ) -> Option<VoiceGatewayCommand> {
        let guild_id = guild_id.into();
        let transport = self.connections.get(&guild_id)?.transport.clone()?;
        Some(VoiceGatewayCommand::SelectProtocol(
            VoiceSelectProtocolCommand::new(
                transport.protocol,
                VoiceSelectProtocolData::new(transport.ip, transport.port, transport.mode),
            ),
        ))
    }

    pub fn speaking_command(
        &mut self,
        guild_id: impl Into<Snowflake>,
        flags: VoiceSpeakingFlags,
        delay: u32,
    ) -> Option<VoiceGatewayCommand> {
        let guild_id = guild_id.into();
        let transport = self.connections.get(&guild_id)?.transport.clone()?;
        self.set_speaking(guild_id.clone(), VoiceSpeakingState::new(flags, delay))?;
        Some(VoiceGatewayCommand::Speaking(
            VoiceSpeakingCommand::new(transport.ssrc)
                .speaking(flags)
                .delay(delay),
        ))
    }

    pub fn connection(&self, guild_id: impl Into<Snowflake>) -> Option<&VoiceConnectionState> {
        let guild_id = guild_id.into();
        self.connections.get(&guild_id)
    }

    pub fn player(&self, guild_id: impl Into<Snowflake>) -> Option<&AudioPlayer> {
        let guild_id = guild_id.into();
        self.players.get(&guild_id)
    }

    pub fn enqueue(&mut self, guild_id: impl Into<Snowflake>, track: AudioTrack) -> Option<usize> {
        let guild_id = guild_id.into();
        let player = self.players.get_mut(&guild_id)?;
        player.enqueue(track);
        Some(player.queue_len())
    }

    pub fn start_next(&mut self, guild_id: impl Into<Snowflake>) -> Option<&AudioTrack> {
        let guild_id = guild_id.into();
        let player = self.players.get_mut(&guild_id)?;
        let started_track = player.play_next().cloned()?;
        self.events.push(VoiceEvent::PlayerStarted {
            guild_id,
            track: started_track,
        });
        player.current()
    }

    pub fn stop(&mut self, guild_id: impl Into<Snowflake>) -> Option<AudioTrack> {
        let guild_id = guild_id.into();
        let player = self.players.get_mut(&guild_id)?;
        let stopped_track = player.stop()?;
        self.events.push(VoiceEvent::PlayerStopped {
            guild_id,
            track: stopped_track.clone(),
        });
        Some(stopped_track)
    }

    pub fn pause(&mut self, guild_id: impl Into<Snowflake>) -> Option<&AudioTrack> {
        let guild_id = guild_id.into();
        let player = self.players.get_mut(&guild_id)?;
        player.pause();
        let paused_track = player.current().cloned()?;
        self.events.push(VoiceEvent::PlayerPaused {
            guild_id,
            track: paused_track,
        });
        player.current()
    }

    pub fn resume(&mut self, guild_id: impl Into<Snowflake>) -> Option<&AudioTrack> {
        let guild_id = guild_id.into();
        let player = self.players.get_mut(&guild_id)?;
        player.resume();
        let resumed_track = player.current().cloned()?;
        self.events.push(VoiceEvent::PlayerResumed {
            guild_id,
            track: resumed_track,
        });
        player.current()
    }

    pub fn skip(&mut self, guild_id: impl Into<Snowflake>) -> Option<&AudioTrack> {
        let guild_id = guild_id.into();
        let player = self.players.get_mut(&guild_id)?;
        let skipped_track = player.current().cloned();
        let next_track = player.skip().cloned()?;
        if let Some(track) = skipped_track {
            self.events.push(VoiceEvent::PlayerSkipped {
                guild_id: guild_id.clone(),
                track,
            });
        }
        self.events.push(VoiceEvent::PlayerStarted {
            guild_id,
            track: next_track,
        });
        player.current()
    }

    pub fn clear_queue(&mut self, guild_id: impl Into<Snowflake>) -> Option<usize> {
        let guild_id = guild_id.into();
        self.players
            .get_mut(&guild_id)
            .map(AudioPlayer::clear_queue)
    }

    pub fn set_volume(&mut self, guild_id: impl Into<Snowflake>, volume: f32) -> Option<f32> {
        let guild_id = guild_id.into();
        let player = self.players.get_mut(&guild_id)?;
        player.set_volume(volume);
        Some(player.volume())
    }

    pub fn events(&self) -> &[VoiceEvent] {
        &self.events
    }
}

#[cfg(test)]
mod tests {
    use super::{
        AudioTrack, VoiceConnectionConfig, VoiceConnectionState, VoiceConnectionStatus,
        VoiceEncryptionMode, VoiceGatewayCommand, VoiceGatewayHello, VoiceGatewayReady,
        VoiceManager, VoiceSelectProtocolCommand, VoiceSpeakingCommand, VoiceSpeakingFlags,
        VoiceTransportProtocol, VoiceTransportState, VoiceUdpDiscoveryPacket,
    };
    use crate::model::{Snowflake, VoiceServerUpdate, VoiceState};

    #[test]
    fn voice_manager_promotes_gateway_updates_into_ready_connection() {
        let mut manager = VoiceManager::new();
        let guild_id = Snowflake::from("1");
        let channel_id = Snowflake::from("2");

        let state = manager.connect(VoiceConnectionConfig::new(guild_id.clone(), channel_id));
        assert_eq!(state.status, VoiceConnectionStatus::Connecting);

        manager.update_voice_state(&VoiceState {
            guild_id: Some(guild_id.clone()),
            channel_id: Some(Snowflake::from("2")),
            session_id: Some("session".to_string()),
            self_mute: false,
            self_deaf: false,
            ..VoiceState::default()
        });
        manager.update_server(VoiceServerUpdate {
            guild_id: guild_id.clone(),
            token: "token".to_string(),
            endpoint: Some("voice.discord.media".to_string()),
        });
        manager.configure_transport(
            guild_id.clone(),
            VoiceTransportState::udp(
                "127.0.0.1",
                5000,
                VoiceEncryptionMode::aead_aes256_gcm_rtpsize(),
                42,
            ),
        );

        let identify = manager
            .identify_command(guild_id.clone(), Snowflake::from("3"))
            .expect("identify command should be available after voice session is ready");
        assert_eq!(identify.opcode().code(), 0);

        let speaking = manager
            .speaking_command(guild_id.clone(), VoiceSpeakingFlags::MICROPHONE, 0)
            .expect("speaking command should use configured transport");
        assert_eq!(speaking.opcode().code(), 5);
        assert_eq!(
            manager.connection(guild_id).unwrap().status,
            VoiceConnectionStatus::Connected
        );
    }

    #[test]
    fn voice_udp_discovery_packet_decodes_discord_response_shape() {
        let mut packet = VoiceUdpDiscoveryPacket::request(1337);
        let address = b"203.0.113.7";
        packet[..2].copy_from_slice(&VoiceUdpDiscoveryPacket::RESPONSE_TYPE.to_be_bytes());
        packet[2..4].copy_from_slice(&VoiceUdpDiscoveryPacket::BODY_LEN.to_be_bytes());
        packet[8..8 + address.len()].copy_from_slice(address);
        packet[72..74].copy_from_slice(&5000_u16.to_be_bytes());

        let decoded = VoiceUdpDiscoveryPacket::decode(&packet).unwrap();
        assert_eq!(decoded.ssrc, 1337);
        assert_eq!(decoded.address, "203.0.113.7");
        assert_eq!(decoded.port, 5000);
    }

    #[test]
    fn voice_gateway_command_payloads_keep_expected_wire_shape() {
        let payload = VoiceGatewayCommand::Heartbeat { nonce: 99 }.payload();
        assert_eq!(payload["op"], serde_json::json!(3));
        assert_eq!(payload["d"], serde_json::json!(99));
    }

    #[test]
    fn voice_gateway_command_payloads_cover_all_variants() {
        let commands = [
            (
                VoiceGatewayCommand::Identify {
                    server_id: Snowflake::from("1"),
                    user_id: Snowflake::from("2"),
                    session_id: "session".to_string(),
                    token: "token".to_string(),
                },
                0,
                serde_json::json!({
                    "server_id": "1",
                    "user_id": "2",
                    "session_id": "session",
                    "token": "token",
                }),
            ),
            (
                VoiceGatewayCommand::Resume {
                    server_id: Snowflake::from("1"),
                    session_id: "session".to_string(),
                    token: "token".to_string(),
                },
                7,
                serde_json::json!({
                    "server_id": "1",
                    "session_id": "session",
                    "token": "token",
                }),
            ),
            (
                VoiceGatewayCommand::SelectProtocol(VoiceSelectProtocolCommand::udp(
                    "203.0.113.7",
                    5000,
                    VoiceEncryptionMode::aead_aes256_gcm_rtpsize(),
                )),
                1,
                serde_json::json!({
                    "protocol": "udp",
                    "data": {
                        "address": "203.0.113.7",
                        "port": 5000,
                        "mode": "aead_aes256_gcm_rtpsize",
                    }
                }),
            ),
            (
                VoiceGatewayCommand::Speaking(
                    VoiceSpeakingCommand::new(42)
                        .speaking(VoiceSpeakingFlags::PRIORITY)
                        .delay(5),
                ),
                5,
                serde_json::json!({
                    "speaking": VoiceSpeakingFlags::PRIORITY.bits(),
                    "delay": 5,
                    "ssrc": 42,
                }),
            ),
        ];

        for (command, opcode, data) in commands {
            let payload = command.payload();
            assert_eq!(command.opcode().code(), opcode);
            assert_eq!(payload["op"], serde_json::json!(opcode));
            assert_eq!(payload["d"], data);
        }
    }

    #[test]
    fn voice_gateway_dave_command_payloads_match_json_and_binary_wire_shapes() {
        let transition_ready =
            VoiceGatewayCommand::DaveProtocolTransitionReady { transition_id: 77 };
        assert_eq!(transition_ready.opcode().code(), 23);
        assert_eq!(transition_ready.payload()["op"], serde_json::json!(23));
        assert_eq!(
            transition_ready.payload()["d"],
            serde_json::json!({ "transition_id": 77 })
        );
        assert_eq!(transition_ready.binary_payload(), None);

        let key_package = VoiceGatewayCommand::DaveMlsKeyPackage {
            key_package: vec![1, 2, 3],
        };
        assert_eq!(key_package.opcode().code(), 26);
        assert_eq!(key_package.binary_payload(), Some(vec![26, 1, 2, 3]));

        let commit_welcome = VoiceGatewayCommand::DaveMlsCommitWelcome {
            commit: vec![4, 5],
            welcome: Some(vec![6, 7]),
        };
        assert_eq!(commit_welcome.opcode().code(), 28);
        assert_eq!(commit_welcome.binary_payload(), Some(vec![28, 4, 5, 6, 7]));

        let invalid = VoiceGatewayCommand::DaveMlsInvalidCommitWelcome { transition_id: 88 };
        assert_eq!(invalid.opcode().code(), 31);
        assert_eq!(invalid.payload()["op"], serde_json::json!(31));
        assert_eq!(
            invalid.payload()["d"],
            serde_json::json!({ "transition_id": 88 })
        );
        assert_eq!(invalid.binary_payload(), None);
    }

    #[test]
    fn voice_player_queue_still_works_with_transport_state() {
        let mut manager = VoiceManager::new();
        manager.connect(
            VoiceConnectionConfig::new("1", "2")
                .session_id("session")
                .token("token")
                .endpoint("voice.discord.media"),
        );

        manager
            .enqueue(
                "1",
                AudioTrack::new("track-1", "memory://track").title("Track 1"),
            )
            .unwrap();
        let current = manager.start_next("1").unwrap();
        assert_eq!(current.id, "track-1");
    }

    #[test]
    fn voice_state_disconnect_clears_runtime_state_and_marks_disconnected() {
        let mut manager = VoiceManager::new();
        let guild_id = Snowflake::from("1");

        manager.connect(
            VoiceConnectionConfig::new(guild_id.clone(), "2")
                .session_id("session")
                .token("token")
                .endpoint("voice.discord.media"),
        );
        manager.configure_transport(
            guild_id.clone(),
            VoiceTransportState::udp(
                "127.0.0.1",
                5000,
                VoiceEncryptionMode::aead_aes256_gcm_rtpsize(),
                42,
            ),
        );
        manager.set_speaking(
            guild_id.clone(),
            super::VoiceSpeakingState::new(VoiceSpeakingFlags::MICROPHONE, 0),
        );

        let state = manager
            .update_voice_state(&VoiceState {
                guild_id: Some(guild_id.clone()),
                channel_id: None,
                session_id: None,
                self_mute: true,
                self_deaf: true,
                ..VoiceState::default()
            })
            .expect("disconnect update should keep tracked state");

        assert_eq!(state.status, VoiceConnectionStatus::Disconnected);
        assert_eq!(state.endpoint, None);
        assert_eq!(state.session_id, None);
        assert_eq!(state.token, None);
        assert_eq!(state.transport, None);
        assert_eq!(state.speaking, None);
        assert_eq!(manager.identify_command(guild_id.clone(), "3"), None);
        assert!(matches!(
            manager.events().last(),
            Some(super::VoiceEvent::Disconnected { guild_id: event_guild_id }) if *event_guild_id == guild_id
        ));

        let tracked = manager.connection(guild_id).unwrap();
        assert_eq!(tracked.status, VoiceConnectionStatus::Disconnected);
        assert_eq!(tracked.endpoint, None);
        assert_eq!(tracked.session_id, None);
        assert_eq!(tracked.token, None);
        assert_eq!(tracked.transport, None);
        assert_eq!(tracked.speaking, None);
    }

    #[test]
    fn voice_server_endpoint_loss_clears_runtime_state_and_marks_disconnected() {
        let mut manager = VoiceManager::new();
        let guild_id = Snowflake::from("1");

        manager.connect(
            VoiceConnectionConfig::new(guild_id.clone(), "2")
                .session_id("session")
                .token("token")
                .endpoint("voice.discord.media"),
        );
        manager.configure_transport(
            guild_id.clone(),
            VoiceTransportState::udp(
                "127.0.0.1",
                5000,
                VoiceEncryptionMode::aead_aes256_gcm_rtpsize(),
                42,
            ),
        );
        manager.set_speaking(
            guild_id.clone(),
            super::VoiceSpeakingState::new(VoiceSpeakingFlags::MICROPHONE, 0),
        );

        let state = manager
            .update_server(VoiceServerUpdate {
                guild_id: guild_id.clone(),
                token: "replacement-token".to_string(),
                endpoint: None,
            })
            .expect("server update should keep tracked state");

        assert_eq!(state.status, VoiceConnectionStatus::Disconnected);
        assert_eq!(state.endpoint, None);
        assert_eq!(state.session_id, None);
        assert_eq!(state.token, None);
        assert_eq!(state.transport, None);
        assert_eq!(state.speaking, None);
        assert_eq!(manager.resume_command(guild_id.clone()), None);
        assert_eq!(manager.runtime_config(guild_id.clone(), "3"), None);
        assert!(matches!(
            manager.events().last(),
            Some(super::VoiceEvent::Disconnected { guild_id: event_guild_id }) if *event_guild_id == guild_id
        ));

        let tracked = manager.connection(guild_id).unwrap();
        assert_eq!(tracked.status, VoiceConnectionStatus::Disconnected);
        assert_eq!(tracked.endpoint, None);
        assert_eq!(tracked.session_id, None);
        assert_eq!(tracked.token, None);
        assert_eq!(tracked.transport, None);
        assert_eq!(tracked.speaking, None);
    }

    #[test]
    fn voice_udp_discovery_packet_rejects_invalid_payloads() {
        let short_packet = [0_u8; VoiceUdpDiscoveryPacket::LEN - 1];
        let short_error = VoiceUdpDiscoveryPacket::decode(&short_packet).unwrap_err();
        assert!(short_error
            .to_string()
            .contains("voice discovery packet must be at least"));

        let mut wrong_type = VoiceUdpDiscoveryPacket::request(7);
        wrong_type[..2].copy_from_slice(&9_u16.to_be_bytes());
        let wrong_type_error = VoiceUdpDiscoveryPacket::decode(&wrong_type).unwrap_err();
        assert!(wrong_type_error
            .to_string()
            .contains("unexpected voice discovery packet type 9"));

        let mut wrong_length = VoiceUdpDiscoveryPacket::request(7);
        wrong_length[..2].copy_from_slice(&VoiceUdpDiscoveryPacket::RESPONSE_TYPE.to_be_bytes());
        wrong_length[2..4].copy_from_slice(&12_u16.to_be_bytes());
        let wrong_length_error = VoiceUdpDiscoveryPacket::decode(&wrong_length).unwrap_err();
        assert!(wrong_length_error
            .to_string()
            .contains("unexpected voice discovery packet length 12"));

        let mut invalid_address = VoiceUdpDiscoveryPacket::request(7);
        invalid_address[..2].copy_from_slice(&VoiceUdpDiscoveryPacket::RESPONSE_TYPE.to_be_bytes());
        invalid_address[2..4].copy_from_slice(&VoiceUdpDiscoveryPacket::BODY_LEN.to_be_bytes());
        invalid_address[8] = 0xff;
        let invalid_address_error = VoiceUdpDiscoveryPacket::decode(&invalid_address).unwrap_err();
        assert!(invalid_address_error
            .to_string()
            .contains("invalid voice discovery ip"));
    }

    #[test]
    fn voice_connection_state_stays_connecting_until_all_runtime_fields_exist() {
        let config = VoiceConnectionConfig::new("1", "2").endpoint("voice.discord.media");
        let mut state = VoiceConnectionState::from_config(config);
        assert_eq!(state.status, VoiceConnectionStatus::Connecting);
        assert!(!state.is_ready());

        state.set_transport(VoiceTransportState::udp(
            "127.0.0.1",
            5000,
            VoiceEncryptionMode::aead_aes256_gcm_rtpsize(),
            42,
        ));
        state.set_speaking(super::VoiceSpeakingState::new(
            VoiceSpeakingFlags::MICROPHONE,
            0,
        ));

        state.apply_server_update(&VoiceServerUpdate {
            guild_id: Snowflake::from("1"),
            token: "token".to_string(),
            endpoint: Some("voice-next.discord.media".to_string()),
        });
        assert_eq!(state.status, VoiceConnectionStatus::Connecting);
        assert_eq!(state.endpoint.as_deref(), Some("voice-next.discord.media"));
        assert_eq!(state.token.as_deref(), Some("token"));
        assert_eq!(state.transport, None);
        assert_eq!(state.speaking, None);

        state.apply_voice_state(&VoiceState {
            guild_id: Some(Snowflake::from("1")),
            channel_id: Some(Snowflake::from("2")),
            session_id: Some("session".to_string()),
            self_mute: true,
            self_deaf: false,
            ..VoiceState::default()
        });
        assert!(state.is_ready());
        assert_eq!(state.status, VoiceConnectionStatus::Connected);
        assert_eq!(state.session_id.as_deref(), Some("session"));
        assert!(state.self_mute);
        assert!(!state.self_deaf);
    }

    #[test]
    fn voice_manager_handles_missing_updates_and_disconnect_cleans_state() {
        let mut manager = VoiceManager::new();
        assert_eq!(manager.update_voice_state(&VoiceState::default()), None);
        assert_eq!(
            manager.update_voice_state(&VoiceState {
                guild_id: Some(Snowflake::from("1")),
                ..VoiceState::default()
            }),
            None
        );

        manager.connect(
            VoiceConnectionConfig::new("1", "2")
                .session_id("session")
                .token("token")
                .endpoint("voice.discord.media"),
        );
        manager
            .enqueue("1", AudioTrack::new("track-1", "memory://track"))
            .unwrap();

        let disconnected = manager.disconnect("1").unwrap();
        assert_eq!(disconnected.status, VoiceConnectionStatus::Disconnected);
        assert_eq!(manager.connection("1"), None);
        assert_eq!(manager.player("1"), None);
        assert!(matches!(
            manager.events().last(),
            Some(super::VoiceEvent::Disconnected { guild_id }) if *guild_id == Snowflake::from("1")
        ));
    }

    #[test]
    fn voice_manager_command_helpers_return_none_without_required_runtime_data() {
        let mut manager = VoiceManager::new();
        manager.connect(VoiceConnectionConfig::new("1", "2"));

        assert_eq!(manager.identify_command("1", "3"), None);
        assert_eq!(manager.runtime_config("1", "3"), None);
        assert_eq!(manager.resume_command("1"), None);
        assert_eq!(manager.select_protocol_command("1"), None);
        assert_eq!(
            manager.speaking_command("1", VoiceSpeakingFlags::MICROPHONE, 0),
            None
        );
    }

    #[test]
    fn voice_manager_records_event_order_for_connect_and_updates() {
        let mut manager = VoiceManager::new();
        let guild_id = Snowflake::from("1");
        manager.connect(
            VoiceConnectionConfig::new(guild_id.clone(), "2")
                .session_id("session")
                .token("token")
                .endpoint("voice.discord.media"),
        );

        assert!(matches!(
            manager.events(),
            [
                super::VoiceEvent::Connecting { guild_id: connect_guild, channel_id },
                super::VoiceEvent::Connected(state),
            ] if *connect_guild == guild_id && *channel_id == Snowflake::from("2") && state.guild_id == guild_id
        ));

        manager.update_server(VoiceServerUpdate {
            guild_id: guild_id.clone(),
            token: "next-token".to_string(),
            endpoint: Some("voice-next.discord.media".to_string()),
        });
        assert!(matches!(
            &manager.events()[2..],
            [
                super::VoiceEvent::ServerUpdated { guild_id: server_guild },
                super::VoiceEvent::Connected(state),
            ] if *server_guild == guild_id && state.guild_id == guild_id
        ));

        manager.update_voice_state(&VoiceState {
            guild_id: Some(guild_id.clone()),
            channel_id: Some(Snowflake::from("2")),
            session_id: Some("next-session".to_string()),
            self_mute: false,
            self_deaf: true,
            ..VoiceState::default()
        });
        assert!(matches!(
            &manager.events()[4..],
            [
                super::VoiceEvent::SessionUpdated { guild_id: session_guild },
                super::VoiceEvent::Connected(state),
            ] if *session_guild == guild_id && state.session_id.as_deref() == Some("next-session")
        ));
    }

    #[test]
    fn voice_audio_player_controls_preserve_fifo_queue_and_current_track() {
        let mut player = super::AudioPlayer::new();
        assert_eq!(player.play_next(), None);
        assert_eq!(player.stop(), None);
        assert!(!player.is_paused());
        assert_eq!(player.volume(), 1.0);

        player.enqueue(AudioTrack::new("track-1", "memory://one"));
        player.enqueue(AudioTrack::new("track-2", "memory://two"));
        assert_eq!(player.queue_len(), 2);

        let first = player.play_next().unwrap();
        assert_eq!(first.id, "track-1");
        assert_eq!(player.queue_len(), 1);
        player.pause();
        assert!(player.is_paused());
        player.resume();
        assert!(!player.is_paused());
        player.set_volume(1.5);
        assert_eq!(player.volume(), 1.0);
        player.set_volume(0.25);
        assert_eq!(player.volume(), 0.25);

        let stopped = player.stop().unwrap();
        assert_eq!(stopped.id, "track-1");
        assert_eq!(player.queue_len(), 1);
        assert_eq!(player.current(), None);

        let second = player.play_next().unwrap().clone();
        assert_eq!(second.id, "track-2");
        assert_eq!(player.queue_len(), 0);
        assert_eq!(player.stop_all(), (Some(second), 0));
        player.enqueue(AudioTrack::new("track-3", "memory://three"));
        player.enqueue(AudioTrack::new("track-4", "memory://four"));
        assert_eq!(player.clear_queue(), 2);
        assert_eq!(player.queue_len(), 0);
    }

    #[test]
    fn voice_manager_exposes_playback_queue_controls_and_events() {
        let mut manager = VoiceManager::new();
        let guild_id = Snowflake::from("1");
        manager.connect(
            VoiceConnectionConfig::new(guild_id.clone(), "2")
                .session_id("session")
                .token("token")
                .endpoint("voice.discord.media"),
        );
        manager
            .enqueue(guild_id.clone(), AudioTrack::new("track-1", "memory://one"))
            .unwrap();
        manager
            .enqueue(guild_id.clone(), AudioTrack::new("track-2", "memory://two"))
            .unwrap();

        assert_eq!(manager.set_volume(guild_id.clone(), 2.0), Some(1.0));
        assert_eq!(
            manager
                .start_next(guild_id.clone())
                .map(|track| track.id.as_str()),
            Some("track-1")
        );
        assert_eq!(
            manager
                .pause(guild_id.clone())
                .map(|track| track.id.as_str()),
            Some("track-1")
        );
        assert!(manager.player(guild_id.clone()).unwrap().is_paused());
        assert_eq!(
            manager
                .resume(guild_id.clone())
                .map(|track| track.id.as_str()),
            Some("track-1")
        );
        assert!(!manager.player(guild_id.clone()).unwrap().is_paused());
        assert_eq!(
            manager
                .skip(guild_id.clone())
                .map(|track| track.id.as_str()),
            Some("track-2")
        );
        assert_eq!(manager.clear_queue(guild_id.clone()), Some(0));
        assert!(matches!(
            manager.events(),
            events if events.iter().any(|event| matches!(
                event,
                super::VoiceEvent::PlayerPaused { track, .. } if track.id == "track-1"
            )) && events.iter().any(|event| matches!(
                event,
                super::VoiceEvent::PlayerResumed { track, .. } if track.id == "track-1"
            )) && events.iter().any(|event| matches!(
                event,
                super::VoiceEvent::PlayerSkipped { track, .. } if track.id == "track-1"
            ))
        ));
    }

    #[test]
    fn voice_builder_and_wrapper_types_serialize_expected_wire_shape() {
        assert_eq!(
            VoiceTransportProtocol::udp(),
            VoiceTransportProtocol::new("udp")
        );
        assert_eq!(
            VoiceEncryptionMode::aead_aes256_gcm_rtpsize(),
            VoiceEncryptionMode::new("aead_aes256_gcm_rtpsize")
        );
        assert_eq!(
            VoiceEncryptionMode::aead_xchacha20_poly1305_rtpsize(),
            VoiceEncryptionMode::new("aead_xchacha20_poly1305_rtpsize")
        );
        assert_eq!(VoiceSpeakingFlags::MICROPHONE.bits(), 1);
        assert_eq!(VoiceSpeakingFlags::SOUNDSHARE.bits(), 2);
        assert_eq!(VoiceSpeakingFlags::PRIORITY.bits(), 4);
        assert_eq!(VoiceGatewayHello::new(250).heartbeat_interval_ms, 250);

        let ready = VoiceGatewayReady::new(42, "127.0.0.1", 5000)
            .mode(VoiceEncryptionMode::aead_aes256_gcm_rtpsize())
            .mode(VoiceEncryptionMode::aead_xchacha20_poly1305_rtpsize())
            .heartbeat_interval_ms(250);
        assert_eq!(
            serde_json::to_value(&ready).unwrap(),
            serde_json::json!({
                "ssrc": 42,
                "ip": "127.0.0.1",
                "port": 5000,
                "modes": [
                    "aead_aes256_gcm_rtpsize",
                    "aead_xchacha20_poly1305_rtpsize"
                ],
                "heartbeat_interval_ms": 250,
            })
        );
    }
}
