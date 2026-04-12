use serde_json::Value;

use crate::error::DiscordError;
use crate::model::{
    AutocompleteInteraction, ChatInputCommandInteraction, CommandInteractionData,
    ComponentInteraction, ComponentInteractionData, Interaction, InteractionContextData,
    MessageContextMenuInteraction, ModalSubmitInteraction, PingInteraction, Snowflake,
    UserContextMenuInteraction,
};
use crate::types::invalid_data_error;

use super::modal::{parse_modal_submission, V2ModalSubmission};
use super::{
    optional_string_field, optional_string_values_field, required_object_field,
    required_string_field, required_u8_field, value_to_string, value_to_u8,
};

#[derive(Clone, Debug)]
pub enum RawInteraction {
    Ping,
    Command {
        id: Option<String>,
        name: Option<String>,
        command_type: Option<u8>,
        data: Value,
    },
    Autocomplete {
        id: Option<String>,
        name: Option<String>,
        command_type: Option<u8>,
        data: Value,
    },
    Component {
        custom_id: Option<String>,
        component_type: Option<u8>,
        values: Vec<String>,
        data: Value,
    },
    ModalSubmit(V2ModalSubmission),
}

#[derive(Clone, Debug)]
pub struct InteractionContext {
    pub id: String,
    pub token: String,
    pub application_id: String,
    pub guild_id: Option<String>,
    pub channel_id: Option<String>,
    pub user_id: Option<String>,
    pub raw: Value,
}

pub fn parse_interaction_context(raw: &Value) -> Result<InteractionContext, DiscordError> {
    let user_id = raw
        .get("member")
        .and_then(|member| member.get("user"))
        .and_then(|user| user.get("id"))
        .and_then(value_to_string)
        .or_else(|| {
            raw.get("user")
                .and_then(|user| user.get("id"))
                .and_then(value_to_string)
        });

    Ok(InteractionContext {
        id: required_string_field(raw, "id", "interaction")?,
        token: required_string_field(raw, "token", "interaction")?,
        application_id: required_string_field(raw, "application_id", "interaction")?,
        guild_id: optional_string_field(raw, "guild_id"),
        channel_id: optional_string_field(raw, "channel_id"),
        user_id,
        raw: raw.clone(),
    })
}

pub fn parse_raw_interaction(raw: &Value) -> Result<RawInteraction, DiscordError> {
    let interaction_type = required_u8_field(raw, "type", "interaction")?;

    match interaction_type {
        1 => Ok(RawInteraction::Ping),
        2 => {
            let data = required_object_field(raw, "data", "interaction")?.clone();
            Ok(RawInteraction::Command {
                id: optional_string_field(&data, "id"),
                name: optional_string_field(&data, "name"),
                command_type: data.get("type").and_then(value_to_u8),
                data,
            })
        }
        4 => {
            let data = required_object_field(raw, "data", "interaction")?.clone();
            Ok(RawInteraction::Autocomplete {
                id: optional_string_field(&data, "id"),
                name: optional_string_field(&data, "name"),
                command_type: data.get("type").and_then(value_to_u8),
                data,
            })
        }
        3 => {
            let data = required_object_field(raw, "data", "interaction")?.clone();
            let values = optional_string_values_field(&data, "values", "component_data")?
                .unwrap_or_default();

            Ok(RawInteraction::Component {
                custom_id: optional_string_field(&data, "custom_id"),
                component_type: data.get("component_type").and_then(value_to_u8),
                values,
                data,
            })
        }
        5 => Ok(RawInteraction::ModalSubmit(parse_modal_submission(raw)?)),
        _ => Err(invalid_data_error(format!(
            "unsupported interaction type: {interaction_type}"
        ))),
    }
}

pub fn parse_interaction(raw: &Value) -> Result<Interaction, DiscordError> {
    let interaction_type = required_u8_field(raw, "type", "interaction")?;
    let context = parse_typed_interaction_context(raw)?;

    let interaction = match interaction_type {
        1 => Interaction::Ping(PingInteraction { context }),
        2 => {
            let data = parse_command_interaction_data(raw)?;
            match data.kind.unwrap_or(1) {
                1 => Interaction::ChatInputCommand(ChatInputCommandInteraction { context, data }),
                2 => Interaction::UserContextMenu(UserContextMenuInteraction { context, data }),
                3 => {
                    Interaction::MessageContextMenu(MessageContextMenuInteraction { context, data })
                }
                kind => Interaction::Unknown {
                    context,
                    kind,
                    raw_data: raw.get("data").cloned().unwrap_or(Value::Null),
                },
            }
        }
        3 => Interaction::Component(ComponentInteraction {
            context,
            data: parse_component_interaction_data(raw)?,
        }),
        4 => Interaction::Autocomplete(AutocompleteInteraction {
            context,
            data: parse_command_interaction_data(raw)?,
        }),
        5 => Interaction::ModalSubmit(ModalSubmitInteraction {
            context,
            submission: parse_modal_submission(raw)?,
        }),
        kind => Interaction::Unknown {
            context,
            kind,
            raw_data: raw.get("data").cloned().unwrap_or(Value::Null),
        },
    };

    Ok(interaction)
}

