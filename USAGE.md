# discord.rs Usage

`discord.rs` is a standalone Rust Discord framework with typed models, typed gateway events, command builders, Components V2 builders, REST helpers, cache managers, collectors, sharding control, and voice runtime foundations.

Brand name: discord.rs. The crates.io package name and Rust import path remain `discordrs`.

## 1. Installation

Pick features based on the runtime surface you want to ship.

```toml
[dependencies]
# Core default: models, builders, parsers, helpers, REST client, cache storage
discordrs = "1.2.1"

# Gateway runtime
discordrs = { version = "1.2.1", features = ["gateway"] }

# HTTP interactions endpoint
discordrs = { version = "1.2.1", features = ["interactions"] }

# Minimal core without cache storage
discordrs = { version = "1.2.1", default-features = false }

# Gateway runtime with collectors
discordrs = { version = "1.2.1", features = ["gateway", "collectors"] }

# Gateway runtime with shard supervisor and shard status APIs
discordrs = { version = "1.2.1", features = ["gateway", "sharding"] }

# Voice manager plus voice gateway/UDP runtime
discordrs = { version = "1.2.1", features = ["voice"] }

# PCM source/mixer plus Opus encoder playback
discordrs = { version = "1.2.1", features = ["voice", "voice-encode"] }

# Experimental DAVE/MLS receive and outbound media hooks
discordrs = { version = "1.2.1", features = ["voice", "dave"] }

# Gateway runtime with voice helpers
discordrs = { version = "1.2.1", features = ["gateway", "voice"] }

# Gateway runtime with zstd-stream transport compression
discordrs = { version = "1.2.1", features = ["gateway", "zstd-stream"] }
```

If you want the common runtime helpers in one import, prefer:

```rust
use discordrs::prelude::*;
```

## 1.5 Migration Notes

The public API was tightened to make the typed surface the default:

- `RestClient` no longer exposes the old raw convenience methods such as `send_message`, `edit_message`, `create_dm_channel`, `create_interaction_response`, and `bulk_overwrite_global_commands`.
- Builder implementation submodules are private. Import from `discordrs::builders::{...}` or use the crate root re-exports.
- `ApplicationCommand` no longer implements `DiscordModel`; use `id_opt()` and `created_at()` directly on the command value.

Common replacements:

| Old path | New path |
|----------|----------|
| `RestClient::send_message(...)` | `send_message(...)` helper or `RestClient::create_message(...)` |
| `RestClient::edit_message(...)` | `RestClient::update_message(...)` |
| `RestClient::create_dm_channel(...)` | `RestClient::create_dm_channel_typed(...)` |
| `RestClient::create_interaction_response(...)` | `RestClient::create_interaction_response_typed(...)` or typed helper functions |
| `RestClient::bulk_overwrite_global_commands(...)` | `RestClient::bulk_overwrite_global_commands_typed(...)` |
| `discordrs::builders::modal::*` | `discordrs::builders::{...}` or crate root re-exports |
| generic `DiscordModel` access for `ApplicationCommand` | `ApplicationCommand::id_opt()` / `ApplicationCommand::created_at()` |

## 2. Start a Typed Gateway Bot

`Client` is the primary runtime entry point. `BotClient` remains as a compatibility alias.

Prefer `EventHandler::handle_event(...)` when you want typed `Event` dispatch from a single match point. The legacy per-event convenience callbacks remain available for compatibility, and `ready`, `message_create`, and `interaction_create` now also receive typed payloads.

```rust
use async_trait::async_trait;
use discordrs::{gateway_intents, Client, Context, Event, EventHandler};

struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn handle_event(&self, ctx: Context, event: Event) {
        match event {
            Event::Ready(ready) => {
                println!("READY: {}", ready.data.user.username);
                println!("Shard: {:?}", ctx.shard_pair());
            }
            Event::MessageCreate(message) => {
                println!("MESSAGE_CREATE: {}", message.message.content);
            }
            _ => {}
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), discordrs::DiscordError> {
    let token = std::env::var("DISCORD_TOKEN")?;

    Client::builder(
        &token,
        gateway_intents::GUILDS | gateway_intents::GUILD_MESSAGES,
    )
    .event_handler(Handler)
    .start()
    .await?;

    Ok(())
}
```

