use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use serde_json::Value;
use tokio::sync::RwLock;
use tracing::info;

use crate::http::DiscordHttpClient;
use crate::types::Error;

use super::client::{EventCallback, GatewayClient};

/// Simple type-safe map for storing shared state
pub struct TypeMap(HashMap<TypeId, Box<dyn Any + Send + Sync>>);

impl TypeMap {
    pub fn new() -> Self {
        Self(HashMap::new())
    }

    pub fn insert<T: Send + Sync + 'static>(&mut self, val: T) {
        self.0.insert(TypeId::of::<T>(), Box::new(val));
    }

    pub fn get<T: Send + Sync + 'static>(&self) -> Option<&T> {
        self.0.get(&TypeId::of::<T>()).and_then(|b| b.downcast_ref())
    }

    pub fn get_mut<T: Send + Sync + 'static>(&mut self) -> Option<&mut T> {
        self.0.get_mut(&TypeId::of::<T>()).and_then(|b| b.downcast_mut())
    }
}

impl Default for TypeMap {
    fn default() -> Self {
        Self::new()
    }
}

/// Shared context passed to event handlers
pub struct Context {
    pub http: Arc<DiscordHttpClient>,
    pub data: Arc<RwLock<TypeMap>>,
}

impl Clone for Context {
    fn clone(&self) -> Self {
        Self {
            http: Arc::clone(&self.http),
            data: Arc::clone(&self.data),
        }
    }
}

/// Trait for handling Discord gateway events
#[async_trait]
pub trait EventHandler: Send + Sync + 'static {
    /// Called when the bot receives READY event
    async fn ready(&self, _ctx: Context, _ready_data: Value) {}
    /// Called on INTERACTION_CREATE - raw JSON, use parse_raw_interaction() to route
    async fn interaction_create(&self, _ctx: Context, _interaction: Value) {}
    /// Called on MESSAGE_CREATE
    async fn message_create(&self, _ctx: Context, _message: Value) {}
    /// Called on any dispatch event not specifically handled above
    async fn raw_event(&self, _ctx: Context, _event_name: String, _data: Value) {}
}

pub struct BotClientBuilder {
    token: String,
    intents: u64,
    handler: Option<Arc<dyn EventHandler>>,
    data: TypeMap,
    application_id: Option<u64>,
}

impl BotClientBuilder {
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

    /// Connect to Discord gateway and start the event loop. Blocks until error.
    pub async fn start(self) -> Result<(), Error> {
        let handler = self.handler.ok_or("event_handler is required")?;
        let application_id = self.application_id.unwrap_or(0);

        let http = Arc::new(DiscordHttpClient::new(&self.token, application_id));
        let data = Arc::new(RwLock::new(self.data));

        let ctx = Context {
            http: Arc::clone(&http),
            data: Arc::clone(&data),
        };

        let mut gateway = GatewayClient::new(self.token.clone(), self.intents);

        let handler_clone = handler;
        let http_for_app_id = Arc::clone(&http);

        let callback: EventCallback = Arc::new(move |event_name: String, data: Value| {
            let handler = Arc::clone(&handler_clone);
            let ctx = ctx.clone();
            let http_ref = Arc::clone(&http_for_app_id);

            tokio::spawn(async move {
                match event_name.as_str() {
                    "READY" => {
                        // Extract application_id from READY if not set
                        if http_ref.application_id() == 0 {
                            if let Some(app_id) = data
                                .pointer("/application/id")
                                .and_then(|v| v.as_str())
                                .and_then(|s| s.parse::<u64>().ok())
                            {
                                http_ref.set_application_id(app_id);
                                info!("Set application_id from READY: {app_id}");
                            }
                        }
                        handler.ready(ctx, data).await;
                    }
                    "INTERACTION_CREATE" => {
                        handler.interaction_create(ctx, data).await;
                    }
                    "MESSAGE_CREATE" => {
                        handler.message_create(ctx, data).await;
                    }
                    _ => {
                        handler.raw_event(ctx, event_name, data).await;
                    }
                }
            });
        });

        gateway.run(callback).await
    }
}

/// Discord bot client with gateway connection
pub struct BotClient;

impl BotClient {
    /// Create a new bot client builder
    pub fn builder(token: impl Into<String>, intents: u64) -> BotClientBuilder {
        BotClientBuilder {
            token: token.into(),
            intents,
            handler: None,
            data: TypeMap::new(),
            application_id: None,
        }
    }
}
