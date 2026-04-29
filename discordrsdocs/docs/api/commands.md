# Commands API

`discord.rs` now exposes typed application command builders instead of relying on raw `Vec<Value>` payload assembly.

## Main Builders

- `SlashCommandBuilder`
- `UserCommandBuilder`
- `MessageCommandBuilder`
- `CommandOptionBuilder`

## Supported Patterns

- slash commands with typed options
- user and message context menu commands
- option choices, ranges, and string length constraints
- autocomplete flags

## Example

```rust
use discordrs::{option_type, CommandOptionBuilder, SlashCommandBuilder};

let command = SlashCommandBuilder::new("ticket", "Create a support ticket")
    .option(
        CommandOptionBuilder::new(option_type::STRING, "topic", "Ticket topic")
            .required(true)
            .autocomplete(true),
    )
    .build();
```

## Registration

- Build the command definition with the typed builders
- Send it with the REST layer instead of hand-written JSON
- Keep raw overwrite calls only for compatibility or uncovered routes