## 3. Create a `Context` Outside the Runtime

If you have test code or helper code that used to build `Context` manually, use `Context::new(http, data)`.

```rust
use std::sync::Arc;

use discordrs::{Context, DiscordHttpClient, TypeMap};
use tokio::sync::RwLock;

let http = Arc::new(DiscordHttpClient::new("token", 0));
let data = Arc::new(RwLock::new(TypeMap::new()));

let ctx = Context::new(http, data);
assert_eq!(ctx.shard_pair(), (0, 1));
```

`Context::new(...)` gives you a default standalone context:

- fresh `CacheHandle`
- shard pair `(0, 1)`
- empty gateway command map
- default `VoiceManager` when `voice` is enabled
- default `CollectorHub` when `collectors` is enabled

## 4. Register Typed Commands

Use command builders instead of passing raw JSON command bodies.

```rust
use discordrs::{
    option_type, CommandOptionBuilder, PermissionsBitField, SlashCommandBuilder,
};

let command = SlashCommandBuilder::new("ticket", "Create a support ticket")
    .string_option("topic", "Ticket topic", true)
    .option(
        CommandOptionBuilder::new(option_type::BOOLEAN, "private", "Create as private ticket")
            .required(false),
    )
    .default_member_permissions(PermissionsBitField(0))
    .build();
```

With a REST client:

```rust
use discordrs::{DiscordHttpClient, SlashCommandBuilder};

async fn register(http: &DiscordHttpClient) -> Result<(), discordrs::DiscordError> {
    let command = SlashCommandBuilder::new("hello", "Reply with hello").build();
    http.create_global_command(&command).await?;
    Ok(())
}
```

## 4.5 OAuth2 Backend Helpers

Use `OAuth2Client` for application backend OAuth2 flows. It is separate from `RestClient` because OAuth2 token endpoints use form-encoded application credentials, not bot-token `Authorization`.

```rust
use discordrs::{
    OAuth2AuthorizationRequest, OAuth2Client, OAuth2CodeExchange, OAuth2Scope,
};

async fn oauth_flow() -> Result<(), discordrs::DiscordError> {
    let oauth = OAuth2Client::new("client_id", "client_secret");
    let authorize_url = oauth.authorization_url(
        OAuth2AuthorizationRequest::code(
            "https://app.example/callback",
            [OAuth2Scope::identify(), OAuth2Scope::guilds()],
        )
        .state("csrf-token")
        .prompt("consent"),
    )?;

    println!("Open: {authorize_url}");

    let token = oauth
        .exchange_code(OAuth2CodeExchange::new(
            "returned-code",
            "https://app.example/callback",
        ))
        .await?;
    println!("OAuth token type: {}", token.token_type);
    Ok(())
}
```

## 5. Send Messages with Typed Helpers

If you want a typed message body instead of hand-written JSON, use `MessageBuilder` and `send_message`.

```rust
use discordrs::{send_message, ActionRowBuilder, ButtonBuilder, MessageBuilder, button_style};

async fn send_panel(
    http: &discordrs::DiscordHttpClient,
    channel_id: u64,
) -> Result<(), discordrs::DiscordError> {
    let message = MessageBuilder::new()
        .content("Support panel")
        .components(vec![
            ActionRowBuilder::new()
                .add_button(
                    ButtonBuilder::new()
                        .custom_id("ticket_open")
                        .label("Open Ticket")
                        .style(button_style::PRIMARY),
                )
                .build(),
        ]);

    send_message(http, channel_id, message.build()).await?;
    Ok(())
}
```

For Components V2 containers, the existing builder path still works:

