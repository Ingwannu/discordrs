# discordrs Usage

`discordrs` is a standalone Discord bot framework for Rust with a typed Gateway runtime, typed REST surface, Components V2 builders, cache managers, and collectors.

## 1. Pick a runtime mode

```toml
[dependencies]
# Core only
discordrs = "1.0.0"

# Typed gateway runtime
discordrs = { version = "1.0.0", features = ["gateway"] }

# Typed gateway runtime with cache storage enabled
discordrs = { version = "1.0.0", features = ["gateway", "cache"] }

# Typed gateway runtime with collectors
discordrs = { version = "1.0.0", features = ["gateway", "collectors"] }

# HTTP interactions endpoint
discordrs = { version = "1.0.0", features = ["interactions"] }
```

## 2. Start a typed Gateway client

```rust
use async_trait::async_trait;
use discordrs::{gateway_intents, Client, Context, Event, EventHandler};

struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn handle_event(&self, _ctx: Context, event: Event) {
        match event {
            Event::Ready(ready) => println!("READY: {}", ready.data.user.username),
            Event::MessageCreate(message) => println!("MESSAGE_CREATE: {}", message.message.id),
            _ => {}
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), discordrs::DiscordError> {
    let token = std::env::var("DISCORD_TOKEN")?;
    Client::builder(&token, gateway_intents::GUILDS | gateway_intents::GUILD_MESSAGES)
        .event_handler(Handler)
        .start()
        .await?;
    Ok(())
}
```

## 3. Register typed commands

```rust
use discordrs::{option_type, CommandOptionBuilder, SlashCommandBuilder};

let command = SlashCommandBuilder::new("ticket", "Create a support ticket")
    .option(
        CommandOptionBuilder::new(option_type::STRING, "topic", "Ticket topic")
            .required(true)
            .autocomplete(true),
    )
    .build();
```

## 4. Use typed REST plus helpers

- Use `RestClient` for direct REST access and manager-backed lookups.
- Use helper functions when you are building Components V2 or interaction responses.
- Keep payload building inside the fluent builders instead of hand-written JSON whenever possible.

## 5. Turn on cache or collectors when the bot needs them

- `cache`: enables the in-memory cache storage used by `CacheHandle` and gateway manager reads
- `collectors`: enables async collectors for messages, interactions, components, and modals

## 6. Keep old raw helpers only for migration

- `parse_raw_interaction(...)` still exists
- `BotClient` still exists as a compatibility alias
- `EventHandler::handle_event(...)` is the typed gateway entry point; legacy callbacks such as `ready`, `message_create`, and `interaction_create` remain available for compatibility and now accept typed payloads too
- New code should prefer `Client`, `Event`, `RestClient`, and `parse_interaction(...)`

