use serde_json::Value;

use crate::types::{invalid_data_error, Error};

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

pub fn parse_interaction_context(raw: &Value) -> Result<InteractionContext, Error> {
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

pub fn parse_raw_interaction(raw: &Value) -> Result<RawInteraction, Error> {
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