```rust
use discordrs::{
    button_style, create_container, send_container_message, ButtonConfig, DiscordHttpClient,
};

async fn send_support_panel(
    http: &DiscordHttpClient,
    channel_id: u64,
) -> Result<(), discordrs::DiscordError> {
    let buttons = vec![
        ButtonConfig::new("ticket_open", "Open Ticket").style(button_style::PRIMARY),
        ButtonConfig::new("ticket_status", "Check Status").style(button_style::SECONDARY),
    ];

    let container = create_container(
        "Support Panel",
        "Use the buttons below to manage support requests.",
        buttons,
        None,
    );

    send_container_message(http, channel_id, container).await?;
    Ok(())
}
```

## 6. Reply to Gateway Interactions Without Raw JSON

`Context` now exposes direct gateway control helpers, and the helpers module exposes typed response helpers.

```rust
use async_trait::async_trait;
use discordrs::{
    defer_interaction, followup_message, gateway_intents, Client, Context, Event, EventHandler,
    MessageBuilder,
};

struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn handle_event(&self, ctx: Context, event: Event) {
        if let Event::InteractionCreate(interaction) = event {
            let interaction = interaction.interaction;
            let interaction_ctx = interaction.context().clone();
            let response = MessageBuilder::new().content("Working...").build();

            let _ = defer_interaction(&ctx.http, &interaction_ctx, true).await;
            let _ = followup_message(&ctx.http, &interaction_ctx, response, true).await;
        }
    }
}
```

Other typed helper entry points:

- `respond_to_interaction(...)`
- `respond_with_message(...)`
- `update_interaction_message(...)`
- `respond_with_modal_typed(...)`

## 7. Build a Typed Interactions Endpoint

If you run an outgoing-interactions HTTP server instead of the gateway runtime, prefer the typed endpoint helpers.

```rust
use async_trait::async_trait;
use axum::Router;
use discordrs::{
    Interaction, InteractionContextData, InteractionResponse, TypedInteractionHandler,
    try_typed_interactions_endpoint,
};

#[derive(Clone)]
struct Handler;

#[async_trait]
impl TypedInteractionHandler for Handler {
    async fn handle_typed(
        &self,
        _ctx: InteractionContextData,
        interaction: Interaction,
    ) -> InteractionResponse {
        match interaction {
            Interaction::ChatInputCommand(command)
                if command.data.name.as_deref() == Some("hello") =>
            {
                InteractionResponse::ChannelMessage(serde_json::json!({
                    "content": "Hello from typed endpoint"
                }))
            }
            _ => InteractionResponse::DeferredMessage,
        }
    }
}

fn build_router(public_key: &str) -> Router {
    try_typed_interactions_endpoint(public_key, Handler)
        .expect("invalid Discord public key")
}
```

Use `try_interactions_endpoint(...)` instead when you intentionally want the raw interaction surface.

Typed slash/autocomplete input now keeps real user-entered option data. `interaction.data.options` uses `CommandInteractionOption`, which preserves nested options plus `value` and `focused` for autocomplete flows.

## 8. Use Cache-Aware Managers

On the gateway runtime, `Context` exposes manager shortcuts in all builds:

- `ctx.guilds()`
- `ctx.channels()`
- `ctx.members()`
- `ctx.messages()`
- `ctx.roles()`

These managers keep the REST handle and cache handle together. The `cache` feature is enabled by default, so normal installs store gateway cache data in memory before falling back to HTTP. The default cache policy is bounded; if you compile with `default-features = false`, the same types still exist but cached reads stay empty. Use `CacheHandle::is_enabled()` when shared code needs to detect that mode.

```rust
async fn inspect_cache(ctx: &discordrs::Context) {
    let guilds = ctx.guilds().list_cached().await;
    println!("Cached guilds: {}", guilds.len());
}
```

For long-running bots, tune the bounded defaults to match your guild size and lookup patterns:

