# Changelog

## 1.1.0

- Added Discord Gateway `zlib-stream` handling that keeps compressed payload state across binary frames and inflates only complete payload boundaries.
- Added multipart file upload support through `reqwest`'s multipart feature, including typed message, webhook, and interaction attachment helpers.
- Added typed webhook message CRUD helpers for token-authenticated webhook message fetch, edit, and delete flows.
- Added typed Poll models and REST/event coverage for poll payloads, vote events, and poll ending.
- Expanded Auto Moderation, Scheduled Event, Audit Log, Sticker, Stage Instance, Welcome Screen, Guild Onboarding, Guild Template, Invite, Integration, Forum, Soundboard, Subscription, SKU, and Entitlement models and REST/event coverage.
- Expanded cache coverage for emoji, stickers, voice states, presences, threads, webhooks, scheduled events, AutoMod rules, invites, integrations, soundboard sounds, and monetization entities with cache policy toggles.
- Added voice receive support for raw UDP packets, RTP parsing, AES-GCM and XChaCha RTP-size transport decrypt, and pure-Rust Opus PCM decoding.
- Added experimental `dave` feature support for DAVE/MLS frame parsing, state tracking, and a `davey`/OpenMLS-backed decryptor hook. Live Discord DAVE interoperability still requires real voice gateway transition testing.
- Changed the HTTP User-Agent version to use `env!("CARGO_PKG_VERSION")` so future package versions no longer need a hard-coded request-header update.
- Updated README, USAGE, Docsify docs, and the `discordrs-dev` Codex skill guidance for the `1.1.0` public surface.

## 1.0.0

- **BREAKING**: Removed the legacy raw `RestClient` convenience methods (`send_message`, `edit_message`, `create_dm_channel`, `create_interaction_response`, and `bulk_overwrite_global_commands`) from the public API. The typed `RestClient` surface is now the supported path, and internal JSON helpers remain crate-private.
- **BREAKING**: Builder implementation submodules are now private. Import builders from `discordrs::builders::{...}` or the crate root re-exports instead of deeper paths such as `discordrs::builders::modal::*`.
- **BREAKING**: `ApplicationCommand` no longer implements `DiscordModel`. Use `ApplicationCommand::id_opt()` and `ApplicationCommand::created_at()` for optional-ID command values.
- Changed gateway event processing to preserve ordering through a dedicated event processor instead of unbounded per-event task spawning.
- Changed unsupported gateway `compress=zlib-stream` configuration to be stripped from normalized URLs so the runtime no longer advertised a mode it could not process.
- Changed interaction request verification to reject stale or future timestamps outside a five-minute freshness window.
- Hardened token-authenticated callback/webhook HTTP paths by rejecting unsafe path segments and omitting bot `Authorization` headers from `/interactions/...` and `/webhooks/...` requests.
- Fixed gateway Identify/Resume payloads to use the raw Discord token instead of an HTTP-style `Bot ` prefix.
- Fixed REST error typing so Discord API failures surface as `DiscordError::Api` / `DiscordError::RateLimit` instead of collapsing into model errors.
- Fixed typed command and autocomplete interactions to preserve nested option `value` / `focused` input data through `CommandInteractionOption`.
- Changed voice state handling to clear stale runtime/session state on disconnect and endpoint loss.
- Added README / USAGE migration notes for the tightened public API surface and canonical replacement paths.

## 0.3.1

- Added safer builder serialization for buttons and select menus so invalid Discord payload combinations are normalized before send.
- Added modal `FILE_UPLOAD` parsing support and `V2ModalSubmission::get_file_values()`.
- Added explicit follow-up webhook methods that accept `application_id` and fail early when it is missing.
- Added `try_interactions_endpoint()` for startup-time Discord public-key validation.
- Changed gateway reconnect behavior to preserve required resume query parameters and stop retrying documented terminal close codes forever.

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
