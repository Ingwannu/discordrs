# Interactions Endpoint API

Use this mode when Discord sends interaction callbacks to your HTTP server.

## Feature Flag

```toml
[dependencies]
discordrs = { version = "0.3.0", features = ["interactions"] }
```

## Capabilities

- Ed25519 request signature verification
- Axum routing helpers
- Typed interaction parsing
- Structured response encoding (`Pong`, message, deferred, modal, update)

## Typical Flow

1. Receive `/interactions` HTTP request
2. Verify signature headers/body
3. Parse into interaction type/context
4. Execute handler logic
5. Return interaction response JSON

## When to Use

- Slash-command-first bots
- serverless or HTTP-native infrastructure
- apps where websocket Gateway runtime is not preferred
