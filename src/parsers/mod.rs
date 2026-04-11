use serde_json::Value;

use crate::error::DiscordError;
use crate::types::invalid_data_error;

pub mod interaction;
pub mod modal;

pub use interaction::{
    parse_interaction, parse_interaction_context, parse_raw_interaction, InteractionContext,
    RawInteraction,
};
pub use modal::{parse_modal_submission, V2ModalComponent, V2ModalSubmission};

pub(crate) fn value_to_string(value: &Value) -> Option<String> {
    match value {
        Value::String(inner) => Some(inner.clone()),
        Value::Number(inner) => Some(inner.to_string()),
        _ => None,
    }
}

pub(crate) fn value_to_u8(value: &Value) -> Option<u8> {
    match value {
        Value::Number(inner) => inner.as_u64().and_then(|raw| u8::try_from(raw).ok()),
        Value::String(inner) => inner.parse::<u8>().ok(),
        _ => None,
    }
}

pub(crate) fn optional_string_field(value: &Value, field: &str) -> Option<String> {
    value.get(field).and_then(value_to_string)
}

pub(crate) fn required_string_field(
    value: &Value,
    field: &str,
    context: &str,
) -> Result<String, DiscordError> {
    value
        .get(field)
        .and_then(value_to_string)
        .ok_or_else(|| invalid_data_error(format!("missing or invalid {context}.{field}")))
}

pub(crate) fn required_u8_field(
    value: &Value,
    field: &str,
    context: &str,
) -> Result<u8, DiscordError> {
    value
        .get(field)
        .and_then(value_to_u8)
        .ok_or_else(|| invalid_data_error(format!("missing or invalid {context}.{field}")))
}

pub(crate) fn required_object_field<'a>(
    value: &'a Value,
    field: &str,
    context: &str,
) -> Result<&'a Value, DiscordError> {
    match value.get(field) {
        Some(inner) if inner.is_object() => Ok(inner),
        Some(_) => Err(invalid_data_error(format!(
            "{context}.{field} must be an object"
        ))),
        None => Err(invalid_data_error(format!("missing {context}.{field}"))),
    }
}

pub(crate) fn required_array_field<'a>(
    value: &'a Value,
    field: &str,
    context: &str,
) -> Result<&'a [Value], DiscordError> {
    value
        .get(field)
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .ok_or_else(|| invalid_data_error(format!("missing or invalid {context}.{field}")))
}

pub(crate) fn required_bool_field(
    value: &Value,
    field: &str,
    context: &str,
) -> Result<bool, DiscordError> {
    value
        .get(field)
        .and_then(Value::as_bool)
        .ok_or_else(|| invalid_data_error(format!("missing or invalid {context}.{field}")))
}

pub(crate) fn required_string_values_field(
    value: &Value,
    field: &str,
    context: &str,
) -> Result<Vec<String>, DiscordError> {
    let values = value
        .get(field)
        .and_then(Value::as_array)
        .ok_or_else(|| invalid_data_error(format!("missing or invalid {context}.{field}")))?;

    let mut parsed_values = Vec::with_capacity(values.len());
    for entry in values {
        let parsed = value_to_string(entry)
            .ok_or_else(|| invalid_data_error(format!("{context}.{field} must contain strings")))?;
        parsed_values.push(parsed);
    }

    Ok(parsed_values)
}

pub(crate) fn optional_string_values_field(
    value: &Value,
    field: &str,
    context: &str,
) -> Result<Option<Vec<String>>, DiscordError> {
    match value.get(field) {
        Some(Value::Array(values)) => {
            let mut parsed_values = Vec::with_capacity(values.len());
            for entry in values {
                let parsed = value_to_string(entry).ok_or_else(|| {
                    invalid_data_error(format!("{context}.{field} must contain strings"))
                })?;
                parsed_values.push(parsed);
            }
            Ok(Some(parsed_values))
        }
        Some(_) => Err(invalid_data_error(format!(
            "{context}.{field} must be an array"
        ))),
        None => Ok(None),
    }
}
