# discord.rs

> This page mirrors the crate README as source documentation. For the current docs-first surface, prefer the typed runtime pages for `Client`, `RestClient`, commands, cache, and collectors.

discord.rs is a standalone Discord bot framework for Rust with typed models, typed gateway events, Components V2, collectors, cache-aware managers, voice playback/receive helpers, and an HTTP client.

Brand name: discord.rs. The crates.io package name and Rust import path remain `discordrs`.

## Features

- Typed `Client` runtime with `Event` enum dispatch and compatibility `BotClient` alias
- Typed `RestClient` with shared route/global rate-limit state and compatibility `DiscordHttpClient` alias
- Gateway WebSocket client with connect, heartbeat, identify, resume, reconnect, zlib compression, and optional zstd-stream compression
- HTTP REST client with automatic 429 rate-limit retry
- Components V2 builders (`Container`, `TextDisplay`, `Section`, `MediaGallery`, `Button`, `SelectMenu`, and more)
- Modal builders with `RadioGroup`, `CheckboxGroup`, and `Checkbox`
- V2 modal submission parser that preserves all V2 component types that serenity drops
- Interaction routing helpers: `parse_interaction`, `parse_raw_interaction`, `parse_interaction_context`, and `try_interactions_endpoint`
- Cache-backed manager reads with in-memory storage enabled by the `cache` feature and optional `CacheConfig` size limits
- Voice runtime helpers for UDP packet receive, Opus-frame RTP send, PCM-to-Opus encode, RTP-size decrypt, Opus PCM decode, and experimental DAVE hooks
- OAuth2 backend helpers for authorization URLs, code exchange, and refresh-token exchange
- Typed Discord coverage for polls, subscriptions, entitlements, soundboard, thread details, forum fields, invites, integrations, Auto Moderation, guild preview/prune/vanity, voice regions, and bulk bans
- Feature-gated runtime and storage layers: `gateway`, `interactions`, `cache`, `collectors`, `sharding`, `voice`, `voice-encode`, and `dave`

## Install

```toml
[dependencies]
discordrs = "1.2.0"
```

```toml
[dependencies]
# Gateway bot client
discordrs = { version = "1.2.0", features = ["gateway"] }

# HTTP Interactions Endpoint
discordrs = { version = "1.2.0", features = ["interactions"] }

# Both runtime modes
discordrs = { version = "1.2.0", features = ["gateway", "interactions"] }

# Voice playback/receive and experimental DAVE hook
discordrs = { version = "1.2.0", features = ["voice"] }
discordrs = { version = "1.2.0", features = ["voice", "voice-encode"] }
discordrs = { version = "1.2.0", features = ["voice", "dave"] }
```

## Quick Example

```rust
use discordrs::{gateway_intents, Client, Context, Event, EventHandler};
use async_trait::async_trait;

struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn handle_event(&self, _ctx: Context, event: Event) {
        match event {
            Event::Ready(ready) => {
                println!("Bot ready! User: {}", ready.data.user.username);
            }
            Event::MessageCreate(message) => {
                println!("Message: {}", message.message.content);
            }
            _ => {}
        }
    }
}

#[tokio::main]
async fn main() {
    let token = std::env::var("DISCORD_TOKEN").unwrap();
    Client::builder(&token, gateway_intents::GUILDS | gateway_intents::GUILD_MESSAGES)
        .event_handler(Handler)
        .start()
        .await
        .unwrap();
}
```

## Components V2 Example

```rust
use discordrs::{
    button_style, create_container, send_container_message, ButtonConfig, DiscordHttpClient,
};

async fn send_support_panel(http: &DiscordHttpClient, channel_id: u64) -> Result<(), discordrs::DiscordError> {
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

## Modal Example with RadioGroup

```rust
use discordrs::{
    CheckboxBuilder, CheckboxGroupBuilder, ModalBuilder, RadioGroupBuilder, SelectOption,
};

let modal = ModalBuilder::new("preferences_modal", "Preferences")
    .add_radio_group(
        "Theme",
        Some("Pick one"),
        RadioGroupBuilder::new("theme")
            .add_option(SelectOption::new("Light", "light"))
            .add_option(SelectOption::new("Dark", "dark"))
            .required(true),
    )
    .add_checkbox_group(
        "Notifications",
        Some("Choose any"),
        CheckboxGroupBuilder::new("notify_channels")
            .add_option(SelectOption::new("Email", "email"))
            .add_option(SelectOption::new("Push", "push"))
            .min_values(0)
            .max_values(2),
    )
    .add_checkbox(
        "Agree to Terms",
        None,
        CheckboxBuilder::new("agree_terms").required(true),
    );
```

## Feature Flags

| Feature | Description | Key deps |
|---------|-------------|----------|
| (default) | Builders, typed models, command builders, parsers, REST client, helpers | reqwest, serde_json |
| `gateway` | Gateway WebSocket, `Client`, typed `Event`, and `EventHandler::handle_event(...)` dispatch | tokio-tungstenite, flate2, async-trait |
| `zstd-stream` | Gateway zstd-stream transport compression | gateway, zstd |
| `interactions` | HTTP Interactions Endpoint with Ed25519 | axum, ed25519-dalek |
| `cache` | Enables the in-memory cache storage used by gateway cache managers | tokio |
| `collectors` | Async collectors for messages and interactions | tokio |
| `sharding` | Sharding manager and reusable gateway config abstractions | tokio |
| `voice` | Voice gateway/UDP runtime receive, Opus-frame send, transport decrypt, and Opus PCM decode helpers | tokio, aes-gcm, chacha20poly1305, opus-decoder |
| `voice-encode` | PCM source/mixer and `opus-rs` encoder helpers for 48 kHz stereo 20 ms voice playback through the existing Opus frame path | voice, opus-rs |
| `dave` | Experimental DAVE/MLS receive and outbound media hooks backed by `davey`; production interop still requires live Discord transition validation | voice, davey |

## Notes

- `discord.rs` started as a helper around serenity workflows, and `1.0.0` is the first stabilized standalone framework release.
- `EventHandler::handle_event(...)` is the typed gateway entry point. Legacy convenience callbacks such as `ready`, `message_create`, and `interaction_create` remain available and now receive typed payloads too.
- The parser keeps V2 modal component types, including `Label`, `RadioGroup`, and `CheckboxGroup`, so routing logic can keep full fidelity.
- Default `voice` provides Opus-frame RTP send, transport-decrypted received Opus frames, and PCM decode. `voice-encode` adds PCM-to-Opus playback. DAVE/MLS is exposed through the experimental `dave` feature and still needs live gateway transition validation before claiming production interoperability.

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](https://github.com/Ingwannu/discord.rs/blob/main/LICENSE-APACHE))
- MIT license ([LICENSE-MIT](https://github.com/Ingwannu/discord.rs/blob/main/LICENSE-MIT))

at your option.

## Developer

- ingwannu
- Contact: ingwannu@teamwicked.me, ingwannu@gmail.com

