# Parsers API

Parsers convert raw Discord interaction payloads into typed structures.

## Interaction Parser

Functions:

- `parse_raw_interaction(&Value) -> Result<RawInteraction, Error>`
- `parse_interaction_context(&Value) -> Result<InteractionContext, Error>`

`RawInteraction` variants include:

- `Ping`
- `Command`
- `Component`
- `ModalSubmit`

## Modal Parser

Function:

- `parse_modal_submission(&Value) -> Result<V2ModalSubmission, Error>`

`V2ModalSubmission` preserves V2 component fidelity, including:

- `Label`
- `RadioGroup`
- `CheckboxGroup`
- `Checkbox`
- text/select variants

## Example

```rust
let ctx = parse_interaction_context(payload)?;
match parse_raw_interaction(payload)? {
    RawInteraction::ModalSubmit(submission) => {
        let value = submission.get_radio_value("theme").unwrap_or("Not selected");
        println!("Theme = {value}");
    }
    _ => {}
}
```

## Why Use Parsers

- Less brittle routing than raw JSON indexing
- Common context extraction (`id`, `token`, `application_id`)
- Full-fidelity modal parsing for advanced workflows
