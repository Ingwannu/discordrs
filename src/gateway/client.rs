use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;

use futures_util::{SinkExt, StreamExt};
use serde_json::Value;
use tokio::sync::mpsc;
use tokio::time::{sleep, timeout, Duration};
use tokio_tungstenite::{
    connect_async,
    tungstenite::{
        protocol::{frame::coding::CloseCode, CloseFrame},
        Message as WsMessage,
    },
};
use tracing::{debug, error, info, warn};

use crate::model::Snowflake;
#[cfg(feature = "sharding")]
use crate::sharding::{ShardRuntimeState, ShardSupervisorEvent};
use crate::ws::GatewayConnectionConfig;

// Gateway opcodes
const OP_DISPATCH: u64 = 0;
const OP_HEARTBEAT: u64 = 1;
const OP_IDENTIFY: u64 = 2;
const OP_RESUME: u64 = 6;
const OP_RECONNECT: u64 = 7;
const OP_INVALID_SESSION: u64 = 9;
const OP_HELLO: u64 = 10;
const OP_HEARTBEAT_ACK: u64 = 11;

pub(crate) struct GatewayClient {
    token: String,
    intents: u64,
    session_id: Option<String>,
    resume_gateway_url: Option<String>,
    gateway_config: GatewayConnectionConfig,
    shard_info: Option<[u32; 2]>,
    command_rx: Option<mpsc::UnboundedReceiver<GatewayCommand>>,
    #[cfg(feature = "sharding")]
    supervisor_callback: Option<SupervisorCallback>,
    sequence: Arc<AtomicU64>,
    heartbeat_ack_received: Arc<AtomicBool>,
}

// Callback type for dispatching events
pub(crate) type EventCallback = Arc<dyn Fn(String, Value) + Send + Sync>;
#[cfg(feature = "sharding")]
pub(crate) type SupervisorCallback = Arc<dyn Fn(ShardSupervisorEvent) + Send + Sync>;

#[derive(Debug)]
pub(crate) enum GatewayCommand {
    Shutdown,
    Reconnect,
    UpdatePresence(String),
    SendPayload(Value),
}

impl GatewayClient {
    pub fn new(token: String, intents: u64) -> Self {
        Self {
            token,
            intents,
            session_id: None,
            resume_gateway_url: None,
            gateway_config: GatewayConnectionConfig::default(),
            shard_info: None,
            command_rx: None,
            #[cfg(feature = "sharding")]
            supervisor_callback: None,
            sequence: Arc::new(AtomicU64::new(0)),
            heartbeat_ack_received: Arc::new(AtomicBool::new(true)),
        }
    }

    pub fn gateway_config(mut self, gateway_config: GatewayConnectionConfig) -> Self {
        self.gateway_config = gateway_config;
        self
    }

    pub fn shard(mut self, shard_id: u32, total_shards: u32) -> Self {
        self.shard_info = Some([shard_id, total_shards.max(1)]);
        self.gateway_config = self
            .gateway_config
            .clone()
            .shard(shard_id, total_shards.max(1));
        self
    }

    pub fn control(mut self, command_rx: mpsc::UnboundedReceiver<GatewayCommand>) -> Self {
        self.command_rx = Some(command_rx);
        self
    }

    #[cfg(feature = "sharding")]
    pub fn supervisor(mut self, supervisor_callback: SupervisorCallback) -> Self {
        self.supervisor_callback = Some(supervisor_callback);
        self
    }

    /// Run the gateway connection loop. Reconnects automatically.
    /// `on_event` is called for every DISPATCH event with (event_name, data).
    pub async fn run(&mut self, on_event: EventCallback) -> Result<(), crate::error::DiscordError> {
        let mut backoff = 1_u64;
        #[cfg(feature = "sharding")]
        self.publish_state(ShardRuntimeState::Starting);
        loop {
            let url = self
                .resume_gateway_url
                .clone()
                .map(|url| GatewayConnectionConfig::new(url).normalized_url())
                .unwrap_or_else(|| self.gateway_config.normalized_url());
            info!("Connecting to gateway: {url}");

            match self.connect_and_run(&url, on_event.clone()).await {
                Ok(action) => match action {
                    ReconnectAction::Resume => {
                        #[cfg(feature = "sharding")]
                        self.publish_state(ShardRuntimeState::Reconnecting);
                        info!("Resuming gateway session");
                        backoff = 1;
                    }
                    ReconnectAction::Reconnect => {
                        #[cfg(feature = "sharding")]
                        self.publish_state(ShardRuntimeState::Reconnecting);
                        info!("Reconnecting with fresh session");
                        self.session_id = None;
                        self.resume_gateway_url = None;
                        self.sequence.store(0, Ordering::Relaxed);
                        backoff = 1;
                    }
                    ReconnectAction::Shutdown => {
                        #[cfg(feature = "sharding")]
                        self.publish_state(ShardRuntimeState::Stopped);
                        return Ok(());
                    }
                },
                Err(e) => {
                    #[cfg(feature = "sharding")]
                    self.publish_error(e.to_string());
                    error!("Gateway connection error: {e}");
                    if self
                        .wait_for_backoff_command(Duration::from_secs(backoff.min(60)))
                        .await?
                    {
                        #[cfg(feature = "sharding")]
                        self.publish_state(ShardRuntimeState::Stopped);
                        return Ok(());
                    }
                    backoff = (backoff * 2).min(60);
                }
            }
        }
    }

