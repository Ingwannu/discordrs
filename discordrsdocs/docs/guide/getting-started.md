# Getting Started

## Prerequisites

- Rust stable toolchain
- A Discord application + bot token
- Optional: public endpoint if using Interaction Endpoint mode

## Add Dependency

```toml
[dependencies]
discordrs = { version = "1.0.0", features = ["gateway"] }
```

## Minimal Typed Gateway Bot

```rust
use async_trait::async_trait;
use discordrs::{gateway_intents, Client, Context, Event, EventHandler};

struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn handle_event(&self, _ctx: Context, event: Event) {
        if let Event::Ready(ready) = event {
            println!("READY as {}", ready.data.user.username);
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

## Environment Variable

```bash
export DISCORD_TOKEN="your-bot-token"
```

## Run

```bash
cargo run
```

## Next

- Go to [Usage Guide](usage-guide.md)
- Read [Architecture](architecture.md)
- Explore [Commands API](../api/commands.md)