```rust
use std::time::Duration;
use discordrs::{gateway_intents, CacheConfig, Client};

let client = Client::builder("bot-token", gateway_intents::GUILD_MESSAGES)
    .cache_config(
        CacheConfig::default()
            .max_messages_per_channel(100)
            .max_total_messages(10_000)
            .message_ttl(Duration::from_secs(60 * 60))
            .presence_ttl(Duration::from_secs(10 * 60))
            .max_presences(50_000)
            .max_members_per_guild(25_000),
    );
```

Use `CacheConfig::unbounded()` only when retaining all cached gateway data is an intentional operator decision.

## 8.5 REST Safety Notes

`RestClient` validates token-like path segments before authenticated routes are built. This includes invite codes passed to `get_invite`, `get_invite_with_options`, and `delete_invite`, so user-provided invite text cannot inject `/`, `\`, `?`, `#`, or control characters into bot-authorized REST paths.

Generated query strings are percent-encoded. Request body serialization failures return `DiscordError::Json` instead of panicking, and repeated HTTP 429 responses are retried up to a bounded limit before surfacing `DiscordError::RateLimit`.

## 9. Control the Active Shard from `Context`

When you are inside a gateway handler, `Context` can drive shard-local gateway actions directly.

```rust
async fn rotate_presence(ctx: &discordrs::Context) -> Result<(), discordrs::DiscordError> {
    ctx.update_presence("Handling tickets").await?;
    Ok(())
}
```

Available `Context` control methods:

- `shard_messenger().await`
- `update_presence(...).await`
- `reconnect_shard().await`
- `shutdown_shard().await`
- `update_voice_state(...).await`
- `join_voice(...).await`
- `leave_voice(...).await`

If you want the underlying shard-local sender, call `ctx.shard_messenger().await` and use `ShardMessenger` directly.

## 10. Spawn and Supervise Multiple Shards

With `gateway + sharding`, you have two entry points:

- `start_shards(count)`: spawn and wait until all shard tasks finish
- `spawn_shards(count)`: return a `ShardSupervisor` so you can inspect state and control shards yourself

```rust
use async_trait::async_trait;
use discordrs::{gateway_intents, Client, Context, EventHandler};

struct Handler;

#[async_trait]
impl EventHandler for Handler {}

#[tokio::main]
async fn main() -> Result<(), discordrs::DiscordError> {
    let token = std::env::var("DISCORD_TOKEN")?;

    let supervisor = Client::builder(&token, gateway_intents::GUILDS)
        .event_handler(Handler)
        .spawn_shards(4)
        .await?;

    for status in supervisor.statuses() {
        println!("Shard {} state: {:?}", status.info.id, status.state);
    }

    supervisor.shutdown_and_wait().await?;
    Ok(())
}
```

Current sharding behavior:

- initial shard boot is queued instead of identifying every shard at once
- queued shards report `ShardRuntimeState::Queued`
- later shards wait for the earlier shard boot window before being released
- shutdown can be awaited with `shutdown_and_wait()` or `wait_for_shutdown(timeout)`
- reconnect backoff is interruptible, so shutdown does not wait for a long sleep to finish

Useful supervisor APIs:

- `statuses()`
- `drain_events()`
- `send(shard_id, ShardIpcMessage)`
- `reconnect(shard_id)`
- `update_presence(shard_id, ...)`
- `join_voice(shard_id, ...)`
- `leave_voice(shard_id, ...)`
- `shutdown()`
- `shutdown_and_wait().await`
- `wait_for_shutdown(duration).await`

## 11. Voice Manager and Voice Runtime

There are two layers:

- `VoiceManager`: tracks gateway voice state/server updates and local queue state
- `VoiceRuntime`: performs voice websocket and UDP handshake work

From `Context`, the common gateway-driven flow is:

```rust
#[cfg(feature = "voice")]
async fn join_and_prepare_voice(ctx: &discordrs::Context) -> Result<(), discordrs::DiscordError> {
    ctx.join_voice("1", "2", false, false).await?;

    if let Some(config) = ctx.voice_runtime_config("1", "1234").await {
        println!("Voice endpoint: {}", config.websocket_url());
    }

    Ok(())
}
```