    async fn connect_and_run(
        &mut self,
        url: &str,
        on_event: EventCallback,
    ) -> Result<ReconnectAction, crate::error::DiscordError> {
        let (ws_stream, _) = connect_async(url).await?;
        let (mut write, mut read) = ws_stream.split();

        // Wait for Hello
        let hello = read.next().await.ok_or("gateway closed before Hello")??;
        let hello_payload: Value = serde_json::from_str(hello.to_text()?)?;
        let hello_op = hello_payload["op"].as_u64().unwrap_or(u64::MAX);
        if hello_op != OP_HELLO {
            return Err(format!("expected Hello opcode {OP_HELLO}, got {hello_op}").into());
        }
        let heartbeat_interval_ms = hello_payload["d"]["heartbeat_interval"]
            .as_u64()
            .ok_or("missing heartbeat_interval in Hello")?;

        debug!("Received Hello, heartbeat_interval={heartbeat_interval_ms}ms");

        // Send Identify or Resume
        if let Some(ref session_id) = self.session_id {
            let seq = self.sequence.load(Ordering::Relaxed);
            let resume = resume_payload(&self.token, session_id, seq);
            write
                .send(WsMessage::Text(resume.to_string().into()))
                .await?;
            debug!("Sent Resume");
        } else {
            let identify = identify_payload(&self.token, self.intents, self.shard_info);
            write
                .send(WsMessage::Text(identify.to_string().into()))
                .await?;
            debug!("Sent Identify");
        }

        // Spawn heartbeat task
        let (heartbeat_tx, mut heartbeat_rx) = mpsc::channel::<String>(8);
        let sequence_clone = self.sequence.clone();
        let ack_received = self.heartbeat_ack_received.clone();
        ack_received.store(true, Ordering::Relaxed);

        let heartbeat_handle = tokio::spawn(async move {
            sleep(initial_heartbeat_delay(
                heartbeat_interval_ms,
                rand_jitter(),
            ))
            .await;

            loop {
                if !ack_received.load(Ordering::Relaxed) {
                    warn!("Heartbeat ACK not received - zombie connection");
                    let _ = heartbeat_tx.send("zombie".to_string()).await;
                    break;
                }

                ack_received.store(false, Ordering::Relaxed);
                let seq = sequence_clone.load(Ordering::Relaxed);
                let hb = serde_json::json!({
                    "op": OP_HEARTBEAT,
                    "d": if seq == 0 { Value::Null } else { Value::Number(seq.into()) }
                });
                if heartbeat_tx.send(hb.to_string()).await.is_err() {
                    break;
                }

                sleep(Duration::from_millis(heartbeat_interval_ms)).await;
            }
        });

        // Main read loop
        let action = loop {
            tokio::select! {
                msg = read.next() => {
                    match msg {
                        Some(Ok(WsMessage::Text(text))) => {
                            let payload: Value = match serde_json::from_str(&text) {
                                Ok(v) => v,
                                Err(e) => {
                                    warn!("Failed to parse gateway message: {e}");
                                    continue;
                                }
                            };

                            let op = payload["op"].as_u64().unwrap_or(u64::MAX);

                            match op {
                                OP_DISPATCH => {
                                    if let Some(s) = payload["s"].as_u64() {
                                        self.sequence.store(s, Ordering::Relaxed);
                                    }
                                    let event_name = payload["t"].as_str().unwrap_or("").to_string();
                                    let data = payload["d"].clone();

                                    // Update session info from READY
                                    if event_name == "READY" {
                                        if let Some(sid) = data["session_id"].as_str() {
                                            self.session_id = Some(sid.to_string());
                                        }
                                        if let Some(resume_url) = data["resume_gateway_url"].as_str() {
                                            self.resume_gateway_url = Some(
                                                GatewayConnectionConfig::new(resume_url).normalized_url(),
                                            );
                                        }
                                        #[cfg(feature = "sharding")]
                                        if let Some(session_id) = self.session_id.clone() {
                                            self.publish_supervisor(ShardSupervisorEvent::SessionEstablished {
                                                shard_id: self.current_shard_id(),
                                                session_id,
                                            });
                                            self.publish_state(ShardRuntimeState::Running);
                                        }
                                        info!("Received READY, session_id={}", self.session_id.as_deref().unwrap_or("?"));
                                    }

                                    on_event(event_name, data);
                                }
                                OP_HEARTBEAT => {
                                    let seq = self.sequence.load(Ordering::Relaxed);
                                    let hb = serde_json::json!({
                                        "op": OP_HEARTBEAT,
                                        "d": if seq == 0 { Value::Null } else { Value::Number(seq.into()) }
                                    });
                                    write.send(WsMessage::Text(hb.to_string().into())).await?;
                                }
                                OP_HEARTBEAT_ACK => {
                                    self.heartbeat_ack_received.store(true, Ordering::Relaxed);
                                    debug!("Heartbeat ACK received");
                                }
                                OP_RECONNECT => {
                                    #[cfg(feature = "sharding")]
                                    self.publish_state(ShardRuntimeState::Reconnecting);
                                    info!("Received Reconnect opcode");
                                    break ReconnectAction::Resume;
                                }
                                OP_INVALID_SESSION => {
                                    let resumable = payload["d"].as_bool().unwrap_or(false);
                                    #[cfg(feature = "sharding")]
                                    self.publish_state(ShardRuntimeState::Reconnecting);
                                    warn!("Invalid session, resumable={resumable}");
                                    sleep(Duration::from_secs(2)).await;
                                    if resumable {
                                        break ReconnectAction::Resume;
                                    } else {
                                        break ReconnectAction::Reconnect;
                                    }
                                }
                                _ => {
                                    debug!("Unhandled gateway opcode: {op}");
                                }
                            }
                        }
                        Some(Ok(WsMessage::Close(frame))) => {
                            #[cfg(feature = "sharding")]
                            self.publish_error(terminal_close_error(frame.clone()));
                            warn!("Gateway closed: {frame:?}");
                            if is_terminal_close_frame(frame.as_ref()) {
                                return Err(terminal_close_error(frame).into());
                            }
                            break ReconnectAction::Resume;
                        }
                        Some(Err(e)) => {
                            #[cfg(feature = "sharding")]
                            self.publish_error(e.to_string());
                            error!("Gateway read error: {e}");
                            break ReconnectAction::Resume;
                        }
                        None => {
                            #[cfg(feature = "sharding")]
                            self.publish_error("gateway stream ended".to_string());
                            warn!("Gateway stream ended");
                            break ReconnectAction::Resume;
                        }
                        _ => {}
                    }
                }
                Some(msg) = heartbeat_rx.recv() => {
                    if msg == "zombie" {
                        #[cfg(feature = "sharding")]
                        self.publish_error("heartbeat zombie connection detected".to_string());
                        warn!("Zombie connection detected, reconnecting");
                        break ReconnectAction::Resume;
                    }
                    write.send(WsMessage::Text(msg.into())).await?;
                    debug!("Sent heartbeat");
                }
                command = recv_control_command(&mut self.command_rx) => {
                    match command {
                        Some(GatewayCommand::Shutdown) => {
                            let _ = write
                                .send(WsMessage::Close(Some(CloseFrame {
                                    code: CloseCode::Normal,
                                    reason: "supervisor shutdown".into(),
                                })))
                                .await;
                            break ReconnectAction::Shutdown;
                        }
                        Some(GatewayCommand::Reconnect) => break ReconnectAction::Resume,
                        Some(GatewayCommand::UpdatePresence(status)) => {
                            let payload = serde_json::json!({
                                "op": 3,
                                "d": {
                                    "since": Value::Null,
                                    "activities": [{
                                        "name": status,
                                        "type": 0
                                    }],
                                    "status": "online",
                                    "afk": false
                                }
                            });
                            write.send(WsMessage::Text(payload.to_string().into())).await?;
                        }
                        Some(GatewayCommand::SendPayload(payload)) => {
                            write.send(WsMessage::Text(payload.to_string().into())).await?;
                        }
                        None => {}
                    }
                }
            }
        };

        heartbeat_handle.abort();
        Ok(action)
    }
}

