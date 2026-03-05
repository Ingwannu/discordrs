use serde_json::Value;

use crate::constants::component_type;
use crate::types::{invalid_data_error, Error};

use super::{
    required_array_field, required_bool_field, required_object_field, required_string_field,
    required_string_values_field, value_to_u8,
};

fn parse_select_component_data(
    component: &Value,
    context: &str,
) -> Result<(String, Vec<String>), Error> {
    let custom_id = required_string_field(component, "custom_id", context)?;
    let values = required_string_values_field(component, "values", context)?;
    Ok((custom_id, values))
}

fn parse_modal_leaf_component(component: &Value) -> Result<Option<V2ModalComponent>, Error> {
    let Some(component_type) = component.get("type").and_then(value_to_u8) else {
        return Ok(None);
    };

    let parsed = match component_type {
        component_type::TEXT_INPUT => V2ModalComponent::TextInput {
            custom_id: required_string_field(component, "custom_id", "text_input")?,
            value: required_string_field(component, "value", "text_input")?,
        },
        component_type::STRING_SELECT => {
            let (custom_id, values) = parse_select_component_data(component, "string_select")?;
            V2ModalComponent::StringSelect { custom_id, values }
        }
        component_type::USER_SELECT => {
            let (custom_id, values) = parse_select_component_data(component, "user_select")?;
            V2ModalComponent::UserSelect { custom_id, values }
        }
        component_type::ROLE_SELECT => {
            let (custom_id, values) = parse_select_component_data(component, "role_select")?;
            V2ModalComponent::RoleSelect { custom_id, values }
        }
        component_type::CHANNEL_SELECT => {
            let (custom_id, values) = parse_select_component_data(component, "channel_select")?;
            V2ModalComponent::ChannelSelect { custom_id, values }
        }
        component_type::MENTIONABLE_SELECT => {
            let (custom_id, values) = parse_select_component_data(component, "mentionable_select")?;
            V2ModalComponent::MentionableSelect { custom_id, values }
        }
        component_type::RADIO_GROUP => V2ModalComponent::RadioGroup {
            custom_id: required_string_field(component, "custom_id", "radio_group")?,
            value: required_string_field(component, "value", "radio_group")?,
        },
        component_type::CHECKBOX_GROUP => {
            let (custom_id, values) = parse_select_component_data(component, "checkbox_group")?;
            V2ModalComponent::CheckboxGroup { custom_id, values }
        }
        component_type::CHECKBOX => V2ModalComponent::Checkbox {
            custom_id: required_string_field(component, "custom_id", "checkbox")?,
            checked: required_bool_field(component, "checked", "checkbox")?,
        },
        _ => return Ok(None),
    };

    Ok(Some(parsed))
}

fn parse_modal_component_tree(
    component: &Value,
    parsed_components: &mut Vec<V2ModalComponent>,
) -> Result<(), Error> {
    match component.get("type").and_then(value_to_u8) {
        Some(component_type::ACTION_ROW) => {
            let nested_components = required_array_field(component, "components", "action_row")?;
            for nested_component in nested_components {
                parse_modal_component_tree(nested_component, parsed_components)?;
            }
        }
        Some(component_type::LABEL) => {
            let nested_component = component
                .get("component")
                .ok_or_else(|| invalid_data_error("missing label.component"))?;
            parse_modal_component_tree(nested_component, parsed_components)?;
        }
        _ => {
            if let Some(parsed_component) = parse_modal_leaf_component(component)? {
                parsed_components.push(parsed_component);
            }
        }
    }

    Ok(())
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum V2ModalComponent {
    TextInput {
        custom_id: String,
        value: String,
    },
    StringSelect {
        custom_id: String,
        values: Vec<String>,
    },
    UserSelect {
        custom_id: String,
        values: Vec<String>,
    },
    RoleSelect {
        custom_id: String,
        values: Vec<String>,
    },
    ChannelSelect {
        custom_id: String,
        values: Vec<String>,
    },
    MentionableSelect {
        custom_id: String,
        values: Vec<String>,
    },
    RadioGroup {
        custom_id: String,
        value: String,
    },
    CheckboxGroup {
        custom_id: String,
        values: Vec<String>,
    },
    Checkbox {
        custom_id: String,
        checked: bool,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct V2ModalSubmission {
    pub custom_id: String,
    pub components: Vec<V2ModalComponent>,
}

impl V2ModalSubmission {
    pub fn get_text(&self, custom_id: &str) -> Option<&str> {
        self.components
            .iter()
            .find_map(|component| match component {
                V2ModalComponent::TextInput {
                    custom_id: component_custom_id,
                    value,
                } if component_custom_id == custom_id => Some(value.as_str()),
                _ => None,
            })
    }

    pub fn get_select_values(&self, custom_id: &str) -> Option<&[String]> {
        self.components
            .iter()
            .find_map(|component| match component {
                V2ModalComponent::StringSelect {
                    custom_id: component_custom_id,
                    values,
                }
                | V2ModalComponent::UserSelect {
                    custom_id: component_custom_id,
                    values,
                }
                | V2ModalComponent::RoleSelect {
                    custom_id: component_custom_id,
                    values,
                }
                | V2ModalComponent::ChannelSelect {
                    custom_id: component_custom_id,
                    values,
                }
                | V2ModalComponent::MentionableSelect {
                    custom_id: component_custom_id,
                    values,
                }
                | V2ModalComponent::CheckboxGroup {
                    custom_id: component_custom_id,
                    values,
                } if component_custom_id == custom_id => Some(values.as_slice()),
                _ => None,
            })
    }

    pub fn get_radio_value(&self, custom_id: &str) -> Option<&str> {
        self.components
            .iter()
            .find_map(|component| match component {
                V2ModalComponent::RadioGroup {
                    custom_id: component_custom_id,
                    value,
                } if component_custom_id == custom_id => Some(value.as_str()),
                _ => None,
            })
    }
}

pub fn parse_modal_submission(data: &Value) -> Result<V2ModalSubmission, Error> {
    let modal_data = if data.get("custom_id").is_some() && data.get("components").is_some() {
        data
    } else {
        required_object_field(data, "data", "interaction")?
    };

    let custom_id = required_string_field(modal_data, "custom_id", "modal_data")?;
    let components = required_array_field(modal_data, "components", "modal_data")?;

    let mut parsed_components = Vec::new();
    for component in components {
        parse_modal_component_tree(component, &mut parsed_components)?;
    }

    Ok(V2ModalSubmission {
        custom_id,
        components: parsed_components,
    })
}
