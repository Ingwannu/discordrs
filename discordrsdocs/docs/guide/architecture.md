# Architecture

`discordrs` is split into focused modules so you can compose only what you need.

```mermaid
flowchart LR
  subgraph App[Your Application]
    H[EventHandler]
    IH[InteractionHandler]
  end

  subgraph Runtime[Gateway Runtime]
    BC[BotClient]
    GC[GatewayClient]
  end

  subgraph Transport[HTTP Layer]
    HC[DiscordHttpClient]
    HP[Helpers]
  end

  subgraph Build[Builders]
    BV2[Components V2 Builders]
    MB[Modal Builders]
  end

  subgraph Parse[Parsers]
    IP[Interaction Parser]
    MP[V2 Modal Parser]
  end

  subgraph Endpoint[Interactions Endpoint]
    AX[Axum Router + Ed25519 Verify]
  end

  H --> BC --> GC
  BC --> HC
  IH --> AX --> HP
  HP --> HC
  HP --> BV2
  IP --> MP
  AX --> IP
```

## Module Layout

- `src/builders/`: fluent payload builders for Components V2 + modals
- `src/gateway/`: websocket runtime, heartbeat/resume, bot event dispatch
- `src/http.rs`: REST wrapper with 429 retry support
- `src/parsers/`: typed routing/extraction from raw interaction JSON
- `src/helpers.rs`: high-level reply helpers for interaction flows
- `src/interactions.rs`: HTTP endpoint mode with signature verification

## Runtime Patterns

- Gateway mode: maintain websocket session, handle events, call HTTP when needed
- Endpoint mode: receive signed interaction payloads, parse, respond with helper APIs
- Hybrid mode: use both for richer operational workflows