enum ReconnectAction {
    Resume,
    Reconnect,
    Shutdown,
}

async fn recv_control_command(
    command_rx: &mut Option<mpsc::UnboundedReceiver<GatewayCommand>>,
) -> Option<GatewayCommand> {
    match command_rx {
        Some(command_rx) => command_rx.recv().await,
        None => std::future::pending::<Option<GatewayCommand>>().await,
    }
}

pub(crate) fn voice_state_update_payload(
    guild_id: Snowflake,
    channel_id: Option<Snowflake>,
    self_mute: bool,
    self_deaf: bool,
) -> Value {
    serde_json::json!({
        "op": 4,
        "d": {
            "guild_id": guild_id,
            "channel_id": channel_id,
            "self_mute": self_mute,
            "self_deaf": self_deaf
        }
    })
}

fn rand_jitter() -> f64 {
    // Simple jitter: use current time nanos as pseudo-random
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .subsec_nanos();
    (nanos as f64 % 1000.0) / 1000.0
}

fn initial_heartbeat_delay(heartbeat_interval_ms: u64, jitter_factor: f64) -> Duration {
    let clamped = jitter_factor.clamp(0.0, 1.0);
    Duration::from_millis((heartbeat_interval_ms as f64 * clamped) as u64)
}

fn resume_payload(token: &str, session_id: &str, seq: u64) -> Value {
    serde_json::json!({
        "op": OP_RESUME,
        "d": {
            "token": token,
            "session_id": session_id,
            "seq": if seq == 0 { Value::Null } else { Value::Number(seq.into()) }
        }
    })
}