If you already have a full runtime config, connect directly:

```rust
use std::time::Duration;

use discordrs::{
    connect_voice_runtime, VoiceOpusDecoder, VoiceOpusFrame, VoiceRuntimeConfig, VoiceSpeakingFlags,
};

async fn connect_runtime() -> Result<(), discordrs::DiscordError> {
    let handle = connect_voice_runtime(VoiceRuntimeConfig::new(
        "1",
        "42",
        "session-id",
        "voice-token",
        "wss://voice.discord.media/?v=8",
    ))
    .await?;

    handle.set_speaking(VoiceSpeakingFlags::MICROPHONE, 0)?;
    handle
        .send_opus_frame(&[0xf8, 0xff, 0xfe], Duration::from_millis(20))
        .await?;
    handle
        .play_opus_frames([VoiceOpusFrame::new([0xf8, 0xff, 0xfe])])
        .await?;

    let mut decoder = VoiceOpusDecoder::discord_default()?;
    let packet = handle.recv_decoded_voice_packet(&mut decoder, 2048).await?;
    println!(
        "received {} samples/channel from SSRC {}",
        packet.samples_per_channel,
        packet.packet.rtp.ssrc
    );
    handle.close().await?;
    Ok(())
}
```

The current runtime covers:

- voice websocket hello and identify
- ready payload handling
- UDP IP discovery
- select protocol
- session description wait
- speaking updates
- server speaking/SSRC-to-user tracking when the voice gateway sends that mapping
- Opus-frame RTP send with sequence/timestamp/nonce management
- simple paced Opus-frame playback through `play_opus_frames(...)`
- 48 kHz stereo 20 ms PCM encode/playback through `PcmFrame`, `AudioSource`, `AudioMixer`, and `VoiceOpusEncoder` when `voice-encode` is enabled
- raw UDP packet receive
- RTP header parsing with CSRC/extension-aware RTP-size header calculation
- transport decrypt for `aead_aes256_gcm_rtpsize` and `aead_xchacha20_poly1305_rtpsize`
- pure-Rust Opus decode to interleaved `i16` PCM through `VoiceOpusDecoder`
- DAVE opcode state tracking and a `VoiceDaveFrameDecryptor` hook
- experimental `VoiceDaveySession`, `VoiceDaveFrameEncryptor`, `send_opus_frame_with_dave(...)`, and DAVE MLS outbound command helpers when the `dave` feature is enabled
- graceful close

The default `voice` feature can send already-encoded Opus frames, returns transport-decrypted Opus frames, and can decode received frames to PCM.
`recv_voice_packet(...)` still rejects active DAVE sessions unless the caller uses
`recv_voice_packet_with_dave(...)` or `recv_decoded_voice_packet_with_dave(...)` with a DAVE decryptor.
The `dave` feature exposes a `davey`/OpenMLS-backed session wrapper and outbound media insertion point, but live Discord DAVE interoperability still depends on validating the full gateway MLS transition lifecycle for the target voice session.

## 12. Modal and Components V2 Helpers

V2 modal parsing still preserves Discord-specific component types such as `FileUpload`, `RadioGroup`, `CheckboxGroup`, and `Checkbox`.

```rust
use discordrs::{
    create_container, parse_interaction_context, parse_raw_interaction,
    respond_modal_with_container, DiscordHttpClient, RawInteraction, V2ModalSubmission,
};
use serde_json::Value;

fn summarize(submission: &V2ModalSubmission) -> String {
    let theme = submission.get_radio_value("theme").unwrap_or("Not selected");
    let channels = submission
        .get_select_values("notify_channels")
        .map(|v| v.join(", "))
        .unwrap_or_else(|| "None".to_string());
    let files = submission
        .get_file_values("attachments")
        .map(|v| v.join(", "))
        .unwrap_or_else(|| "No files".to_string());

    format!("Theme: {theme}, Notifications: {channels}, Files: {files}")
}

async fn handle_modal(http: &DiscordHttpClient, payload: &Value) -> Result<(), discordrs::DiscordError> {
    let ctx = parse_interaction_context(payload)?;

    if let RawInteraction::ModalSubmit(submission) = parse_raw_interaction(payload)? {
        let result = summarize(&submission);
        let container = create_container("Modal Processed", &result, vec![], None);
        respond_modal_with_container(http, &ctx.id, &ctx.token, container, true).await?;
    }

    Ok(())
}
```

