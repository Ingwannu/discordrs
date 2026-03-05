# Interaction Flow

This is the typical command/component/modal response flow.

## 1. Parse Context and Type

```rust
let ctx = parse_interaction_context(payload)?;
let raw = parse_raw_interaction(payload)?;
```

## 2. Route by `RawInteraction`

```rust
match raw {
    RawInteraction::Command { name, .. } => { /* slash */ }
    RawInteraction::Component { custom_id, .. } => { /* button/select */ }
    RawInteraction::ModalSubmit(submission) => { /* modal */ }
    RawInteraction::Ping => { /* endpoint pong */ }
}
```

## 3. Respond with Helpers

- `respond_with_container(...)` for slash commands
- `respond_component_with_container(...)` for components
- `respond_modal_with_container(...)` for modal submissions
- `respond_with_modal(...)` to open a modal

## 4. Ephemeral vs Public

Each helper supports the ephemeral flag so you can keep operational responses private.

## End-to-End Example

```rust
if let RawInteraction::Command { name, .. } = parse_raw_interaction(payload)? {
    if name.as_deref() == Some("hello") {
        let c = create_container("Hello", "Command processed.", vec![], None);
        respond_with_container(http, &ctx.id, &ctx.token, c, true).await?;
    }
}
```
