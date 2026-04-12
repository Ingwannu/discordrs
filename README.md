# discordrs

Standalone Discord bot framework for Rust with typed models, typed gateway events, Components V2, collectors, cache managers, and HTTP client

## Features

- Typed `Client` runtime with `Event` enum dispatch and compatibility `BotClient` alias
- Typed `RestClient` with shared route/global rate-limit state and compatibility `DiscordHttpClient` alias
- `prelude::*` re-exports for common runtime, builder, helper, and response types
- Cache-backed manager reads for guilds, channels, members, roles, and messages, with in-memory storage enabled by the `cache` feature
- Collectors for messages, interactions, components, and modals behind the `collectors` feature
- Gateway WebSocket client with connect, heartbeat, identify, resume, reconnect, and terminal close-code handling
- Shard supervisor and shard messenger control paths for queued shard boot, reconnect, shutdown, presence, and voice state updates
- Voice manager plus voice runtime handshake support for websocket hello/identify, UDP discovery, select-protocol, and speaking updates
- Components V2 builders (`Container`, `TextDisplay`, `Section`, `MediaGallery`, `Button`, `SelectMenu`, and more)
- Typed command builders for slash, user, and message commands
- Modal builders with `RadioGroup`, `CheckboxGroup`, `Checkbox`, and `FileUpload`
- V2 modal submission parser with preserved `FileUpload`, `RadioGroup`, `CheckboxGroup`, and other V2 component types
- Typed interaction decoding with chat-input, context-menu, autocomplete, component, and modal submit variants
- Interaction routing helpers: `parse_interaction`, `parse_raw_interaction`, `parse_interaction_context`, and `try_interactions_endpoint`
- Feature-gated runtime and storage layers: `gateway`, `interactions`, `cache`, `collectors`, `sharding`, and `voice`

## Install

```toml
[dependencies]
discordrs = "1.0.0"
```

```toml
[dependencies]
# Gateway bot client
discordrs = { version = "1.0.0", features = ["gateway"] }

# HTTP Interactions Endpoint
discordrs = { version = "1.0.0", features = ["interactions"] }

# Gateway runtime with cache storage enabled
discordrs = { version = "1.0.0", features = ["gateway", "cache"] }

# Gateway runtime with collectors
discordrs = { version = "1.0.0", features = ["gateway", "collectors"] }

# Sharding foundations
discordrs = { version = "1.0.0", features = ["gateway", "sharding"] }

# Voice foundations
discordrs = { version = "1.0.0", features = ["voice"] }

# Both runtime modes
discordrs = { version = "1.0.0", features = ["gateway", "interactions"] }
```

## API Cleanup

Recent API cleanup tightened the public surface:

- `RestClient` now exposes the typed REST methods as the supported public path. The old raw convenience methods such as `send_message`, `edit_message`, `create_dm_channel`, `create_interaction_response`, and `bulk_overwrite_global_commands` are no longer public.
- Builder implementation submodules are private. Import builders from `discordrs::builders::{...}` or use the crate root re-exports.
- `ApplicationCommand` no longer implements `DiscordModel` because its ID is optional until Discord assigns one. Use `ApplicationCommand::id_opt()` and `ApplicationCommand::created_at()` instead.

If you are upgrading existing code, the common replacements are:

| Old path | New path |
|----------|----------|
| `RestClient::send_message(...)` | `send_message(...)` helper or `RestClient::create_message(...)` |
| `RestClient::edit_message(...)` | `RestClient::update_message(...)` |
| `RestClient::create_dm_channel(...)` | `RestClient::create_dm_channel_typed(...)` |
| `RestClient::create_interaction_response(...)` | `RestClient::create_interaction_response_typed(...)` or typed helper functions |
| `RestClient::bulk_overwrite_global_commands(...)` | `RestClient::bulk_overwrite_global_commands_typed(...)` |
| `discordrs::builders::modal::*` | `discordrs::builders::{...}` or crate root re-exports |
| generic `DiscordModel` access for `ApplicationCommand` | `ApplicationCommand::id_opt()` / `ApplicationCommand::created_at()` |

## Documentation Site

A navigation-focused docs website (similar to the discord.js docs browsing style) is available in [`discordrsdocs/`](discordrsdocs/):

- Live URL: `http://discordrs.teamwicked.me/discordrsdocs/#/`
- Korean URL: `http://discordrs.teamwicked.me/discordrsdocs/#/ko/README`
- Entry: [`discordrsdocs/index.html`](discordrsdocs/index.html)
- Testing guide: [`discordrsdocs/docs/guide/testing-and-coverage.md`](discordrsdocs/docs/guide/testing-and-coverage.md)
- Local preview:
  ```bash
  python3 -m http.server 8080 --directory discordrsdocs
  ```