## 13. Frequently Used APIs

- `Client::builder(token, intents)`
- `Context::new(http, data)`
- `Context::rest()`
- `RestClient::new(token, application_id)`
- `get_poll_answer_voters(...)`
- `end_poll(...)`
- `get_sku_subscriptions(...)`
- `get_sku_subscription(...)`
- `get_guild_integrations(...)`
- `list_thread_members(...)`
- `list_public_archived_threads(...)`
- `list_private_archived_threads(...)`
- `list_joined_private_archived_threads(...)`
- `get_active_guild_threads(...)`
- `SlashCommandBuilder`, `UserCommandBuilder`, `MessageCommandBuilder`
- `MessageBuilder`, `InteractionResponseBuilder`
- `send_message(...)`
- `respond_to_interaction(...)`
- `respond_with_message(...)`
- `followup_message(...)`
- `defer_interaction(...)`
- `update_interaction_message(...)`
- `parse_interaction(...)`
- `parse_raw_interaction(...)`
- `try_interactions_endpoint(...)`
- `try_typed_interactions_endpoint(...)`
- `CacheHandle`, `GuildManager`, `ChannelManager`, `MemberManager`, `MessageManager`, `RoleManager`
- `ShardMessenger`
- `ShardSupervisor`
- `VoiceRuntimeConfig`
- `connect_voice_runtime(...)`
- `VoiceOpusDecoder`
- `VoiceOpusEncoder` with `voice,voice-encode`
- `VoiceDaveFrameDecryptor`
- `VoiceDaveFrameEncryptor` and `VoiceDaveySession` with `voice,dave`

## 14. Notes

- `Client` is the main gateway runtime surface. `BotClient` is kept as an alias for compatibility.
- `EventHandler::handle_event(...)` is the typed gateway entry point. Legacy callbacks such as `ready`, `message_create`, and `interaction_create` are still available for compatibility and now receive typed payloads.
- `RestClient` is the preferred REST-facing name. `DiscordHttpClient` remains available.
- Prefer the typed `RestClient` methods for new code.
- Token-authenticated `/interactions/...` and `/webhooks/...` requests intentionally omit bot `Authorization` headers, and webhook/callback path segments are validated before requests are built.
- `Context::new(...)` exists for tests and helper code that need a standalone context outside the live gateway runtime.
- Prefer builder imports from `discordrs::builders::{...}` or the crate root re-exports. Deeper implementation submodules are private.
- Use `ApplicationCommand::id_opt()` until Discord has assigned an ID. Unsaved commands are no longer treated as generic `DiscordModel`s.
- `spawn_shards(...)` is the right choice when you want status inspection, manual shutdown, or supervisor-driven shard control.
- `start_shards(...)` is the right choice when you only want the runtime to own the shard lifecycle and block until it exits.
- `voice` currently provides handshake, state plumbing, raw UDP receive, transport-decrypted Opus frames, Opus send, and PCM decode. `voice-encode` adds PCM-to-Opus playback. Full DAVE/MLS operation is exposed through an experimental feature because it requires live gateway MLS transition handling.
- Poll vote, subscription, integration, entitlement, soundboard, invite, thread, and forum data now have typed models/events or REST wrappers where Discord documents them.

## 15. Testing And Coverage

Coverage-specific workflow guidance lives in:

- [`discordrsdocs/docs/guide/testing-and-coverage.md`](discordrsdocs/docs/guide/testing-and-coverage.md)

Use that guide when you need repeatable local HTTP harnesses, websocket harnesses, or a fast order
for attacking low-coverage modules.
