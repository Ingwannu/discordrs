# Gateway API

Gateway runtime is provided behind the `gateway` feature.

## Primary Types

- `GatewayClient`: raw websocket lifecycle management (identify, heartbeat, resume, reconnect)
- `BotClient`: high-level runtime that binds gateway events to your `EventHandler`
- `Context`: shared state in handlers (`http`, optional typemap)
- `EventHandler`: async trait for event callbacks

## Setup

```toml
[dependencies]
discordrs = { version = "0.3.0", features = ["gateway"] }
```

## Boot Pattern

```rust
BotClient::builder(&token, gateway_intents::GUILDS | gateway_intents::GUILD_MESSAGES)
    .event_handler(handler)
    .start()
    .await?;
```

## Event Surface

Implement what you need:

- `ready`
- `message_create`
- `interaction_create`
- `raw_event`

## Operational Notes

- Keep handler methods non-blocking.
- Push heavy work to background tasks.
- Use `Context.http` for follow-up API calls.