fn identify_payload(token: &str, intents: u64, shard_info: Option<[u32; 2]>) -> Value {
    let mut identify = serde_json::json!({
        "op": OP_IDENTIFY,
        "d": {
            "token": token,
            "intents": intents,
            "properties": {
                "os": std::env::consts::OS,
                "browser": "discordrs",
                "device": "discordrs"
            }
        }
    });
    if let Some(shard_info) = shard_info {
        identify["d"]["shard"] = serde_json::json!(shard_info);
    }
    identify
}

fn is_terminal_close_frame(frame: Option<&CloseFrame>) -> bool {
    frame
        .map(|frame| is_terminal_close_code(u16::from(frame.code)))
        .unwrap_or(false)
}

fn is_terminal_close_code(code: u16) -> bool {
    matches!(code, 4004 | 4010 | 4011 | 4012 | 4013 | 4014)
}

fn terminal_close_error(frame: Option<CloseFrame>) -> String {
    match frame {
        Some(frame) => format!(
            "gateway closed with terminal close code {}: {}",
            u16::from(frame.code),
            frame.reason
        ),
        None => "gateway closed with terminal close code".to_string(),
    }
}

#[cfg(feature = "sharding")]
impl GatewayClient {
    fn current_shard_id(&self) -> u32 {
        self.shard_info.map(|pair| pair[0]).unwrap_or(0)
    }

    fn publish_state(&self, state: ShardRuntimeState) {
        self.publish_supervisor(ShardSupervisorEvent::StateChanged {
            shard_id: self.current_shard_id(),
            state,
        });
    }

    fn publish_error(&self, message: String) {
        self.publish_supervisor(ShardSupervisorEvent::GatewayError {
            shard_id: self.current_shard_id(),
            message,
        });
    }

    fn publish_supervisor(&self, event: ShardSupervisorEvent) {
        if let Some(callback) = &self.supervisor_callback {
            callback(event);
        }
    }
}

