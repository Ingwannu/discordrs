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

    use super::{parse_interaction, parse_raw_interaction, RawInteraction};
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
}
