use std::sync::Arc;

use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::net::UdpSocket;
use tokio::sync::{mpsc, oneshot, watch};
use tokio::task::JoinHandle;
use tokio::time::{interval, Duration};
use tokio_tungstenite::{connect_async, tungstenite::Message as WsMessage};

use crate::error::DiscordError;
use crate::model::Snowflake;
use crate::types::invalid_data_error;
use crate::voice::{
    VoiceEncryptionMode, VoiceGatewayCommand, VoiceGatewayReady, VoiceSelectProtocolCommand,
    VoiceSpeakingCommand, VoiceSpeakingFlags, VoiceUdpDiscoveryPacket,
};

const VOICE_OP_READY: u64 = 2;
const VOICE_OP_HEARTBEAT: u64 = 3;
const VOICE_OP_SESSION_DESCRIPTION: u64 = 4;
const VOICE_OP_HEARTBEAT_ACK: u64 = 6;
const VOICE_OP_RESUME: u64 = 7;
const VOICE_OP_HELLO: u64 = 8;
const VOICE_OP_RESUMED: u64 = 9;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VoiceRuntimeConfig {
    pub server_id: Snowflake,
    pub user_id: Snowflake,
    pub session_id: String,
    pub token: String,
    pub endpoint: String,
    pub gateway_version: u8,
    pub preferred_mode: Option<VoiceEncryptionMode>,
    pub max_dave_protocol_version: Option<u8>,
}

impl VoiceRuntimeConfig {
    pub fn new(
        server_id: impl Into<Snowflake>,
        user_id: impl Into<Snowflake>,
        session_id: impl Into<String>,
        token: impl Into<String>,
        endpoint: impl Into<String>,
    ) -> Self {
        Self {
            server_id: server_id.into(),
            user_id: user_id.into(),
            session_id: session_id.into(),
            token: token.into(),
            endpoint: endpoint.into(),
            gateway_version: 8,
            preferred_mode: None,
            max_dave_protocol_version: Some(1),
        }
    }

    pub fn gateway_version(mut self, gateway_version: u8) -> Self {
        self.gateway_version = gateway_version.max(4);
        self
    }

    pub fn preferred_mode(mut self, preferred_mode: VoiceEncryptionMode) -> Self {
        self.preferred_mode = Some(preferred_mode);
        self
    }

    pub fn max_dave_protocol_version(mut self, version: u8) -> Self {
        self.max_dave_protocol_version = Some(version);
        self
    }