impl GatewayClient {
    async fn wait_for_backoff_command(
        &mut self,
        duration: Duration,
    ) -> Result<bool, crate::error::DiscordError> {
        let Some(command_rx) = self.command_rx.as_mut() else {
            sleep(duration).await;
            return Ok(false);
        };

        match timeout(duration, command_rx.recv()).await {
            Ok(Some(GatewayCommand::Shutdown)) => Ok(true),
            Ok(Some(GatewayCommand::Reconnect)) => {
                self.session_id = None;
                self.resume_gateway_url = None;
                self.sequence.store(0, Ordering::Relaxed);
                Ok(false)
            }
            Ok(Some(_)) => Ok(false),
            Ok(None) | Err(_) => Ok(false),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};
    use std::time::Duration;

    use super::{
        identify_payload, initial_heartbeat_delay, is_terminal_close_code, is_terminal_close_frame,
        recv_control_command, resume_payload, terminal_close_error, voice_state_update_payload,
        EventCallback, GatewayClient, GatewayCommand, ReconnectAction,
    };
    use crate::model::Snowflake;
    #[cfg(feature = "sharding")]
    use crate::sharding::{ShardRuntimeState, ShardSupervisorEvent};
    use crate::ws::GatewayConnectionConfig;
    use futures_util::{SinkExt, StreamExt};
    use tokio::net::TcpListener;
    use tokio::sync::mpsc;
    use tokio_tungstenite::tungstenite::protocol::{frame::coding::CloseCode, CloseFrame};
    use tokio_tungstenite::{accept_async, tungstenite::Message as WsMessage};

    #[test]
    fn normalize_gateway_url_adds_missing_gateway_query() {
        assert_eq!(
            GatewayConnectionConfig::new("gateway.discord.gg").normalized_url(),
            "wss://gateway.discord.gg?v=10&encoding=json"
        );
    }

    #[test]
    fn normalize_gateway_url_preserves_existing_query_values() {
        assert_eq!(
            GatewayConnectionConfig::new("wss://gateway.discord.gg/?encoding=json")
                .normalized_url(),
            "wss://gateway.discord.gg/?encoding=json&v=10"
        );
    }

    #[test]
    fn terminal_close_codes_match_discord_non_reconnectable_codes() {
        for code in [4004_u16, 4010, 4011, 4012, 4013, 4014] {
            assert!(is_terminal_close_code(code));
        }

        for code in [4000_u16, 4007, 4009] {
            assert!(!is_terminal_close_code(code));
        }
    }

    #[test]
    fn voice_state_update_payload_matches_gateway_shape() {
        let payload = voice_state_update_payload(
            Snowflake::from("1"),
            Some(Snowflake::from("2")),
            false,
            true,
        );

        assert_eq!(payload["op"], serde_json::json!(4));
        assert_eq!(payload["d"]["guild_id"], serde_json::json!("1"));
        assert_eq!(payload["d"]["channel_id"], serde_json::json!("2"));
        assert_eq!(payload["d"]["self_mute"], serde_json::json!(false));
        assert_eq!(payload["d"]["self_deaf"], serde_json::json!(true));
    }

    #[test]
    fn initial_heartbeat_delay_uses_only_jitter_fraction() {
        assert_eq!(
            initial_heartbeat_delay(1_000, 0.0),
            Duration::from_millis(0)
        );
        assert_eq!(
            initial_heartbeat_delay(1_000, 0.25),
            Duration::from_millis(250)
        );
        assert_eq!(
            initial_heartbeat_delay(1_000, 1.5),
            Duration::from_millis(1_000)
        );
    }

    #[test]
    fn identify_and_resume_payloads_use_raw_gateway_token() {
        let identify = identify_payload("secret-token", 513, Some([2, 4]));
        let resume = resume_payload("secret-token", "session", 42);

        assert_eq!(identify["d"]["token"], serde_json::json!("secret-token"));
        assert_eq!(resume["d"]["token"], serde_json::json!("secret-token"));
    }

    #[test]
    fn identify_without_shard_and_resume_without_sequence_keep_expected_shape() {
        let identify = identify_payload("secret-token", 513, None);
        let resume = resume_payload("secret-token", "session", 0);

        assert!(identify["d"].get("shard").is_none());
        assert_eq!(identify["d"]["intents"], serde_json::json!(513));
        assert_eq!(resume["d"]["session_id"], serde_json::json!("session"));
        assert!(resume["d"]["seq"].is_null());
    }

    #[test]
    fn shard_clamps_total_shards_and_updates_gateway_config() {
        let client = GatewayClient::new("secret-token".into(), 513).shard(2, 0);

        assert_eq!(client.shard_info, Some([2, 1]));
        assert_eq!(
            client.gateway_config.normalized_url(),
            "wss://gateway.discord.gg/?v=10&encoding=json&shard=2,1"
        );
    }

    #[test]
    fn terminal_close_helpers_cover_frame_and_none_cases() {
        let frame = CloseFrame {
            code: CloseCode::from(4004),
            reason: "bad auth".into(),
        };

        assert!(is_terminal_close_frame(Some(&frame)));
        assert_eq!(
            terminal_close_error(Some(frame.clone())),
            "gateway closed with terminal close code 4004: bad auth"
        );
        assert!(!is_terminal_close_frame(None));
        assert_eq!(
            terminal_close_error(None),
            "gateway closed with terminal close code"
        );
    }

    #[tokio::test]
    async fn recv_control_command_and_wait_for_backoff_command_handle_control_flow() {
        let (shutdown_tx, shutdown_rx) = mpsc::unbounded_channel();
        let mut shutdown_client =
            GatewayClient::new("secret-token".into(), 513).control(shutdown_rx);
        shutdown_tx.send(GatewayCommand::Shutdown).unwrap();
        assert!(shutdown_client
            .wait_for_backoff_command(Duration::from_millis(10))
            .await
            .unwrap());

        let (reconnect_tx, reconnect_rx) = mpsc::unbounded_channel();
        let mut reconnect_client =
            GatewayClient::new("secret-token".into(), 513).control(reconnect_rx);
        reconnect_client.session_id = Some("session".into());
        reconnect_client.resume_gateway_url = Some("wss://gateway.discord.gg".into());
        reconnect_client
            .sequence
            .store(42, std::sync::atomic::Ordering::Relaxed);
        reconnect_tx.send(GatewayCommand::Reconnect).unwrap();
        assert!(!reconnect_client
            .wait_for_backoff_command(Duration::from_millis(10))
            .await
            .unwrap());
        assert!(reconnect_client.session_id.is_none());
        assert!(reconnect_client.resume_gateway_url.is_none());
        assert_eq!(
            reconnect_client
                .sequence
                .load(std::sync::atomic::Ordering::Relaxed),
            0
        );

        let (presence_tx, presence_rx) = mpsc::unbounded_channel();
        let mut presence_client =
            GatewayClient::new("secret-token".into(), 513).control(presence_rx);
        presence_tx
            .send(GatewayCommand::UpdatePresence("busy".into()))
            .unwrap();
        assert!(!presence_client
            .wait_for_backoff_command(Duration::from_millis(10))
            .await
            .unwrap());

        let mut no_control_client = GatewayClient::new("secret-token".into(), 513);
        assert!(!no_control_client
            .wait_for_backoff_command(Duration::from_millis(1))
            .await
            .unwrap());
    }

    #[tokio::test]
    async fn recv_control_command_reads_payloads_and_handles_missing_channel() {
        let (tx, rx) = mpsc::unbounded_channel();
        let mut command_rx = Some(rx);

        tx.send(GatewayCommand::SendPayload(serde_json::json!({ "op": 4 })))
            .unwrap();
        match recv_control_command(&mut command_rx).await {
            Some(GatewayCommand::SendPayload(payload)) => {
                assert_eq!(payload["op"], serde_json::json!(4));
            }
            other => panic!("unexpected control command: {other:?}"),
        }

        let mut none_rx = None;
        let pending =
            tokio::time::timeout(Duration::from_millis(5), recv_control_command(&mut none_rx))
                .await;
        assert!(pending.is_err());
    }

    #[tokio::test]
    async fn connect_and_run_identifies_processes_ready_and_shuts_down() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let (command_tx, command_rx) = mpsc::unbounded_channel();
        let events = Arc::new(Mutex::new(Vec::<(String, serde_json::Value)>::new()));
        let events_for_callback = Arc::clone(&events);
        let server = tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            let mut ws = accept_async(stream).await.unwrap();

            ws.send(WsMessage::Text(
                serde_json::json!({
                    "op": 10,
                    "d": { "heartbeat_interval": 60_000 }
                })
                .to_string()
                .into(),
            ))
            .await
            .unwrap();

            let identify_payload: serde_json::Value =
                serde_json::from_str(&ws.next().await.unwrap().unwrap().into_text().unwrap())
                    .unwrap();

            ws.send(WsMessage::Text(
                serde_json::json!({
                    "op": 0,
                    "t": "READY",
                    "s": 7,
                    "d": {
                        "user": {
                            "id": "1",
                            "username": "discordrs"
                        },
                        "session_id": "session-1",
                        "resume_gateway_url": "wss://gateway.discord.gg"
                    }
                })
                .to_string()
                .into(),
            ))
            .await
            .unwrap();

            let _ = ws.next().await;

            identify_payload
        });

        let shutdown = tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(30)).await;
            command_tx.send(GatewayCommand::Shutdown).unwrap();
        });

        let mut client = GatewayClient::new("secret-token".into(), 513).control(command_rx);
        let on_event: EventCallback = Arc::new(move |name, data| {
            events_for_callback.lock().unwrap().push((name, data));
        });
        let action = client
            .connect_and_run(&format!("ws://{address}"), on_event)
            .await
            .unwrap();

        shutdown.await.unwrap();
        let identify = server.await.unwrap();

        assert!(matches!(
            action,
            ReconnectAction::Shutdown | ReconnectAction::Resume
        ));
        assert_eq!(identify["op"], serde_json::json!(2));
        assert_eq!(identify["d"]["token"], serde_json::json!("secret-token"));
        assert_eq!(client.session_id.as_deref(), Some("session-1"));
        assert_eq!(
            client.resume_gateway_url.as_deref(),
            Some("wss://gateway.discord.gg?v=10&encoding=json")
        );
        assert_eq!(
            client.sequence.load(std::sync::atomic::Ordering::Relaxed),
            7
        );
        assert_eq!(events.lock().unwrap()[0].0, "READY");
    }

    #[tokio::test]
    async fn connect_and_run_resumes_existing_session_and_handles_invalid_session() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            let mut ws = accept_async(stream).await.unwrap();

            ws.send(WsMessage::Text(
                serde_json::json!({
                    "op": 10,
                    "d": { "heartbeat_interval": 60_000 }
                })
                .to_string()
                .into(),
            ))
            .await
            .unwrap();

            let resume_payload: serde_json::Value =
                serde_json::from_str(&ws.next().await.unwrap().unwrap().into_text().unwrap())
                    .unwrap();

            ws.send(WsMessage::Text(
                serde_json::json!({
                    "op": 9,
                    "d": false
                })
                .to_string()
                .into(),
            ))
            .await
            .unwrap();

            resume_payload
        });

        let mut client = GatewayClient::new("secret-token".into(), 513);
        client.session_id = Some("session-2".into());
        client
            .sequence
            .store(42, std::sync::atomic::Ordering::Relaxed);
        let action = client
            .connect_and_run(&format!("ws://{address}"), Arc::new(|_, _| {}))
            .await
            .unwrap();

        let resume = server.await.unwrap();

        assert!(matches!(action, ReconnectAction::Reconnect));
        assert_eq!(resume["op"], serde_json::json!(6));
        assert_eq!(resume["d"]["token"], serde_json::json!("secret-token"));
        assert_eq!(resume["d"]["session_id"], serde_json::json!("session-2"));
        assert_eq!(resume["d"]["seq"], serde_json::json!(42));
    }

    #[tokio::test]
    async fn connect_and_run_skips_malformed_messages_and_honors_reconnect_opcode() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            let mut ws = accept_async(stream).await.unwrap();

            ws.send(WsMessage::Text(
                serde_json::json!({
                    "op": 10,
                    "d": { "heartbeat_interval": 60_000 }
                })
                .to_string()
                .into(),
            ))
            .await
            .unwrap();

            let identify_payload: serde_json::Value =
                serde_json::from_str(&ws.next().await.unwrap().unwrap().into_text().unwrap())
                    .unwrap();

            ws.send(WsMessage::Text("not-json".into())).await.unwrap();
            ws.send(WsMessage::Text(
                serde_json::json!({
                    "op": 7,
                    "d": null
                })
                .to_string()
                .into(),
            ))
            .await
            .unwrap();

            identify_payload
        });

        let mut client = GatewayClient::new("secret-token".into(), 513)
            .gateway_config(GatewayConnectionConfig::new(format!("ws://{address}")));
        let action = client
            .connect_and_run(&format!("ws://{address}"), Arc::new(|_, _| {}))
            .await
            .unwrap();

        let identify = server.await.unwrap();

        assert!(matches!(action, ReconnectAction::Resume));
        assert_eq!(identify["op"], serde_json::json!(2));
    }

    #[tokio::test]
    async fn connect_and_run_replies_to_server_heartbeat_requests() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let (command_tx, command_rx) = mpsc::unbounded_channel();
        let server = tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            let mut ws = accept_async(stream).await.unwrap();

            ws.send(WsMessage::Text(
                serde_json::json!({
                    "op": 10,
                    "d": { "heartbeat_interval": 60_000 }
                })
                .to_string()
                .into(),
            ))
            .await
            .unwrap();

            let identify_payload: serde_json::Value =
                serde_json::from_str(&ws.next().await.unwrap().unwrap().into_text().unwrap())
                    .unwrap();

            ws.send(WsMessage::Text(
                serde_json::json!({
                    "op": 1,
                    "d": null
                })
                .to_string()
                .into(),
            ))
            .await
            .unwrap();

            let heartbeat_payload: serde_json::Value =
                serde_json::from_str(&ws.next().await.unwrap().unwrap().into_text().unwrap())
                    .unwrap();
            let _ = ws.next().await;

            (identify_payload, heartbeat_payload)
        });

        let shutdown = tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(30)).await;
            command_tx.send(GatewayCommand::Shutdown).unwrap();
        });

        let mut client = GatewayClient::new("secret-token".into(), 513).control(command_rx);
        let action = client
            .connect_and_run(&format!("ws://{address}"), Arc::new(|_, _| {}))
            .await
            .unwrap();

        shutdown.await.unwrap();
        let (identify, heartbeat) = server.await.unwrap();

        assert!(matches!(
            action,
            ReconnectAction::Shutdown | ReconnectAction::Resume
        ));
        assert_eq!(identify["op"], serde_json::json!(2));
        assert_eq!(heartbeat["op"], serde_json::json!(1));
        assert!(heartbeat["d"].is_null());
    }

    #[tokio::test]
    async fn run_reconnects_after_invalid_session_and_then_shuts_down_cleanly() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let (command_tx, command_rx) = mpsc::unbounded_channel();
        let payloads = Arc::new(Mutex::new(Vec::<serde_json::Value>::new()));
        let payloads_for_server = Arc::clone(&payloads);

        let server = tokio::spawn(async move {
            for iteration in 0..2 {
                let (stream, _) = listener.accept().await.unwrap();
                let mut ws = accept_async(stream).await.unwrap();

                ws.send(WsMessage::Text(
                    serde_json::json!({
                        "op": 10,
                        "d": { "heartbeat_interval": 60_000 }
                    })
                    .to_string()
                    .into(),
                ))
                .await
                .unwrap();

                let payload: serde_json::Value =
                    serde_json::from_str(&ws.next().await.unwrap().unwrap().into_text().unwrap())
                        .unwrap();
                payloads_for_server.lock().unwrap().push(payload);

                if iteration == 0 {
                    ws.send(WsMessage::Text(
                        serde_json::json!({
                            "op": 9,
                            "d": false
                        })
                        .to_string()
                        .into(),
                    ))
                    .await
                    .unwrap();
                } else {
                    let close = ws.next().await;
                    assert!(matches!(close, Some(Ok(WsMessage::Close(_))) | None));
                }
            }
        });

        let shutdown = tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(80)).await;
            command_tx.send(GatewayCommand::Shutdown).unwrap();
        });

        let mut client = GatewayClient::new("secret-token".into(), 513)
            .gateway_config(GatewayConnectionConfig::new(format!("ws://{address}/")))
            .control(command_rx);
        client.session_id = Some("session-2".into());
        client.resume_gateway_url = Some(format!("ws://{address}/"));
        client
            .sequence
            .store(42, std::sync::atomic::Ordering::Relaxed);

        client.run(Arc::new(|_, _| {})).await.unwrap();

        shutdown.await.unwrap();
        server.await.unwrap();

        let payloads = payloads.lock().unwrap();
        assert_eq!(payloads.len(), 2);
        assert_eq!(payloads[0]["op"], serde_json::json!(6));
        assert_eq!(payloads[1]["op"], serde_json::json!(2));
        assert!(client.session_id.is_none());
        assert!(client.resume_gateway_url.is_none());
        assert_eq!(
            client.sequence.load(std::sync::atomic::Ordering::Relaxed),
            0
        );
    }

    #[cfg(feature = "sharding")]
    #[test]
    fn supervisor_callback_records_state_and_error_events() {
        let seen = Arc::new(Mutex::new(Vec::<ShardSupervisorEvent>::new()));
        let seen_for_callback = Arc::clone(&seen);
        let callback = Arc::new(move |event| {
            seen_for_callback.lock().unwrap().push(event);
        });

        let client = GatewayClient::new("secret-token".into(), 513)
            .shard(3, 5)
            .supervisor(callback);

        client.publish_state(ShardRuntimeState::Running);
        client.publish_error("boom".to_string());

        let seen = seen.lock().unwrap();
        assert_eq!(
            seen[0],
            ShardSupervisorEvent::StateChanged {
                shard_id: 3,
                state: ShardRuntimeState::Running,
            }
        );
        assert_eq!(
            seen[1],
            ShardSupervisorEvent::GatewayError {
                shard_id: 3,
                message: "boom".to_string(),
            }
        );
    }
}
