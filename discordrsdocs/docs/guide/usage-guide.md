# discord.rs Usage

`discord.rs` is a standalone Discord bot framework for Rust with a typed Gateway runtime, typed REST surface, Components V2 builders, cache managers, and collectors.

Brand name: discord.rs. The crates.io package name and Rust import path remain `discordrs`.

## 1. Pick a runtime mode

```toml
[dependencies]
# Core only
discordrs = "1.1.0"

# Typed gateway runtime
discordrs = { version = "1.1.0", features = ["gateway"] }

# Typed gateway runtime with cache storage enabled
discordrs = { version = "1.1.0", features = ["gateway", "cache"] }

# Typed gateway runtime with collectors
discordrs = { version = "1.1.0", features = ["gateway", "collectors"] }

# HTTP interactions endpoint
discordrs = { version = "1.1.0", features = ["interactions"] }

# Voice receive and Opus decode
discordrs = { version = "1.1.0", features = ["voice"] }

# Experimental DAVE/MLS receive hook
discordrs = { version = "1.1.0", features = ["voice", "dave"] }
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

## 6. Use typed Discord coverage before raw JSON

- Polls: `CreatePoll`, `Poll`, `get_poll_answer_voters(...)`, `end_poll(...)`, `MESSAGE_POLL_VOTE_ADD`, `MESSAGE_POLL_VOTE_REMOVE`
- Monetization: `Sku`, `Entitlement`, `Subscription`, entitlement helpers, SKU subscription helpers, `ENTITLEMENT_*`, `SUBSCRIPTION_*`
- Soundboard: default/guild soundboard REST helpers plus `GUILD_SOUNDBOARD_*` and `SOUNDBOARD_SOUNDS`
- Threads and forums: thread member/detail/archive helpers plus forum tags, applied tags, default reactions, and default thread slowmode fields
- Integrations and invites: integration list/delete, `INTEGRATION_*`, invite options, `INVITE_CREATE`, and `INVITE_DELETE`

## 7. Voice receive boundaries

```rust
use discordrs::{connect_voice_runtime, VoiceOpusDecoder, VoiceRuntimeConfig};

async fn receive_pcm() -> Result<(), discordrs::DiscordError> {
    let handle = connect_voice_runtime(VoiceRuntimeConfig::new(
        "guild_id",
        "bot_user_id",
        "voice_session_id",
        "voice_token",
        "wss://voice.discord.media/?v=8",
    ))
    .await?;

    let mut decoder = VoiceOpusDecoder::discord_default()?;
    let decoded = handle.recv_decoded_voice_packet(&mut decoder, 2048).await?;
    println!("{} PCM samples/channel", decoded.samples_per_channel);
    handle.close().await
}
```

Default `voice` covers raw UDP receive, RTP header parsing, RTP-size transport decrypt, and Opus PCM decode. Active DAVE sessions require `recv_voice_packet_with_dave(...)` or `recv_decoded_voice_packet_with_dave(...)` with a `VoiceDaveFrameDecryptor`; the `dave` feature exposes an experimental `VoiceDaveyDecryptor`.

## 8. Keep old raw helpers only for migration

- `parse_raw_interaction(...)` still exists
- `BotClient` still exists as a compatibility alias
- `EventHandler::handle_event(...)` is the typed gateway entry point; legacy callbacks such as `ready`, `message_create`, and `interaction_create` remain available for compatibility and now accept typed payloads too
- New code should prefer `Client`, `Event`, `RestClient`, and `parse_interaction(...)`