    pub fn websocket_url(&self) -> String {
        let mut endpoint = if self.endpoint.contains("://") {
            self.endpoint.clone()
        } else {
            format!("wss://{}", self.endpoint)
        };

        if !endpoint.contains("?v=") {
            let separator = if endpoint.contains('?') { "&" } else { "/?" };
            endpoint.push_str(separator);
            endpoint.push_str(&format!("v={}", self.gateway_version.max(4)));
        }

        endpoint
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct VoiceSessionDescription {
    pub mode: VoiceEncryptionMode,
    #[serde(default)]
    pub secret_key: Vec<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub audio_codec: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dave_protocol_version: Option<u8>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VoiceRuntimeState {
    pub config: VoiceRuntimeConfig,
    pub heartbeat_interval_ms: u64,
    pub last_sequence: Option<i64>,
    pub ready: VoiceGatewayReady,
    pub discovery: VoiceUdpDiscoveryPacket,
    pub selected_mode: VoiceEncryptionMode,
    pub session_description: Option<VoiceSessionDescription>,
    pub resumed: bool,
}

pub struct VoiceRuntimeHandle {
    state_rx: watch::Receiver<VoiceRuntimeState>,
    command_tx: mpsc::UnboundedSender<VoiceGatewayCommand>,
    close_tx: Option<oneshot::Sender<()>>,
    task: JoinHandle<Result<(), DiscordError>>,
    udp_socket: Arc<UdpSocket>,
}

impl VoiceRuntimeHandle {
    pub fn state(&self) -> VoiceRuntimeState {
        self.state_rx.borrow().clone()
    }

    pub fn subscribe(&self) -> watch::Receiver<VoiceRuntimeState> {
        self.state_rx.clone()
    }

    pub fn udp_socket(&self) -> Arc<UdpSocket> {
        Arc::clone(&self.udp_socket)
    }

    pub fn send(&self, command: VoiceGatewayCommand) -> Result<(), DiscordError> {
        self.command_tx
            .send(command)
            .map_err(|error| invalid_data_error(format!("failed to send voice command: {error}")))
    }

    pub fn set_speaking(&self, flags: VoiceSpeakingFlags, delay: u32) -> Result<(), DiscordError> {
        let ssrc = self.state().ready.ssrc;
        self.send(VoiceGatewayCommand::Speaking(
            VoiceSpeakingCommand::new(ssrc).speaking(flags).delay(delay),
        ))
    }

    pub async fn close(mut self) -> Result<(), DiscordError> {
        if let Some(close_tx) = self.close_tx.take() {
            let _ = close_tx.send(());
        }

        match self.task.await {
            Ok(Ok(())) => Ok(()),
            Ok(Err(error)) if error.to_string().contains("ResetWithoutClosingHandshake") => Ok(()),
            Ok(result) => result,
            Err(error) => Err(format!("voice runtime task failed: {error}").into()),
        }
    }
}

pub async fn connect(config: VoiceRuntimeConfig) -> Result<VoiceRuntimeHandle, DiscordError> {
    let websocket_url = config.websocket_url();
    let (ws_stream, _) = connect_async(&websocket_url).await?;
    let (mut write, mut read) = ws_stream.split();

    let hello = read_voice_payload(&mut read).await?;
    let heartbeat_interval_ms = read_hello_interval(&hello)?;

    let identify = build_identify_payload(&config);
    write
        .send(WsMessage::Text(identify.to_string().into()))
        .await?;

    let mut last_sequence = hello.get("seq").and_then(Value::as_i64);
    let ready_payload =
        wait_for_voice_opcode(&mut read, VOICE_OP_READY, &mut last_sequence).await?;
    let ready: VoiceGatewayReady = serde_json::from_value(
        ready_payload
            .get("d")
            .cloned()
            .ok_or_else(|| invalid_data_error("missing ready data"))?,
    )?;

    let udp_socket = Arc::new(UdpSocket::bind("0.0.0.0:0").await?);
    udp_socket.connect((&*ready.ip, ready.port)).await?;

    let request = VoiceUdpDiscoveryPacket::request(ready.ssrc);
    udp_socket.send(&request).await?;

    let mut discovery_buffer = [0_u8; VoiceUdpDiscoveryPacket::LEN];
    let received = udp_socket.recv(&mut discovery_buffer).await?;
    let discovery = VoiceUdpDiscoveryPacket::decode(&discovery_buffer[..received])?;
    let selected_mode = select_encryption_mode(&config, &ready)?;

    let select_protocol = VoiceGatewayCommand::SelectProtocol(VoiceSelectProtocolCommand::udp(
        discovery.address.clone(),
        discovery.port,
        selected_mode.clone(),
    ));
    write
        .send(WsMessage::Text(
            select_protocol.payload().to_string().into(),
        ))
        .await?;

    let session_description_payload =
        wait_for_voice_opcode(&mut read, VOICE_OP_SESSION_DESCRIPTION, &mut last_sequence).await?;
    let session_description: VoiceSessionDescription = serde_json::from_value(
        session_description_payload
            .get("d")
            .cloned()
            .ok_or_else(|| invalid_data_error("missing session description data"))?,
    )?;

    let initial_state = VoiceRuntimeState {
        config,
        heartbeat_interval_ms,
        last_sequence,
        ready,
        discovery,
        selected_mode,
        session_description: Some(session_description),
        resumed: false,
    };
    let (state_tx, state_rx) = watch::channel(initial_state);
    let (command_tx, mut command_rx) = mpsc::unbounded_channel::<VoiceGatewayCommand>();
    let (close_tx, mut close_rx) = oneshot::channel();
    let udp_socket_handle = Arc::clone(&udp_socket);

    let task = tokio::spawn(async move {
        let mut heartbeat = interval(Duration::from_millis(heartbeat_interval_ms));
        let mut seq_ack = state_tx.borrow().last_sequence;

        loop {
            tokio::select! {
                _ = heartbeat.tick() => {
                    let heartbeat_payload = build_heartbeat_payload(heartbeat_interval_ms, seq_ack);
                    write.send(WsMessage::Text(heartbeat_payload.to_string().into())).await?;
                }
                command = command_rx.recv() => {
                    match command {
                        Some(command) => {
                            write.send(WsMessage::Text(command.payload().to_string().into())).await?;
                        }
                        None => break,
                    }
                }
                _ = &mut close_rx => {
                    let _ = write.send(WsMessage::Close(None)).await;
                    break;
                }
                message = read.next() => {
                    match message {
                        Some(Ok(WsMessage::Text(text))) => {
                            let payload: Value = serde_json::from_str(&text)?;
                            if let Some(seq) = payload.get("seq").and_then(Value::as_i64) {
                                seq_ack = Some(seq);
                                update_state(&state_tx, |state| state.last_sequence = Some(seq))?;
                            }

                            match payload.get("op").and_then(Value::as_u64) {
                                Some(VOICE_OP_SESSION_DESCRIPTION) => {
                                    let description: VoiceSessionDescription = serde_json::from_value(
                                        payload.get("d").cloned().ok_or_else(|| invalid_data_error("missing session description data"))?
                                    )?;
                                    update_state(&state_tx, |state| state.session_description = Some(description))?;
                                }
                                Some(VOICE_OP_RESUMED) => {
                                    update_state(&state_tx, |state| state.resumed = true)?;
                                }
                                Some(VOICE_OP_HEARTBEAT_ACK) | Some(VOICE_OP_HELLO) | Some(VOICE_OP_READY) | Some(VOICE_OP_HEARTBEAT) | Some(VOICE_OP_RESUME) => {}
                                _ => {}
                            }
                        }
                        Some(Ok(WsMessage::Binary(bytes))) => {
                            if bytes.len() >= 2 {
                                let seq = i64::from(u16::from_be_bytes([bytes[0], bytes[1]]));
                                seq_ack = Some(seq);
                                update_state(&state_tx, |state| state.last_sequence = Some(seq))?;
                            }
                        }
                        Some(Ok(WsMessage::Close(_))) => break,
                        Some(Ok(_)) => {}
                        Some(Err(error)) => return Err(error.into()),
                        None => break,
                    }
                }
            }
        }

        Ok(())
    });

    Ok(VoiceRuntimeHandle {
        state_rx,
        command_tx,
        close_tx: Some(close_tx),
        task,
        udp_socket: udp_socket_handle,
    })
}

fn build_identify_payload(config: &VoiceRuntimeConfig) -> Value {
    let mut payload = serde_json::json!({
        "op": 0,
        "d": {
            "server_id": config.server_id,
            "user_id": config.user_id,
            "session_id": config.session_id,
            "token": config.token,
        }
    });

    if let Some(max_dave_protocol_version) = config.max_dave_protocol_version {
        payload["d"]["max_dave_protocol_version"] = serde_json::json!(max_dave_protocol_version);
    }

    payload
}

fn build_heartbeat_payload(heartbeat_nonce: u64, seq_ack: Option<i64>) -> Value {
    serde_json::json!({
        "op": VOICE_OP_HEARTBEAT,
        "d": {
            "t": heartbeat_nonce,
            "seq_ack": seq_ack.unwrap_or(-1),
        }
    })
}

fn read_hello_interval(payload: &Value) -> Result<u64, DiscordError> {
    payload
        .get("d")
        .and_then(|data| data.get("heartbeat_interval"))
        .and_then(Value::as_u64)
        .ok_or_else(|| invalid_data_error("missing voice hello heartbeat interval"))
}

fn select_encryption_mode(
    config: &VoiceRuntimeConfig,
    ready: &VoiceGatewayReady,
) -> Result<VoiceEncryptionMode, DiscordError> {
    if let Some(preferred_mode) = &config.preferred_mode {
        if ready.modes.contains(preferred_mode) {
            return Ok(preferred_mode.clone());
        }
    }

    ready
        .modes
        .first()
        .cloned()
        .ok_or_else(|| invalid_data_error("voice ready payload did not include encryption modes"))
}

fn update_state(
    state_tx: &watch::Sender<VoiceRuntimeState>,
    update: impl FnOnce(&mut VoiceRuntimeState),
) -> Result<(), DiscordError> {
    let mut state = state_tx.borrow().clone();
    update(&mut state);
    state_tx.send(state).map_err(|error| {
        invalid_data_error(format!("failed to publish voice runtime state: {error}"))
    })
}

async fn read_voice_payload(
    read: &mut futures_util::stream::SplitStream<
        tokio_tungstenite::WebSocketStream<
            tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
        >,
    >,
) -> Result<Value, DiscordError> {
    loop {
        match read.next().await {
            Some(Ok(WsMessage::Text(text))) => return Ok(serde_json::from_str(&text)?),
            Some(Ok(_)) => {}
            Some(Err(error)) => return Err(error.into()),
            None => return Err(invalid_data_error("voice websocket closed unexpectedly")),
        }
    }
}

async fn wait_for_voice_opcode(
    read: &mut futures_util::stream::SplitStream<
        tokio_tungstenite::WebSocketStream<
            tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
        >,
    >,
    opcode: u64,
    last_sequence: &mut Option<i64>,
) -> Result<Value, DiscordError> {
    loop {
        let payload = read_voice_payload(read).await?;
        if let Some(seq) = payload.get("seq").and_then(Value::as_i64) {
            *last_sequence = Some(seq);
        }
        if payload.get("op").and_then(Value::as_u64) == Some(opcode) {
            return Ok(payload);
        }
    }
}

#[cfg(test)]
mod tests {
    use futures_util::{SinkExt, StreamExt};
    use serde_json::Value;
    use tokio::net::{TcpListener, UdpSocket};
    use tokio::sync::{oneshot, watch};
    use tokio::time::{timeout, Duration};
    use tokio_tungstenite::{accept_async, tungstenite::Message as WsMessage};

    use super::{
        build_heartbeat_payload, build_identify_payload, connect, read_hello_interval,
        select_encryption_mode, update_state, VoiceRuntimeConfig, VoiceRuntimeState,
        VoiceSessionDescription,
    };
    use crate::voice::{
        VoiceEncryptionMode, VoiceGatewayCommand, VoiceGatewayReady, VoiceSpeakingFlags,
        VoiceUdpDiscoveryPacket,
    };

    #[test]
    fn voice_runtime_config_normalizes_voice_gateway_url() {
        let config =
            VoiceRuntimeConfig::new("1", "2", "session", "token", "voice.discord.media:443");
        assert_eq!(config.websocket_url(), "wss://voice.discord.media:443/?v=8");
    }

    #[test]
    fn voice_runtime_config_clamps_version_and_extends_existing_query() {
        let config = VoiceRuntimeConfig::new(
            "1",
            "2",
            "session",
            "token",
            "ws://127.0.0.1/socket?encoding=json",
        )
        .gateway_version(2)
        .preferred_mode(VoiceEncryptionMode::aead_xchacha20_poly1305_rtpsize())
        .max_dave_protocol_version(3);

        assert_eq!(
            config.websocket_url(),
            "ws://127.0.0.1/socket?encoding=json&v=4"
        );
        assert_eq!(
            config.preferred_mode,
            Some(VoiceEncryptionMode::aead_xchacha20_poly1305_rtpsize())
        );
        assert_eq!(config.max_dave_protocol_version, Some(3));
    }

    #[test]
    fn voice_runtime_helper_builders_cover_optional_and_fallback_fields() {
        let config = VoiceRuntimeConfig {
            max_dave_protocol_version: None,
            ..VoiceRuntimeConfig::new("1", "2", "session", "token", "voice.discord.media")
        };
        assert_eq!(
            build_identify_payload(&config),
            serde_json::json!({
                "op": 0,
                "d": {
                    "server_id": "1",
                    "user_id": "2",
                    "session_id": "session",
                    "token": "token",
                }
            })
        );
        assert_eq!(
            build_heartbeat_payload(55, None),
            serde_json::json!({
                "op": 3,
                "d": {
                    "t": 55,
                    "seq_ack": -1,
                }
            })
        );
        assert_eq!(
            build_heartbeat_payload(55, Some(7)),
            serde_json::json!({
                "op": 3,
                "d": {
                    "t": 55,
                    "seq_ack": 7,
                }
            })
        );
    }

    #[test]
    fn voice_runtime_read_hello_interval_and_mode_selection_handle_edge_cases() {
        assert_eq!(
            read_hello_interval(&serde_json::json!({
                "d": { "heartbeat_interval": 250 }
            }))
            .unwrap(),
            250
        );
        assert!(read_hello_interval(&serde_json::json!({ "d": {} }))
            .unwrap_err()
            .to_string()
            .contains("missing voice hello heartbeat interval"));

        let preferred = VoiceEncryptionMode::aead_xchacha20_poly1305_rtpsize();
        let fallback = VoiceEncryptionMode::aead_aes256_gcm_rtpsize();
        let ready = VoiceGatewayReady::new(42, "127.0.0.1", 5000)
            .mode(fallback.clone())
            .mode(preferred.clone());
        let config = VoiceRuntimeConfig::new("1", "2", "session", "token", "voice.discord.media")
            .preferred_mode(preferred.clone());
        assert_eq!(select_encryption_mode(&config, &ready).unwrap(), preferred);

        let fallback_only = VoiceGatewayReady::new(42, "127.0.0.1", 5000).mode(fallback.clone());
        let config_without_match =
            VoiceRuntimeConfig::new("1", "2", "session", "token", "voice.discord.media")
                .preferred_mode(VoiceEncryptionMode::aead_xchacha20_poly1305_rtpsize());
        assert_eq!(
            select_encryption_mode(&config_without_match, &fallback_only).unwrap(),
            fallback
        );

        let empty_ready = VoiceGatewayReady::new(42, "127.0.0.1", 5000);
        assert!(select_encryption_mode(
            &VoiceRuntimeConfig::new("1", "2", "session", "token", "voice.discord.media"),
            &empty_ready
        )
        .unwrap_err()
        .to_string()
        .contains("did not include encryption modes"));
    }

    #[test]
    fn voice_runtime_update_state_publishes_changes_and_reports_closed_receivers() {
        let initial_state = VoiceRuntimeState {
            config: VoiceRuntimeConfig::new("1", "2", "session", "token", "voice.discord.media"),
            heartbeat_interval_ms: 250,
            last_sequence: Some(3),
            ready: VoiceGatewayReady::new(42, "127.0.0.1", 5000),
            discovery: VoiceUdpDiscoveryPacket {
                ssrc: 42,
                address: "203.0.113.7".to_string(),
                port: 5000,
            },
            selected_mode: VoiceEncryptionMode::aead_aes256_gcm_rtpsize(),
            session_description: None,
            resumed: false,
        };
        let (state_tx, state_rx) = watch::channel(initial_state.clone());
        update_state(&state_tx, |state| {
            state.last_sequence = Some(9);
            state.resumed = true;
        })
        .unwrap();

        let updated = state_rx.borrow().clone();
        assert_eq!(updated.last_sequence, Some(9));
        assert!(updated.resumed);

        let (closed_tx, closed_rx) = watch::channel(initial_state);
        drop(closed_rx);
        assert!(update_state(&closed_tx, |state| state.resumed = true)
            .unwrap_err()
            .to_string()
            .contains("failed to publish voice runtime state"));
    }

    #[tokio::test]
    async fn voice_runtime_connects_and_completes_handshake() {
        let udp_listener = UdpSocket::bind("127.0.0.1:0").await.unwrap();
        let udp_port = udp_listener.local_addr().unwrap().port();

        let tcp_listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let ws_port = tcp_listener.local_addr().unwrap().port();
        let (speaking_tx, speaking_rx) = oneshot::channel();

        let server = tokio::spawn(async move {
            let mut speaking_tx = Some(speaking_tx);
            let (tcp_stream, _) = tcp_listener.accept().await.unwrap();
            let mut ws = accept_async(tcp_stream).await.unwrap();

            ws.send(WsMessage::Text(
                serde_json::json!({
                    "op": 8,
                    "d": { "heartbeat_interval": 20 }
                })
                .to_string()
                .into(),
            ))
            .await
            .unwrap();

            let identify = ws.next().await.unwrap().unwrap().into_text().unwrap();
            let identify_payload: Value = serde_json::from_str(&identify).unwrap();
            assert_eq!(identify_payload["op"], serde_json::json!(0));

            ws.send(WsMessage::Text(
                serde_json::json!({
                    "op": 2,
                    "d": {
                        "ssrc": 42,
                        "ip": "127.0.0.1",
                        "port": udp_port,
                        "modes": ["aead_aes256_gcm_rtpsize"]
                    },
                    "seq": 7
                })
                .to_string()
                .into(),
            ))
            .await
            .unwrap();

            let mut discovery_buffer = [0_u8; VoiceUdpDiscoveryPacket::LEN];
            let (received, remote_addr) =
                udp_listener.recv_from(&mut discovery_buffer).await.unwrap();
            assert_eq!(received, VoiceUdpDiscoveryPacket::LEN);

            let mut response = VoiceUdpDiscoveryPacket::request(42);
            response[..2].copy_from_slice(&VoiceUdpDiscoveryPacket::RESPONSE_TYPE.to_be_bytes());
            response[2..4].copy_from_slice(&VoiceUdpDiscoveryPacket::BODY_LEN.to_be_bytes());
            let address = b"203.0.113.7";
            response[8..8 + address.len()].copy_from_slice(address);
            response[72..74].copy_from_slice(&remote_addr.port().to_be_bytes());
            udp_listener.send_to(&response, remote_addr).await.unwrap();

            let select_protocol = ws.next().await.unwrap().unwrap().into_text().unwrap();
            let select_protocol_payload: Value = serde_json::from_str(&select_protocol).unwrap();
            assert_eq!(select_protocol_payload["op"], serde_json::json!(1));

            ws.send(WsMessage::Text(
                serde_json::json!({
                    "op": 4,
                    "d": {
                        "mode": "aead_aes256_gcm_rtpsize",
                        "secret_key": [1, 2, 3, 4],
                        "dave_protocol_version": 1
                    }
                })
                .to_string()
                .into(),
            ))
            .await
            .unwrap();

            loop {
                let message = ws.next().await.unwrap().unwrap();
                match message {
                    WsMessage::Text(text) => {
                        let payload: Value = serde_json::from_str(&text).unwrap();
                        if payload["op"] == serde_json::json!(5) {
                            if let Some(speaking_tx) = speaking_tx.take() {
                                let _ = speaking_tx.send(());
                            }
                        }
                    }
                    WsMessage::Close(frame) => {
                        let _ = ws.send(WsMessage::Close(frame)).await;
                        break;
                    }
                    _ => {}
                }
            }
        });

        let handle = connect(VoiceRuntimeConfig::new(
            "1",
            "2",
            "session",
            "token",
            format!("ws://127.0.0.1:{ws_port}"),
        ))
        .await
        .unwrap();

        assert_eq!(handle.state().ready.ssrc, 42);
        assert_eq!(handle.state().discovery.address, "203.0.113.7");
        handle
            .set_speaking(VoiceSpeakingFlags::MICROPHONE, 0)
            .unwrap();
        speaking_rx.await.unwrap();
        handle.close().await.unwrap();
        server.await.unwrap();
    }

    #[tokio::test]
    async fn voice_runtime_loop_updates_state_and_sends_custom_commands() {
        let udp_listener = UdpSocket::bind("127.0.0.1:0").await.unwrap();
        let udp_port = udp_listener.local_addr().unwrap().port();

        let tcp_listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let ws_port = tcp_listener.local_addr().unwrap().port();
        let (heartbeat_tx, heartbeat_rx) = oneshot::channel();

        let server = tokio::spawn(async move {
            let mut heartbeat_tx = Some(heartbeat_tx);
            let (tcp_stream, _) = tcp_listener.accept().await.unwrap();
            let mut ws = accept_async(tcp_stream).await.unwrap();

            ws.send(WsMessage::Text(
                serde_json::json!({
                    "op": 8,
                    "d": { "heartbeat_interval": 5_000 },
                    "seq": 3
                })
                .to_string()
                .into(),
            ))
            .await
            .unwrap();

            let identify = ws.next().await.unwrap().unwrap().into_text().unwrap();
            let identify_payload: Value = serde_json::from_str(&identify).unwrap();
            assert_eq!(identify_payload["d"]["max_dave_protocol_version"], 1);

            ws.send(WsMessage::Text(
                serde_json::json!({
                    "op": 2,
                    "d": {
                        "ssrc": 42,
                        "ip": "127.0.0.1",
                        "port": udp_port,
                        "modes": [
                            "aead_aes256_gcm_rtpsize",
                            "aead_xchacha20_poly1305_rtpsize"
                        ]
                    },
                    "seq": 7
                })
                .to_string()
                .into(),
            ))
            .await
            .unwrap();

            let mut discovery_buffer = [0_u8; VoiceUdpDiscoveryPacket::LEN];
            let (received, remote_addr) =
                udp_listener.recv_from(&mut discovery_buffer).await.unwrap();
            assert_eq!(received, VoiceUdpDiscoveryPacket::LEN);

            let mut response = VoiceUdpDiscoveryPacket::request(42);
            response[..2].copy_from_slice(&VoiceUdpDiscoveryPacket::RESPONSE_TYPE.to_be_bytes());
            response[2..4].copy_from_slice(&VoiceUdpDiscoveryPacket::BODY_LEN.to_be_bytes());
            let address = b"203.0.113.7";
            response[8..8 + address.len()].copy_from_slice(address);
            response[72..74].copy_from_slice(&remote_addr.port().to_be_bytes());
            udp_listener.send_to(&response, remote_addr).await.unwrap();

            let select_protocol = ws.next().await.unwrap().unwrap().into_text().unwrap();
            let select_protocol_payload: Value = serde_json::from_str(&select_protocol).unwrap();
            assert_eq!(
                select_protocol_payload["d"]["data"]["mode"],
                serde_json::json!("aead_xchacha20_poly1305_rtpsize")
            );

            ws.send(WsMessage::Text(
                serde_json::json!({
                    "op": 4,
                    "d": {
                        "mode": "aead_xchacha20_poly1305_rtpsize",
                        "secret_key": [1, 2, 3, 4]
                    },
                    "seq": 9
                })
                .to_string()
                .into(),
            ))
            .await
            .unwrap();

            ws.send(WsMessage::Text(
                serde_json::json!({
                    "op": 4,
                    "d": {
                        "mode": "aead_xchacha20_poly1305_rtpsize",
                        "secret_key": [9, 9],
                        "audio_codec": "opus"
                    },
                    "seq": 11
                })
                .to_string()
                .into(),
            ))
            .await
            .unwrap();
            ws.send(WsMessage::Text(
                serde_json::json!({
                    "op": 9,
                    "seq": 12
                })
                .to_string()
                .into(),
            ))
            .await
            .unwrap();
            ws.send(WsMessage::Binary(vec![0, 25].into()))
                .await
                .unwrap();

            loop {
                let message = ws.next().await.unwrap().unwrap();
                match message {
                    WsMessage::Text(text) => {
                        let payload: Value = serde_json::from_str(&text).unwrap();
                        if payload["op"] == serde_json::json!(3)
                            && payload["d"] == serde_json::json!(55)
                        {
                            if let Some(heartbeat_tx) = heartbeat_tx.take() {
                                let _ = heartbeat_tx.send(());
                            }
                        }
                    }
                    WsMessage::Close(frame) => {
                        let _ = ws.send(WsMessage::Close(frame)).await;
                        break;
                    }
                    _ => {}
                }
            }
        });

        let handle = connect(
            VoiceRuntimeConfig::new(
                "1",
                "2",
                "session",
                "token",
                format!("ws://127.0.0.1:{ws_port}"),
            )
            .preferred_mode(VoiceEncryptionMode::aead_xchacha20_poly1305_rtpsize()),
        )
        .await
        .unwrap();

        let mut state_rx = handle.subscribe();
        handle
            .send(VoiceGatewayCommand::Heartbeat { nonce: 55 })
            .unwrap();
        heartbeat_rx.await.unwrap();

        timeout(Duration::from_secs(1), async {
            loop {
                let state = state_rx.borrow().clone();
                if state.resumed
                    && state.last_sequence == Some(25)
                    && state
                        .session_description
                        .as_ref()
                        .map(|description| description.secret_key.as_slice() == [9, 9])
                        == Some(true)
                {
                    break;
                }

                state_rx.changed().await.unwrap();
            }
        })
        .await
        .unwrap();

        let state = handle.state();
        assert_eq!(
            state.selected_mode,
            VoiceEncryptionMode::aead_xchacha20_poly1305_rtpsize()
        );
        assert_eq!(state.last_sequence, Some(25));
        assert!(state.resumed);
        assert_eq!(
            state.session_description,
            Some(VoiceSessionDescription {
                mode: VoiceEncryptionMode::aead_xchacha20_poly1305_rtpsize(),
                secret_key: vec![9, 9],
                audio_codec: Some("opus".to_string()),
                dave_protocol_version: None,
            })
        );

        handle.close().await.unwrap();
        server.await.unwrap();
    }
}
