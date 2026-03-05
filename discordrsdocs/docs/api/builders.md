# Builders API

The builders module gives a fluent API for Components V2 and modal payloads.

## Key Submodules

- `components.rs`: low-level message components (`ButtonBuilder`, `SelectMenuBuilder`, `ActionRowBuilder`)
- `container.rs`: high-level container layouts + helper factories
- `media.rs`: sections, thumbnails, media galleries
- `modal.rs`: text input, radio, checkbox, file upload, labels

## Common Pattern

```rust
use discordrs::{ActionRowBuilder, ButtonBuilder, ComponentsV2Message, button_style};

let btn = ButtonBuilder::new()
    .label("Open")
    .style(button_style::PRIMARY)
    .custom_id("open_ticket");

let row = ActionRowBuilder::new().add_button(btn);
let message = ComponentsV2Message::new().add_action_row(row);
```

## Container Helper

```rust
use discordrs::{create_container, ButtonConfig, button_style};

let buttons = vec![
    ButtonConfig::new("ticket_open", "Open Ticket").style(button_style::PRIMARY),
    ButtonConfig::new("ticket_status", "Check Status").style(button_style::SECONDARY),
];

let container = create_container(
    "Support Panel",
    "Use controls below.",
    buttons,
    None,
);
```

## Modal Helper

```rust
use discordrs::{ModalBuilder, RadioGroupBuilder, SelectOption};

let modal = ModalBuilder::new("preferences_modal", "Preferences")
    .add_radio_group(
        "Theme",
        Some("Pick one"),
        RadioGroupBuilder::new("theme")
            .add_option(SelectOption::new("Light", "light"))
            .add_option(SelectOption::new("Dark", "dark"))
            .required(true),
    );
```

## Practical Advice

- Keep `custom_id` values stable; they are routing keys.
- Use small helper factories to avoid repeating layout blocks.
- Prefer module-level re-exports from `discordrs` root when importing builders.
