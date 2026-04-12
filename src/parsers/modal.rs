use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::constants::component_type;
use crate::error::DiscordError;
use crate::types::invalid_data_error;

use super::{
    required_array_field, required_bool_field, required_object_field, required_string_field,
    required_string_values_field, value_to_u8,
};

fn parse_select_component_data(
    component: &Value,
    context: &str,
) -> Result<(String, Vec<String>), DiscordError> {
    let custom_id = required_string_field(component, "custom_id", context)?;
    let values = required_string_values_field(component, "values", context)?;
    Ok((custom_id, values))
}

fn parse_modal_leaf_component(component: &Value) -> Result<Option<V2ModalComponent>, DiscordError> {
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
        component_type::FILE_UPLOAD => {
            let (custom_id, values) = parse_select_component_data(component, "file_upload")?;
            V2ModalComponent::FileUpload { custom_id, values }
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
) -> Result<(), DiscordError> {
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

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
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
    FileUpload {
        custom_id: String,
        values: Vec<String>,
    },
    Checkbox {
        custom_id: String,
        checked: bool,
    },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
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

    pub fn get_file_values(&self, custom_id: &str) -> Option<&[String]> {
        self.components
            .iter()
            .find_map(|component| match component {
                V2ModalComponent::FileUpload {
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

pub fn parse_modal_submission(data: &Value) -> Result<V2ModalSubmission, DiscordError> {
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

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{parse_modal_submission, V2ModalComponent};

    #[test]
    fn parse_modal_submission_preserves_file_upload_values() {
        let payload = json!({
            "type": 5,
            "data": {
                "custom_id": "bug_submit_modal",
                "components": [
                    {
                        "type": 18,
                        "label": "Screenshot",
                        "component": {
                            "type": 19,
                            "custom_id": "file_upload",
                            "values": ["123", "456"]
                        }
                    }
                ]
            }
        });

        let submission = parse_modal_submission(&payload).expect("modal submit should parse");

        assert_eq!(submission.custom_id, "bug_submit_modal");
        assert_eq!(
            submission.get_file_values("file_upload"),
            Some(&["123".to_string(), "456".to_string()][..])
        );
        assert_eq!(
            submission.components,
            vec![V2ModalComponent::FileUpload {
                custom_id: "file_upload".to_string(),
                values: vec!["123".to_string(), "456".to_string()],
            }]
        );
    }

    #[test]
    fn parse_modal_submission_keeps_existing_component_types() {
        let payload = json!({
            "custom_id": "mixed_modal",
            "components": [
                {
                    "type": 1,
                    "components": [
                        {
                            "type": 4,
                            "custom_id": "summary",
                            "value": "details"
                        }
                    ]
                },
                {
                    "type": 18,
                    "label": "Logs",
                    "component": {
                        "type": 19,
                        "custom_id": "attachments",
                        "values": ["789"]
                    }
                }
            ]
        });

        let submission = parse_modal_submission(&payload).expect("mixed modal should parse");

        assert_eq!(submission.get_text("summary"), Some("details"));
        assert_eq!(
            submission.get_file_values("attachments"),
            Some(&["789".to_string()][..])
        );
    }

    #[test]
    fn parse_modal_submission_covers_select_radio_checkbox_and_lookup_helpers() {
        let payload = json!({
            "custom_id": "settings_modal",
            "components": [
                {
                    "type": 18,
                    "label": "Roles",
                    "component": {
                        "type": 6,
                        "custom_id": "roles",
                        "values": ["mod", "admin"]
                    }
                },
                {
                    "type": 18,
                    "label": "Radio",
                    "component": {
                        "type": 21,
                        "custom_id": "visibility",
                        "value": "public"
                    }
                },
                {
                    "type": 18,
                    "label": "Checkbox",
                    "component": {
                        "type": 23,
                        "custom_id": "enabled",
                        "checked": true
                    }
                }
            ]
        });

        let submission = parse_modal_submission(&payload).unwrap();
        assert_eq!(
            submission.get_select_values("roles"),
            Some(&["mod".to_string(), "admin".to_string()][..])
        );
        assert_eq!(submission.get_radio_value("visibility"), Some("public"));
        assert_eq!(submission.get_text("roles"), None);
        assert_eq!(
            submission.components,
            vec![
                V2ModalComponent::RoleSelect {
                    custom_id: "roles".to_string(),
                    values: vec!["mod".to_string(), "admin".to_string()],
                },
                V2ModalComponent::RadioGroup {
                    custom_id: "visibility".to_string(),
                    value: "public".to_string(),
                },
                V2ModalComponent::Checkbox {
                    custom_id: "enabled".to_string(),
                    checked: true,
                }
            ]
        );
    }

    #[test]
    fn parse_modal_submission_ignores_unknown_leaf_components_and_reports_missing_label_component()
    {
        let payload = json!({
            "custom_id": "mixed_modal",
            "components": [
                {
                    "type": 18,
                    "label": "Known",
                    "component": {
                        "type": 4,
                        "custom_id": "summary",
                        "value": "details"
                    }
                },
                {
                    "type": 255,
                    "custom_id": "unknown"
                }
            ]
        });

        let submission = parse_modal_submission(&payload).unwrap();
        assert_eq!(submission.get_text("summary"), Some("details"));
        assert_eq!(submission.components.len(), 1);

        let error = parse_modal_submission(&json!({
            "custom_id": "broken",
            "components": [{
                "type": 18,
                "label": "Missing nested component"
            }]
        }))
        .unwrap_err();
        assert!(error.to_string().contains("missing label.component"));
    }

    #[test]
    fn parse_modal_submission_covers_remaining_select_types_and_accessor_fallbacks() {
        let payload = json!({
            "custom_id": "picker_modal",
            "components": [
                {
                    "type": 1,
                    "components": [
                        {
                            "type": 3,
                            "custom_id": "strings",
                            "values": ["alpha", "beta"]
                        },
                        {
                            "type": 5,
                            "custom_id": "users",
                            "values": ["100"]
                        },
                        {
                            "type": 8,
                            "custom_id": "channels",
                            "values": ["200"]
                        },
                        {
                            "type": 7,
                            "custom_id": "mentionables",
                            "values": ["300", "400"]
                        },
                        {
                            "type": 22,
                            "custom_id": "checks",
                            "values": ["x", "y"]
                        },
                        {
                            "type": "invalid"
                        }
                    ]
                }
            ]
        });

        let submission = parse_modal_submission(&payload).unwrap();
        assert_eq!(
            submission.get_select_values("strings"),
            Some(&["alpha".to_string(), "beta".to_string()][..])
        );
        assert_eq!(
            submission.get_select_values("users"),
            Some(&["100".to_string()][..])
        );
        assert_eq!(
            submission.get_select_values("channels"),
            Some(&["200".to_string()][..])
        );
        assert_eq!(
            submission.get_select_values("mentionables"),
            Some(&["300".to_string(), "400".to_string()][..])
        );
        assert_eq!(
            submission.get_select_values("checks"),
            Some(&["x".to_string(), "y".to_string()][..])
        );
        assert_eq!(submission.get_text("strings"), None);
        assert_eq!(submission.get_radio_value("strings"), None);
        assert_eq!(submission.get_file_values("strings"), None);
        assert_eq!(submission.get_select_values("missing"), None);
        assert_eq!(submission.components.len(), 5);
    }

    #[test]
    fn parse_modal_submission_reports_leaf_validation_errors() {
        let error = parse_modal_submission(&json!({
            "custom_id": "broken_text",
            "components": [{
                "type": 4,
                "custom_id": "summary"
            }]
        }))
        .unwrap_err();
        assert!(error
            .to_string()
            .contains("missing or invalid text_input.value"));

        let error = parse_modal_submission(&json!({
            "custom_id": "broken_checkbox",
            "components": [{
                "type": 23,
                "custom_id": "enabled",
                "checked": "yes"
            }]
        }))
        .unwrap_err();
        assert!(error.to_string().contains("checkbox.checked"));

        let error = parse_modal_submission(&json!({
            "custom_id": "broken_select",
            "components": [{
                "type": 3,
                "custom_id": "strings",
                "values": [{}]
            }]
        }))
        .unwrap_err();
        assert!(error
            .to_string()
            .contains("string_select.values must contain strings"));
    }
}
