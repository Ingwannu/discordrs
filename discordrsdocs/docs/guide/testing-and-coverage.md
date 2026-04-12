# Testing And Coverage

This guide records the local testing and coverage patterns that worked well while raising
`discordrs` coverage.

## Goals

- Keep tests hermetic: no live Discord traffic.
- Prefer unit tests first, local harness tests second.
- Raise real `cargo llvm-cov` coverage, not only filtered CI coverage.

## Verification Commands

Run the full verification set after meaningful changes:

```bash
cargo test --all-features
cargo check --all-features --all-targets
cargo clippy --all-features --tests -- -D warnings
cargo fmt --all -- --check
cargo llvm-cov --all-features --summary-only
```

If `cargo llvm-cov` is not installed:

```bash
rustup component add llvm-tools-preview
cargo install cargo-llvm-cov
```

## High-Yield Test Seams

### Local HTTP Harness

`src/http.rs` exposes a test-only base URL seam via `RestClient::new_with_base_url(...)`.
Use it with a local `TcpListener` and scripted raw HTTP responses.

Best targets:

- REST wrapper methods that only differ by path, method, or body
- rate-limit state transitions
- auth-header behavior
- validation-before-network error paths

### Local WebSocket Harness

For `src/gateway/client.rs` and `src/voice_runtime.rs`, use:

- `tokio::net::TcpListener`
- `tokio_tungstenite::accept_async`
- scripted HELLO / READY / INVALID_SESSION / RECONNECT payloads

This is enough to cover identify, resume, reconnect, heartbeat, READY propagation, and voice
runtime handshake logic without using Discord.

### Validation-First Helper Tests

`src/helpers.rs` and parts of `src/http.rs` often validate tokens, application ids, and message ids
before making a request. These are cheap coverage wins and should be preferred over external mocks.

### Event Decode Tables

`src/event.rs` coverage moves fastest when you add tiny valid payloads per event variant instead of
building large shared fixtures.

### Runtime State Assertions

For `gateway/bot`, `cache`, `collector`, `sharding`, `voice`, and `voice_runtime`:

1. prefill state
2. apply one event or command
3. assert only the intended mutation
4. assert neighboring state stayed untouched

## Coverage Order That Worked Well

1. pure builders / bitfields / types / parser helpers
2. event decode tables
3. `http.rs` local server harness
4. `gateway/client.rs` local websocket harness
5. `gateway/bot.rs` stateful helper and dispatch tests
6. `voice.rs` and `voice_runtime.rs`
7. collector and sharding cleanup

## Things That Were Not Worth It

- real Discord network tests
- huge end-to-end fixtures when a minimal payload hits the same branch
- stable-toolchain coverage attributes such as `#[coverage(off)]`
- production refactors that exist only to move trivial coverage numbers

## Practical Notes

- Check `cargo llvm-cov --summary-only` often and start with the biggest files.
- Prefer existing `#[cfg(test)]` modules when the target is private helper code.
- Use harness tests only after direct unit tests stop being enough.
- Treat `gateway/bot.rs`, `http.rs`, and `interactions.rs` as the final expensive coverage tiers.
