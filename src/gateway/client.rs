use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;

use futures_util::{SinkExt, StreamExt};
use serde_json::Value;
use tokio::sync::mpsc;
use tokio::time::{sleep, Duration, interval};
use tokio_tungstenite::{connect_async, tungstenite::Message as WsMessage};
use tracing::{info, warn, error, debug};

const GATEWAY_URL: &str = "wss://gateway.discord.gg/?v=10&encoding=json";

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
    sequence: Arc<AtomicU64>,
    heartbeat_ack_received: Arc<AtomicBool>,
}

// Callback type for dispatching events
pub(crate) type EventCallback = Arc<dyn Fn(String, Value) + Send + Sync>;

impl GatewayClient {
    pub fn new(token: String, intents: u64) -> Self {
        Self {
            token,
            intents,
            session_id: None,
            resume_gateway_url: None,
            sequence: Arc::new(AtomicU64::new(0)),
            heartbeat_ack_received: Arc::new(AtomicBool::new(true)),
        }
    }

    /// Run the gateway connection loop. Reconnects automatically.
    /// `on_event` is called for every DISPATCH event with (event_name, data).
    pub async fn run(&mut self, on_event: EventCallback) -> Result<(), crate::types::Error> {
        let mut backoff = 1_u64;
        loop {
            let url = self
                .resume_gateway_url
                .clone()
                .unwrap_or_else(|| GATEWAY_URL.to_string());
            info!("Connecting to gateway: {url}");

            match self.connect_and_run(&url, on_event.clone()).await {
                Ok(action) => match action {
                    ReconnectAction::Resume => {
                        info!("Resuming gateway session");
                        backoff = 1;
                    }
                    ReconnectAction::Reconnect => {
                        info!("Reconnecting with fresh session");
                        self.session_id = None;
                        self.resume_gateway_url = None;
                        self.sequence.store(0, Ordering::Relaxed);
                        backoff = 1;
                    }
                },
                Err(e) => {
                    error!("Gateway connection error: {e}");
                    sleep(Duration::from_secs(backoff.min(60))).await;
                    backoff = (backoff * 2).min(60);
                }
            }
        }
    }

    async fn connect_and_run(&mut self, url: &str, on_event: EventCallback) -> Result<ReconnectAction, crate::types::Error> {
        let (ws_stream, _) = connect_async(url).await?;
        let (mut write, mut read) = ws_stream.split();

        // Wait for Hello
        let hello = read.next().await
            .ok_or("gateway closed before Hello")??;
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
            let resume = serde_json::json!({
                "op": OP_RESUME,
                "d": {
                    "token": format!("Bot {}", self.token),
                    "session_id": session_id,
                    "seq": if seq == 0 { Value::Null } else { Value::Number(seq.into()) }
                }
            });
            write.send(WsMessage::Text(resume.to_string().into())).await?;
            debug!("Sent Resume");
        } else {
            let identify = serde_json::json!({
                "op": OP_IDENTIFY,
                "d": {
                    "token": format!("Bot {}", self.token),
                    "intents": self.intents,
                    "properties": {
                        "os": std::env::consts::OS,
                        "browser": "discordrs",
                        "device": "discordrs"
                    }
                }
            });
            write.send(WsMessage::Text(identify.to_string().into())).await?;
            debug!("Sent Identify");
        }

        // Spawn heartbeat task
        let (heartbeat_tx, mut heartbeat_rx) = mpsc::channel::<String>(8);
        let sequence_clone = self.sequence.clone();
        let ack_received = self.heartbeat_ack_received.clone();
        ack_received.store(true, Ordering::Relaxed);

        let heartbeat_handle = tokio::spawn(async move {
            // Jitter first heartbeat
            let jitter = heartbeat_interval_ms as f64 * rand_jitter();
            sleep(Duration::from_millis(jitter as u64)).await;

            let mut ticker = interval(Duration::from_millis(heartbeat_interval_ms));
            ticker.tick().await;

            loop {
                ticker.tick().await;

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
                                            self.resume_gateway_url = Some(resume_url.to_string());
                                        }
                                        info!("Received READY, session_id={}", self.session_id.as_deref().unwrap_or("?"));
                                    }

                                    on_event(event_name, data);
                                }
                                OP_HEARTBEAT => {
                                    // Discord requests immediate heartbeat
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
                                    info!("Received Reconnect opcode");
                                    break ReconnectAction::Resume;
                                }
                                OP_INVALID_SESSION => {
                                    let resumable = payload["d"].as_bool().unwrap_or(false);
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
                            warn!("Gateway closed: {frame:?}");
                            break ReconnectAction::Resume;
                        }
                        Some(Err(e)) => {
                            error!("Gateway read error: {e}");
                            break ReconnectAction::Resume;
                        }
                        None => {
                            warn!("Gateway stream ended");
                            break ReconnectAction::Resume;
                        }
                        _ => {}
                    }
                }
                Some(msg) = heartbeat_rx.recv() => {
                    if msg == "zombie" {
                        warn!("Zombie connection detected, reconnecting");
                        break ReconnectAction::Resume;
                    }
                    write.send(WsMessage::Text(msg.into())).await?;
                    debug!("Sent heartbeat");
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
}

fn rand_jitter() -> f64 {
    // Simple jitter: use current time nanos as pseudo-random
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .subsec_nanos();
    (nanos as f64 % 1000.0) / 1000.0
}
