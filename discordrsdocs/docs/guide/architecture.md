# Architecture

`discord.rs` is split into focused modules so you can compose only what you need.

```mermaid
flowchart LR
  subgraph App[Your Application]
    H[EventHandler]
    IH[InteractionHandler]
    C[Collectors]
  end

  subgraph Runtime[Gateway Runtime]
    CL[Client]
    EV[Event]
    GC[GatewayClient]
  end

  subgraph REST[HTTP Layer]
    RC[RestClient]
    HP[Helpers]
  end

  subgraph Build[Builders]
    BV2[Components V2 Builders]
    MB[Modal Builders]
  end

  subgraph Typed[Typed Surface]
    M[Models]
    CMD[Command Builders]
    CACHE[Cache Managers]
  end

  subgraph Endpoint[Interactions Endpoint]
    AX[Axum Router + Ed25519 Verify]
  end

  H --> CL --> GC
  GC --> EV
  CL --> RC
  C --> EV
  CL --> CACHE
  IH --> AX --> HP
  HP --> RC
  HP --> BV2
  AX --> M
  CMD --> RC
```

## Module Layout

- `src/builders/`: fluent payload builders for Components V2 + modals
- `src/model.rs`: typed Discord models, interactions, and ID helpers
- `src/command.rs`: typed slash/user/message command builders
- `src/gateway/`: websocket runtime, heartbeat/resume, typed event dispatch
- `src/http.rs`: `RestClient` and compatibility `DiscordHttpClient`
- `src/cache.rs`: opt-in cache handle and manager types
- `src/collector.rs`: opt-in async collectors
- `src/parsers/`: raw + typed interaction parsing helpers
- `src/helpers.rs`: high-level reply helpers for interaction flows
- `src/interactions.rs`: HTTP endpoint mode with signature verification

## Runtime Patterns

- Gateway mode: maintain websocket session, handle typed `Event`, call `RestClient` or managers when needed
- Endpoint mode: receive signed interaction payloads, parse with `parse_interaction(...)`, respond with helpers
- Hybrid mode: use both for richer operational workflows
- Sharding and voice are planned as dedicated layers above the current runtime, not mixed into the base `Client`
