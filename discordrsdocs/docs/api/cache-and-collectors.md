# Cache and Collectors

These layers are optional. They are meant to improve runtime ergonomics without making the base crate heavy.

## Cache

Enable the `cache` feature when the bot needs in-memory state for common lookups.

Main types:

- `CacheConfig`
- `CacheHandle`
- `GuildManager`
- `ChannelManager`
- `MemberManager`
- `MessageManager`
- `RoleManager`

The managers prefer cache hits and fall back to `RestClient` fetches.

`CacheHandle::with_config(...)` lets long-running bots bound message, presence, and member storage by size and TTL. Size limits are enforced on insert, and TTL limits are purged on insert, explicit `purge_expired()`, and cache reads for the affected entity type.

```rust
use std::time::Duration;
use discordrs::{CacheConfig, CacheHandle};

let cache = CacheHandle::with_config(
    CacheConfig::unbounded()
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
