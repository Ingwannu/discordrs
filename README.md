# discordrs

`discordrs` is a Rust library that provides:

- Discord Components V2 builders (`Container`, `TextDisplay`, `MediaGallery`, `Section`, `SelectMenu`, etc.)
- Modal builders and raw interaction response helpers
- Modal `Radio Group`, `Checkbox Group`, and `Checkbox` component builders
- Convenience helpers for sending/editing/followup messages with Components V2

## Install

```toml
[dependencies]
discordrs = "0.1.1"
```

## Modal Example (Radio/Checkbox)

```rust
use discordrs::{
    CheckboxBuilder, CheckboxGroupBuilder, ModalBuilder, RadioGroupBuilder, SelectOption,
};

let modal = ModalBuilder::new("preferences_modal", "Preferences")
    .add_radio_group(
        "Theme",
        Some("Pick one"),
        RadioGroupBuilder::new("theme")
            .add_option(SelectOption::new("Light", "light"))
            .add_option(SelectOption::new("Dark", "dark"))
            .required(true),
    )
    .add_checkbox_group(
        "Notifications",
        Some("Choose any"),
        CheckboxGroupBuilder::new("notify_channels")
            .add_option(SelectOption::new("Email", "email"))
            .add_option(SelectOption::new("Push", "push"))
            .min_values(0)
            .max_values(2),
    )
    .add_checkbox(
        "Agree to Terms",
        None,
        CheckboxBuilder::new("agree_terms").required(true),
    );
```

## Quick Example

```rust
use discordrs::{button_style, create_container, send_container_message, ButtonConfig};
use serenity::http::Http;
use serenity::all::ChannelId;

async fn send_panel(http: &Http, channel_id: ChannelId) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let buttons = vec![
        ButtonConfig::new("open_ticket", "티켓 열기").style(button_style::PRIMARY).emoji("🎫")
    ];

    let container = create_container(
        "지원 패널",
        "아래 버튼을 눌러 티켓을 생성하세요.",
        buttons,
        None,
    );

    send_container_message(http, channel_id, container).await?;
    Ok(())
}
```

## Slash Command Registration

`discordrs` includes both payload builders and ready-to-use bulk registration helpers.

```rust
use discordrs::{
    CommandOptionBuilder, CommandOptionChoice, SlashCommandBuilder, SlashCommandScope,
    SlashCommandSet,
};
use serenity::all::GuildId;
use serenity::http::Http;

async fn register(http: &Http, guild_id: GuildId) -> Result<(), discordrs::Error> {
    let mut commands = SlashCommandSet::new()
        .with_command(
            SlashCommandBuilder::new("ping", "Latency check")
                .dm_permission(false)
                .add_option(
                    CommandOptionBuilder::string("target", "who to ping")
                        .required(true)
                        .add_choice(CommandOptionChoice::string("all", "all")),
                ),
        )
        .with_commands(vec![SlashCommandBuilder::new("about", "About this bot")]);

    // Name-based upsert/remove helpers for ergonomic command management
    commands.set_command(SlashCommandBuilder::new("ping", "Updated latency check"));
    let _removed = commands.remove("about");
    assert!(commands.contains("ping"));

    // Non-consuming helpers (useful when you want to re-use the same set)
    let payload = commands.payload();
    assert_eq!(payload.len(), 1);

    // Unified scope-based registration API
    let _global = commands.register_ref(http, SlashCommandScope::Global).await?;
    let _guild = commands
        .register_ref(http, SlashCommandScope::Guild(guild_id))
        .await?;
    Ok(())
}
```

## Interaction Dispatch Helper

Use `InteractionRouter` for ergonomic routing by slash command name, component `custom_id`, or prefix patterns.
You can either chain with `on_*` or mutate with `insert_*` methods.
Need upsert/removal semantics? use `set_*` and `remove_*` helpers.

```rust
use discordrs::{dispatch_interaction, dispatch_interaction_match, InteractionRouter};

let router = InteractionRouter::new()
    .on_command("ping", "ping_handler")
    .on_component_prefix("ticket:", "ticket_component_handler")
    .on_modal_prefix("ticket_modal:", "ticket_modal_handler")
    .with_component_fallback("component_fallback_handler");

// inside event handler:
// if let Some(route) = router.resolve_interaction(&interaction) { ... }
// if let Some(m) = router.resolve_interaction_match(&interaction) {
//     println!("matched {:?} by key {}", m.kind, m.key);
// }
// assert!(router.contains_command("ping"));
// You can still use free functions:
// router.set_component_prefix("ticket:", "new_ticket_component_handler");
// router.remove_modal("ticket_modal:legacy");
// dispatch_interaction(&router, &interaction)
```

Routing rules:
- Exact match wins first.
- If no exact match, prefix routes are checked.
- Among prefixes, the longest matching prefix wins.
- If specificity ties (same exact key or same prefix length), the latest inserted route wins.
- If no route matches, per-kind fallback handlers are used when configured.
- Use `set_*` to upsert, `insert_*` to append, and `remove_*` to delete routes.
- Convenience methods are available for each kind: `resolve_command`, `resolve_component`, `resolve_modal`.

For slash command collection ergonomics, `SlashCommandSet` also supports:
- `names()` to iterate command names in insertion order
- `retain(...)` to filter commands in place before registration

## Notes

- This library uses raw Discord HTTP payloads for Components V2 because serenity does not yet model all V2 structures directly.
- For crates.io publication, ensure the package name `discordrs` is available in your account.
