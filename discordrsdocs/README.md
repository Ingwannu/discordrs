# discordrs Documentation

A practical docs site for building typed Discord bots with `discordrs`.

> `discordrs` now centers on `Client`, `RestClient`, typed models/events/interactions, command builders, Components V2 helpers, and optional cache/collector layers.

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
- `CacheHandle` plus manager types, with in-memory cache storage enabled by the `cache` feature
- collector types behind the `collectors` feature

## Feature Flags

```toml
[dependencies]
# core only
discordrs = "1.0.0"

# typed gateway runtime
discordrs = { version = "1.0.0", features = ["gateway"] }

# typed gateway runtime with cache storage or collectors
discordrs = { version = "1.0.0", features = ["gateway", "cache"] }
discordrs = { version = "1.0.0", features = ["gateway", "collectors"] }

# interactions endpoint
discordrs = { version = "1.0.0", features = ["interactions"] }
```

## Roadmap Placeholders

- [Sharding](#/docs/api/sharding)
- [Voice](#/docs/api/voice)

## Language

Use the floating `LANG` button (bottom-right) to switch language.

