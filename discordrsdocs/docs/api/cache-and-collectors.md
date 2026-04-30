# Cache and Collectors

These layers are optional. They are meant to improve runtime ergonomics without making the base crate heavy.

## Cache

The `cache` feature is enabled by default in `1.2.1`, so normal installs keep in-memory state for common lookups. `CacheHandle::new()` uses bounded defaults; builds using `default-features = false` keep the cache API available but use empty no-op storage.

Main types:

- `CacheConfig`
- `CacheHandle`
- `GuildManager`
- `ChannelManager`
- `MemberManager`
- `MessageManager`
- `RoleManager`

The managers prefer cache hits and fall back to `RestClient` fetches.

`ClientBuilder::cache_config(...)` and `CacheHandle::with_config(...)` let long-running bots tune message, presence, and member storage by size and TTL. Size limits are enforced on insert, and TTL limits are purged on insert, explicit `purge_expired()`, and cache reads for the affected entity type. Use `CacheConfig::unbounded()` only when retaining all cached gateway data is intentional.

Use `CacheHandle::is_enabled()` when reusable code needs to detect whether it was compiled with real cache storage.

```rust
use std::time::Duration;
use discordrs::{gateway_intents, CacheConfig, Client};

let client = Client::builder("bot-token", gateway_intents::GUILD_MESSAGES)
    .cache_config(
        CacheConfig::default()
            .max_messages_per_channel(100)
            .max_total_messages(10_000)
            .message_ttl(Duration::from_secs(60 * 60))
            .presence_ttl(Duration::from_secs(10 * 60))
            .max_members_per_guild(25_000),
    );
```

## Collectors

Enable the `collectors` feature when the bot needs event-driven waiting flows.

Main types:

- `CollectorHub`
- `MessageCollector`
- `InteractionCollector`
- `ComponentCollector`
- `ModalCollector`

Collectors subscribe to typed `Event` values and let handlers wait for the next matching runtime event.

## Typical Use

- cache for hot-path lookups
- collectors for button, modal, or follow-up message flows
- both together for stateful multi-step bots
