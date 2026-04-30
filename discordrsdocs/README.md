# discord.rs Documentation

A practical docs site for building typed Discord bots with the `discordrs` crate.

> discord.rs now centers on `Client`, `RestClient`, typed models/events/interactions, command builders, Components V2 helpers, and optional cache/collector layers.

Brand name: discord.rs. The crates.io package name and Rust import path remain `discordrs`.

## Start Here

- [Getting Started](#/docs/guide/getting-started)
- [Architecture](#/docs/guide/architecture)
- [Usage Guide](#/docs/guide/usage-guide)
- [Commands API](#/docs/api/commands)
- [Cache and Collectors](#/docs/api/cache-and-collectors)

## Main Runtime Surfaces

- `Client`: typed Gateway runtime with `Event` dispatch through `EventHandler::handle_event(...)`
- `RestClient`: low-level REST surface with shared rate-limit state
- `parse_interaction(...)`: typed interaction decoding
- `SlashCommandBuilder` / `UserCommandBuilder` / `MessageCommandBuilder`
- `CacheHandle` plus manager types, with bounded in-memory cache storage enabled by default
- collector types behind the `collectors` feature
- `connect_voice_runtime(...)`, `VoiceOpusDecoder`, and experimental DAVE hooks behind `voice` / `dave`
- typed REST/event coverage for polls, subscriptions, entitlements, soundboard, thread details, forum fields, invites, and integrations

## Feature Flags

```toml
[dependencies]
# core only
discordrs = "1.2.1"

# typed gateway runtime
discordrs = { version = "1.2.1", features = ["gateway"] }

# typed gateway runtime with cache storage or collectors
discordrs = { version = "1.2.1", features = ["gateway", "cache"] }
discordrs = { version = "1.2.1", features = ["gateway", "collectors"] }

# interactions endpoint
discordrs = { version = "1.2.1", features = ["interactions"] }

# voice receive, Opus decode, and experimental DAVE hook
discordrs = { version = "1.2.1", features = ["voice"] }
discordrs = { version = "1.2.1", features = ["voice", "dave"] }
```

## Runtime Extensions

- [Sharding](#/docs/api/sharding)
- [Voice](#/docs/api/voice)

## Language

Use the floating `LANG` button (bottom-right) to switch language.