fn parse_typed_interaction_context(raw: &Value) -> Result<InteractionContextData, DiscordError> {
    let member = raw
        .get("member")
        .cloned()
        .map(serde_json::from_value)
        .transpose()?;
    let user = raw
        .get("user")
        .cloned()
        .map(serde_json::from_value)
        .transpose()?
        .or_else(|| {
            member
                .as_ref()
                .and_then(|member: &crate::model::Member| member.user.clone())
        });

    Ok(InteractionContextData {
        id: Snowflake::from(required_string_field(raw, "id", "interaction")?),
        application_id: Snowflake::from(required_string_field(
            raw,
            "application_id",
            "interaction",
        )?),
        token: required_string_field(raw, "token", "interaction")?,
        guild_id: optional_string_field(raw, "guild_id").map(Snowflake::from),
        channel_id: optional_string_field(raw, "channel_id").map(Snowflake::from),
        user,
        member,
        app_permissions: raw
            .get("app_permissions")
            .cloned()
            .map(serde_json::from_value)
            .transpose()?,
        locale: optional_string_field(raw, "locale"),
        guild_locale: optional_string_field(raw, "guild_locale"),
    })
}

fn parse_command_interaction_data(raw: &Value) -> Result<CommandInteractionData, DiscordError> {
    let data = required_object_field(raw, "data", "interaction")?.clone();
    let options = data
        .get("options")
        .cloned()
        .map(serde_json::from_value)
        .transpose()?
        .unwrap_or_default();
    Ok(CommandInteractionData {
        id: optional_string_field(&data, "id").map(Snowflake::from),
        name: optional_string_field(&data, "name"),
        kind: data.get("type").and_then(value_to_u8),
        options,
        resolved: data.get("resolved").cloned(),
    })
}

