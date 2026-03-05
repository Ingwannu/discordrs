# Changelog

## 0.3.0

- **BREAKING**: Complete rewrite from a serenity helper library to a standalone Discord bot framework.
- **BREAKING**: Helper functions now take `&DiscordHttpClient` with raw `&str` and `u64` IDs instead of serenity model types.
- **Added**: Gateway WebSocket client behind the `gateway` feature.
- **Added**: `BotClient`, `BotClientBuilder`, `EventHandler`, `Context`, and `TypeMap` for gateway bot runtime.
- **Added**: `DiscordHttpClient`, a reqwest-based REST client with automatic HTTP 429 retry.
- **Added**: `parse_raw_interaction()` and `parse_interaction_context()` for interaction routing.
- **Added**: `V2ModalSubmission` parser that preserves `Label`, `RadioGroup`, `CheckboxGroup`, `Checkbox`, and other V2 modal components.
- **Added**: `InteractionContext` with `id`, `token`, `application_id`, `guild_id`, `channel_id`, and `user_id`.
- **Added**: HTTP Interactions Endpoint behind the `interactions` feature, including Ed25519 request verification.
- **Removed**: All serenity dependencies.
- **Changed**: Module structure reorganized into dedicated `gateway/`, `parsers/`, and `builders/` directories.

## 0.1.3

- Added modal interaction components:
  - `RadioGroupBuilder` for single-choice selection.
  - `CheckboxGroupBuilder` for multi-choice selection.
  - `CheckboxBuilder` for yes/no style toggles.
- Updated package version to `0.1.3` in `Cargo.toml`.
