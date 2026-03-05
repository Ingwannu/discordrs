# HTTP and Helpers API

## `DiscordHttpClient`

`DiscordHttpClient` is a Discord REST v10 wrapper with retry behavior for `429 Too Many Requests`.

Common operations include:

- send/edit/delete messages
- interaction responses
- follow-up messages
- command registration endpoints

## Helper Functions

For Components V2 and interaction response workflows, use:

- `send_container_message(...)`
- `respond_with_container(...)`
- `respond_component_with_container(...)`
- `respond_modal_with_container(...)`
- `respond_with_modal(...)`

## Example

```rust
let c = create_container("Notice", "Done", vec![], None);
respond_with_container(http, &ctx.id, &ctx.token, c, true).await?;
```

## Recommended Pattern

- Build payloads with builders
- Route payloads with parsers
- Send/ack with helper functions

This keeps handler code short and consistent.
