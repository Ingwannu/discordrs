# discordrs Usage

`discordrs` is a standalone Discord bot framework for Rust that includes Components V2 builders, a Gateway WebSocket client, and an HTTP client.

## 1. Installation

A common setup for running a Gateway bot runtime is:

```toml
[dependencies]
discordrs = { version = "0.3.0", features = ["gateway"] }
```

You can choose feature flags depending on your use case.

```toml
[dependencies]
# Core only (builders, parsers, HTTP client, helpers)
discordrs = "0.3.0"

# Gateway + bot client runtime
discordrs = { version = "0.3.0", features = ["gateway"] }

# Interactions Endpoint
discordrs = { version = "0.3.0", features = ["interactions"] }

# Both runtime modes
discordrs = { version = "0.3.0", features = ["gateway", "interactions"] }
```

## 2. Start a Bot

```rust
use async_trait::async_trait;
use discordrs::{gateway_intents, BotClient, Context, EventHandler};
use serde_json::Value;

struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn ready(&self, _ctx: Context, ready: Value) {
        println!("READY: {}", ready["user"]["username"]);
    }

    async fn message_create(&self, _ctx: Context, message: Value) {
        println!("MESSAGE_CREATE: {}", message["id"]);
    }
}

#[tokio::main]
async fn main() -> Result<(), discordrs::Error> {
    let token = std::env::var("DISCORD_TOKEN")?;

    BotClient::builder(
        &token,
        gateway_intents::GUILDS | gateway_intents::GUILD_MESSAGES,
    )
    .event_handler(Handler)
    .start()
    .await?;

    Ok(())
}
```

## 3. Send a Container Message to a Channel

```rust
use discordrs::{button_style, create_container, send_container_message, ButtonConfig, DiscordHttpClient};

async fn send_panel(http: &DiscordHttpClient, channel_id: u64) -> Result<(), discordrs::Error> {
    let buttons = vec![
        ButtonConfig::new("ticket_open", "Open Ticket").style(button_style::PRIMARY),
        ButtonConfig::new("ticket_status", "Check Status").style(button_style::SECONDARY),
    ];

    let container = create_container(
        "Support Panel",
        "Use the buttons below to submit a request or check its status.",
        buttons,
        None,
    );

    send_container_message(http, channel_id, container).await?;
    Ok(())
}
```

## 4. Respond to a Slash Command

With `InteractionContext`, you can directly access `interaction_id`, `token`, and `application_id` for replies.

```rust
use discordrs::{
    create_container, parse_interaction_context, parse_raw_interaction,
    respond_with_container, DiscordHttpClient, RawInteraction,
};
use serde_json::Value;

async fn handle_slash(http: &DiscordHttpClient, payload: &Value) -> Result<(), discordrs::Error> {
    let ctx = parse_interaction_context(payload)?;

    if let RawInteraction::Command { name, .. } = parse_raw_interaction(payload)? {
        if name.as_deref() == Some("hello") {
            let container = create_container("Notice", "Your command has been processed.", vec![], None);
            respond_with_container(http, &ctx.id, &ctx.token, container, true).await?;
        }
    }

    Ok(())
}
```

## 5. Respond to Button/Select Interactions

```rust
use discordrs::{
    create_container, parse_interaction_context, respond_component_with_container,
    DiscordHttpClient,
};
use serde_json::Value;

async fn handle_component(http: &DiscordHttpClient, payload: &Value) -> Result<(), discordrs::Error> {
    let ctx = parse_interaction_context(payload)?;
    let container = create_container("Processed", "The selected value has been saved.", vec![], None);

    respond_component_with_container(http, &ctx.id, &ctx.token, container, true).await?;
    Ok(())
}
```

## 6. Respond to Modal Submissions

From `RawInteraction::ModalSubmit`, you can read Radio/Checkbox values from `V2ModalSubmission` without losing V2 structure.

```rust
use discordrs::{
    create_container, parse_interaction_context, parse_raw_interaction,
    respond_modal_with_container, DiscordHttpClient, RawInteraction, V2ModalSubmission,
};
use serde_json::Value;

fn summarize(submission: &V2ModalSubmission) -> String {
    let theme = submission.get_radio_value("theme").unwrap_or("Not selected");
    let channels = submission
        .get_select_values("notify_channels")
        .map(|v| v.join(", "))
        .unwrap_or_else(|| "None".to_string());

    format!("Theme: {theme}, Notifications: {channels}")
}

async fn handle_modal(http: &DiscordHttpClient, payload: &Value) -> Result<(), discordrs::Error> {
    let ctx = parse_interaction_context(payload)?;

    if let RawInteraction::ModalSubmit(submission) = parse_raw_interaction(payload)? {
        let result = summarize(&submission);
        let container = create_container("Modal Processed", &result, vec![], None);
        respond_modal_with_container(http, &ctx.id, &ctx.token, container, true).await?;
    }

    Ok(())
}
```

## 7. Frequently Used APIs

- `DiscordHttpClient::new(token, application_id)`: Create a REST client
- `create_container(...)`: Build a base Components V2 container message
- `send_container_message(...)`: Send a Components V2 message to a channel
- `respond_with_container(...)`: Reply to a Slash Command
- `respond_component_with_container(...)`: Reply to button/select interactions
- `respond_modal_with_container(...)`: Reply to modal submissions
- `respond_with_modal(...)`: Open a modal as a response
- `parse_raw_interaction(...)`: Route by interaction type
- `parse_interaction_context(...)`: Extract shared context required for replies
- `parse_modal_submission(...)`: Parse V2 modal submission data

## 8. Modal Radio/Checkbox Example

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
        "Notification Channels",
        Some("You can select multiple options"),
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

## 9. Notes

- Since v0.3.0, `discordrs` is a standalone framework that provides both Gateway and HTTP capabilities.
- The V2 modal parser preserves component types such as `Label`, `RadioGroup`, `CheckboxGroup`, and `Checkbox`, which helps downstream processing.
- Interaction response helpers can directly use `id` and `token` from `InteractionContext`.