## Quick Example

```rust
use async_trait::async_trait;
use discordrs::{gateway_intents, Client, Context, Event, EventHandler};

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

## Typed APIs

```rust
use discordrs::{
    option_type, CommandOptionBuilder, PermissionsBitField, SlashCommandBuilder,
};

let command = SlashCommandBuilder::new("ticket", "Create a support ticket")
    .option(
        CommandOptionBuilder::new(option_type::STRING, "topic", "Ticket topic")
            .required(true)
            .autocomplete(true),
    )
    .default_member_permissions(PermissionsBitField(0))
    .build();
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
    CheckboxBuilder, CheckboxGroupBuilder, FileUploadBuilder, ModalBuilder, RadioGroupBuilder,
    SelectOption,
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
    )
    .add_file_upload(
        "Screenshot",
        Some("Attach one or more files"),
        FileUploadBuilder::new("attachments").min_values(1),
    );
```

## Interactions Endpoint Example

```rust
use async_trait::async_trait;
use axum::Router;
use discordrs::{
    create_container, InteractionContext, InteractionHandler, InteractionResponse, RawInteraction,
    try_interactions_endpoint,
};

#[derive(Clone)]
struct Handler;

#[async_trait]
impl InteractionHandler for Handler {
    async fn handle(
        &self,
        ctx: InteractionContext,
        interaction: RawInteraction,
    ) -> InteractionResponse {
        match interaction {
            RawInteraction::Command { name, .. } if name.as_deref() == Some("hello") => {
                let data = serde_json::json!({
                    "components": [create_container("Hello", "World", vec![], None).build()],
                    "flags": 1 << 15,
                });
                let _ = ctx;
                InteractionResponse::ChannelMessage(data)
            }
            _ => InteractionResponse::Raw(serde_json::json!({ "type": 5 })),
        }
    }
}

fn app(public_key: &str) -> Router {
    try_interactions_endpoint(public_key, Handler)
        .expect("invalid Discord public key")
}
```

## Feature Flags

| Feature | Description | Key deps |
|---------|-------------|----------|
| (default) | Builders, typed models, command builders, parsers, REST client, helpers | reqwest, serde_json |
| `gateway` | Gateway WebSocket, `Client`, typed `Event`, and `EventHandler::handle_event(...)` dispatch | tokio-tungstenite, flate2, async-trait |
| `interactions` | HTTP Interactions Endpoint with Ed25519 | axum, ed25519-dalek |
| `cache` | Enables the in-memory cache storage used by gateway cache managers | tokio |
| `collectors` | Async collectors for messages and interactions | tokio |
| `sharding` | Sharding manager and reusable gateway config abstractions | tokio |
| `voice` | In-memory voice connection and player skeletons | tokio |

## Notes

- `discordrs` started as a helper around serenity workflows, and `1.0.0` is the first stabilized standalone framework release with the typed runtime surface.
- Use `try_interactions_endpoint()` when you want invalid public keys to fail at startup instead of during requests.
- Use `discordrs::prelude::*` when you want the shortest path to the main runtime, command, helper, and response APIs.
- Use `DiscordHttpClient::create_followup_message_with_application_id()` when you already have `InteractionContext.application_id` and the client was not initialized with an application id.
- Prefer the typed `RestClient` methods such as `create_message`, `update_message`, `create_interaction_response_typed`, and `bulk_overwrite_*_typed`.
- Token-authenticated `/interactions/...` and `/webhooks/...` requests intentionally omit bot `Authorization` headers, and token/path segments are validated before webhook/callback paths are built.
- Typed slash and autocomplete interaction payloads preserve option `value`, `focused`, and nested option input through `CommandInteractionOption`.
- Use `Client` for new gateway code. `BotClient` remains available as a compatibility alias.
- Use `EventHandler::handle_event(...)` when you want one typed entry point for every gateway event. Legacy convenience callbacks such as `ready`, `message_create`, and `interaction_create` still exist, but they now receive typed payloads too.
- Use `Context::new(...)` when tests or helper crates need a standalone context outside the live gateway runtime.
- Prefer builder imports from `discordrs::builders::{...}` or the crate root re-exports. Deeper implementation submodules are private.
- Use `ApplicationCommand::id_opt()` until Discord has assigned an ID. Unsaved commands are no longer treated as generic `DiscordModel`s.
- The parser keeps V2 modal component types, including `FileUpload`, `RadioGroup`, and `CheckboxGroup`, so routing logic can keep full fidelity.

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT license ([LICENSE-MIT](LICENSE-MIT))

at your option.

## Developer

- ingwannu
- Contact: ingwannu@teamwicked.me, ingwannu@gmail.com