fn parse_component_interaction_data(raw: &Value) -> Result<ComponentInteractionData, DiscordError> {
    let data = required_object_field(raw, "data", "interaction")?.clone();
    Ok(ComponentInteractionData {
        custom_id: required_string_field(&data, "custom_id", "component_data")?,
        component_type: required_u8_field(&data, "component_type", "component_data")?,
        values: optional_string_values_field(&data, "values", "component_data")?
            .unwrap_or_default(),
    })
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{
        parse_interaction, parse_interaction_context, parse_raw_interaction, RawInteraction,
    };
    use crate::model::Interaction;

    #[test]
    fn parse_raw_interaction_supports_autocomplete() {
        let interaction = parse_raw_interaction(&json!({
            "id": "1",
            "application_id": "2",
            "token": "token",
            "type": 4,
            "data": {
                "id": "3",
                "name": "ticket",
                "type": 1,
                "options": []
            }
        }))
        .unwrap();

        match interaction {
            RawInteraction::Autocomplete {
                id,
                name,
                command_type,
                ..
            } => {
                assert_eq!(id.as_deref(), Some("3"));
                assert_eq!(name.as_deref(), Some("ticket"));
                assert_eq!(command_type, Some(1));
            }
            other => panic!("unexpected raw interaction: {other:?}"),
        }
    }

    #[test]
    fn parse_interaction_preserves_command_option_values_and_focus() {
        let interaction = parse_interaction(&json!({
            "id": "1",
            "application_id": "2",
            "token": "token",
            "type": 4,
            "data": {
                "id": "3",
                "name": "ticket",
                "type": 1,
                "options": [{
                    "type": 3,
                    "name": "topic",
                    "value": "billing",
                    "focused": true
                }, {
                    "type": 1,
                    "name": "nested",
                    "options": [{
                        "type": 4,
                        "name": "priority",
                        "value": 2
                    }]
                }]
            }
        }))
        .unwrap();

        match interaction {
            Interaction::Autocomplete(interaction) => {
                assert_eq!(interaction.data.options.len(), 2);
                assert_eq!(interaction.data.options[0].name, "topic");
                assert_eq!(interaction.data.options[0].value, Some(json!("billing")));
                assert!(interaction.data.options[0].is_focused());
                assert_eq!(interaction.data.options[1].name, "nested");
                assert_eq!(interaction.data.options[1].options.len(), 1);
                assert_eq!(interaction.data.options[1].options[0].value, Some(json!(2)));
            }
            other => panic!("unexpected typed interaction: {other:?}"),
        }
    }

    #[test]
    fn parse_interaction_context_prefers_member_user_and_keeps_raw_payload() {
        let raw = json!({
            "id": "1",
            "application_id": "2",
            "token": "token",
            "guild_id": "3",
            "channel_id": "4",
            "member": {
                "user": {
                    "id": "5",
                    "username": "discordrs"
                }
            }
        });

        let context = parse_interaction_context(&raw).unwrap();
        assert_eq!(context.id, "1");
        assert_eq!(context.application_id, "2");
        assert_eq!(context.guild_id.as_deref(), Some("3"));
        assert_eq!(context.channel_id.as_deref(), Some("4"));
        assert_eq!(context.user_id.as_deref(), Some("5"));
        assert_eq!(context.raw, raw);
    }

    #[test]
    fn parse_raw_interaction_covers_ping_component_modal_and_unknown_types() {
        assert!(matches!(
            parse_raw_interaction(&json!({
                "id": "1",
                "application_id": "2",
                "token": "token",
                "type": 1
            }))
            .unwrap(),
            RawInteraction::Ping
        ));

        match parse_raw_interaction(&json!({
            "id": "1",
            "application_id": "2",
            "token": "token",
            "type": 3,
            "data": {
                "custom_id": "button",
                "component_type": 2,
                "values": ["first", "second"]
            }
        }))
        .unwrap()
        {
            RawInteraction::Component {
                custom_id,
                component_type,
                values,
                ..
            } => {
                assert_eq!(custom_id.as_deref(), Some("button"));
                assert_eq!(component_type, Some(2));
                assert_eq!(values, vec!["first".to_string(), "second".to_string()]);
            }
            other => panic!("unexpected component interaction: {other:?}"),
        }

        match parse_raw_interaction(&json!({
            "id": "1",
            "application_id": "2",
            "token": "token",
            "type": 5,
            "data": {
                "custom_id": "modal",
                "components": []
            }
        }))
        .unwrap()
        {
            RawInteraction::ModalSubmit(submission) => {
                assert_eq!(submission.custom_id, "modal");
                assert!(submission.components.is_empty());
            }
            other => panic!("unexpected modal submission: {other:?}"),
        }

        let error = parse_raw_interaction(&json!({
            "id": "1",
            "application_id": "2",
            "token": "token",
            "type": 99
        }))
        .unwrap_err();
        assert!(error.to_string().contains("unsupported interaction type"));
    }

    #[test]
    fn parse_interaction_covers_component_and_unknown_command_variants() {
        match parse_interaction(&json!({
            "id": "1",
            "application_id": "2",
            "token": "token",
            "type": 3,
            "data": {
                "custom_id": "button",
                "component_type": 2,
                "values": ["accepted"]
            }
        }))
        .unwrap()
        {
            Interaction::Component(component) => {
                assert_eq!(component.data.custom_id, "button");
                assert_eq!(component.data.component_type, 2);
                assert_eq!(component.data.values, vec!["accepted".to_string()]);
            }
            other => panic!("unexpected typed component: {other:?}"),
        }

        match parse_interaction(&json!({
            "id": "1",
            "application_id": "2",
            "token": "token",
            "type": 2,
            "data": {
                "type": 99,
                "name": "unknown-command"
            }
        }))
        .unwrap()
        {
            Interaction::Unknown { kind, raw_data, .. } => {
                assert_eq!(kind, 99);
                assert_eq!(raw_data["name"], json!("unknown-command"));
            }
            other => panic!("unexpected unknown interaction: {other:?}"),
        }
    }

    #[test]
    fn parse_interaction_context_falls_back_to_top_level_user() {
        let raw = json!({
            "id": "10",
            "application_id": "20",
            "token": "token",
            "user": {
                "id": "30",
                "username": "fallback-user"
            }
        });

        let context = parse_interaction_context(&raw).unwrap();
        assert_eq!(context.id, "10");
        assert_eq!(context.application_id, "20");
        assert_eq!(context.guild_id, None);
        assert_eq!(context.channel_id, None);
        assert_eq!(context.user_id.as_deref(), Some("30"));
        assert_eq!(context.raw, raw);
    }

    #[test]
    fn parse_raw_interaction_covers_command_defaults_and_component_value_fallback() {
        match parse_raw_interaction(&json!({
            "id": "1",
            "application_id": "2",
            "token": "token",
            "type": 2,
            "data": {}
        }))
        .unwrap()
        {
            RawInteraction::Command {
                id,
                name,
                command_type,
                data,
            } => {
                assert_eq!(id, None);
                assert_eq!(name, None);
                assert_eq!(command_type, None);
                assert_eq!(data, json!({}));
            }
            other => panic!("unexpected command interaction: {other:?}"),
        }

        match parse_raw_interaction(&json!({
            "id": "1",
            "application_id": "2",
            "token": "token",
            "type": 3,
            "data": {
                "custom_id": "menu",
                "component_type": 3
            }
        }))
        .unwrap()
        {
            RawInteraction::Component { values, data, .. } => {
                assert!(values.is_empty());
                assert_eq!(data["custom_id"], json!("menu"));
            }
            other => panic!("unexpected component interaction: {other:?}"),
        }
    }

    #[test]
    fn parse_interaction_covers_ping_command_variants_modal_and_unknown_type() {
        match parse_interaction(&json!({
            "id": "1",
            "application_id": "2",
            "token": "token",
            "type": 1,
            "member": {
                "user": {
                    "id": "9",
                    "username": "member-user"
                }
            },
            "app_permissions": "2048",
            "locale": "en-US",
            "guild_locale": "ko"
        }))
        .unwrap()
        {
            Interaction::Ping(ping) => {
                assert_eq!(ping.context.id.as_u64(), Some(1));
                assert_eq!(ping.context.application_id.as_u64(), Some(2));
                assert_eq!(ping.context.user.unwrap().id.as_u64(), Some(9));
                assert_eq!(ping.context.app_permissions.unwrap().bits(), 2048);
                assert_eq!(ping.context.locale.as_deref(), Some("en-US"));
                assert_eq!(ping.context.guild_locale.as_deref(), Some("ko"));
            }
            other => panic!("unexpected ping interaction: {other:?}"),
        }

        match parse_interaction(&json!({
            "id": "1",
            "application_id": "2",
            "token": "token",
            "type": 2,
            "data": {
                "id": "5",
                "name": "deploy",
                "type": 1,
                "resolved": {
                    "users": {
                        "42": {
                            "id": "42",
                            "username": "resolved-user"
                        }
                    }
                }
            }
        }))
        .unwrap()
        {
            Interaction::ChatInputCommand(command) => {
                assert_eq!(command.data.id.unwrap().as_u64(), Some(5));
                assert_eq!(command.data.name.as_deref(), Some("deploy"));
                assert_eq!(command.data.kind, Some(1));
                assert!(command.data.options.is_empty());
                assert_eq!(
                    command.data.resolved.unwrap()["users"]["42"]["username"],
                    json!("resolved-user")
                );
            }
            other => panic!("unexpected chat input interaction: {other:?}"),
        }

        assert!(matches!(
            parse_interaction(&json!({
                "id": "1",
                "application_id": "2",
                "token": "token",
                "type": 2,
                "data": {
                    "type": 2
                }
            }))
            .unwrap(),
            Interaction::UserContextMenu(_)
        ));

        assert!(matches!(
            parse_interaction(&json!({
                "id": "1",
                "application_id": "2",
                "token": "token",
                "type": 2,
                "data": {
                    "type": 3
                }
            }))
            .unwrap(),
            Interaction::MessageContextMenu(_)
        ));

        match parse_interaction(&json!({
            "id": "1",
            "application_id": "2",
            "token": "token",
            "type": 5,
            "data": {
                "custom_id": "feedback",
                "components": [{
                    "type": 1,
                    "components": [{
                        "type": 4,
                        "custom_id": "summary",
                        "value": "works"
                    }]
                }]
            }
        }))
        .unwrap()
        {
            Interaction::ModalSubmit(modal) => {
                assert_eq!(modal.submission.custom_id, "feedback");
                assert_eq!(modal.submission.get_text("summary"), Some("works"));
            }
            other => panic!("unexpected modal interaction: {other:?}"),
        }

        match parse_interaction(&json!({
            "id": "1",
            "application_id": "2",
            "token": "token",
            "type": 99
        }))
        .unwrap()
        {
            Interaction::Unknown {
                kind,
                raw_data,
                context,
            } => {
                assert_eq!(kind, 99);
                assert_eq!(raw_data, json!(null));
                assert_eq!(context.id.as_u64(), Some(1));
            }
            other => panic!("unexpected unknown interaction: {other:?}"),
        }
    }

    #[test]
    fn parse_interaction_reports_component_value_errors() {
        let error = parse_raw_interaction(&json!({
            "id": "1",
            "application_id": "2",
            "token": "token",
            "type": 3,
            "data": {
                "custom_id": "menu",
                "component_type": 3,
                "values": [{}]
            }
        }))
        .unwrap_err();
        assert!(error
            .to_string()
            .contains("component_data.values must contain strings"));

        let error = parse_interaction(&json!({
            "id": "1",
            "application_id": "2",
            "token": "token",
            "type": 3,
            "data": {
                "component_type": 2
            }
        }))
        .unwrap_err();
        assert!(error
            .to_string()
            .contains("missing or invalid component_data.custom_id"));
    }
}
