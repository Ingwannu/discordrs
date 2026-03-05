# Getting Started

## Prerequisites

- Rust stable toolchain
- A Discord application + bot token
- Optional: public endpoint if using Interaction Endpoint mode

## Add Dependency

```toml
[dependencies]
discordrs = { version = "0.3.0", features = ["gateway"] }
```

## Minimal Gateway Bot

```rust
use async_trait::async_trait;
use discordrs::{gateway_intents, BotClient, Context, EventHandler};
use serde_json::Value;

struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn ready(&self, _ctx: Context, ready: Value) {
        println!("READY as {}", ready["user"]["username"]);
    }
}

#[tokio::main]
async fn main() -> Result<(), discordrs::Error> {
    let token = std::env::var("DISCORD_TOKEN")?;

    BotClient::builder(&token, gateway_intents::GUILDS | gateway_intents::GUILD_MESSAGES)
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
- Explore [API Reference](../api/builders.md)
