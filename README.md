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
    slash_commands, CommandOptionBuilder, CommandOptionChoice, SlashCommandBuilder,
    SlashCommandScope, SlashCommandSet,
};
use serenity::all::GuildId;
use serenity::http::Http;

async fn register(http: &Http, guild_id: GuildId) -> Result<(), discordrs::Error> {
    let mut commands = slash_commands![
        SlashCommandBuilder::new("ping", "Latency check")
            .dm_permission(false)
            .add_option(
                CommandOptionBuilder::string("target", "who to ping")
                    .required(true)
                    .add_choice(CommandOptionChoice::string("all", "all")),
            ),
        SlashCommandBuilder::new("about", "About this bot"),
    ];

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

    // Register to multiple scopes with one call
    let registered = commands
        .register_many_ref(
            http,
            [SlashCommandScope::Global, SlashCommandScope::Guild(guild_id)],
        )
        .await?;
    assert_eq!(registered.len(), 2);
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
- `contains_command` / `contains_component` / `contains_modal` check **registered routes only** (fallbacks do not count).
- Use `set_*` to upsert, `insert_*` to append, and `remove_*` to delete routes.
- Convenience methods are available for each kind: `resolve_command`, `resolve_component`, `resolve_modal`.
- Generic fallback helpers are available too: `set_fallback(kind, ...)`, `remove_fallback(kind)`, `has_fallback(kind)`.
- Generic exact-route helpers are available too: `insert(kind, key, ...)`, `set(kind, key, ...)`, `remove(kind, key)`, `contains(kind, key)`.
- Generic prefix-route helpers are available too: `insert_prefix(kind, ...)`, `set_prefix(kind, ...)`, `remove_prefix(kind, ...)`.
- Per-kind route housekeeping helpers: `len_for(kind)`, `has_routes_for(kind)`, `clear_kind(kind)`.

For slash command collection ergonomics, `SlashCommandSet` also supports:
- `slash_commands![ ... ]` macro for concise set construction
- `names()` / `iter()` / `iter_mut()` for ordered traversal
- `get("name")` / `get_mut("name")` for name-based lookup and in-place edits
- `retain(...)` to filter commands in place before registration
- `with_set_commands(...)` / `set_commands(...)` for bulk upsert by command name
- `merge(...)` / `with_merged(...)` to upsert from another `SlashCommandSet`
- `dedup_by_name()` to keep the latest command for duplicate names
- `without("name")` and `remove_where(...)` for concise pre-registration pruning
- `register_many_ref(...)` / `register_many(...)` for multi-scope registration in a single call

## Notes

- This library uses raw Discord HTTP payloads for Components V2 because serenity does not yet model all V2 structures directly.
- For crates.io publication, ensure the package name `discordrs` is available in your account.
