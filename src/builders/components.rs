use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::constants::{button_style, component_type};
use crate::types::{to_json_value, Emoji, SelectOption};

use super::container::{ContainerBuilder, SeparatorBuilder, TextDisplayBuilder};
use super::media::{FileBuilder, MediaGalleryBuilder, SectionBuilder};
use super::modal::TextInputBuilder;

#[derive(Clone, Serialize, Deserialize, Default)]
pub struct ButtonBuilder {
    #[serde(rename = "type")]
    component_type: u8,
    style: u8,
    #[serde(skip_serializing_if = "Option::is_none")]
    label: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    emoji: Option<Emoji>,
    #[serde(skip_serializing_if = "Option::is_none")]
    custom_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    disabled: Option<bool>,
}

impl ButtonBuilder {
    pub fn new() -> Self {
        Self {
            component_type: component_type::BUTTON,
            style: button_style::PRIMARY,
            label: None,
            emoji: None,
            custom_id: None,
            url: None,
            disabled: None,
        }
    }

    pub fn style(mut self, style: u8) -> Self {
        self.style = style;
        self
    }

    pub fn label(mut self, label: &str) -> Self {
        self.label = Some(label.to_string());
        self
    }

    pub fn emoji(mut self, emoji: Emoji) -> Self {
        self.emoji = Some(emoji);
        self
    }

    pub fn emoji_unicode(mut self, emoji: &str) -> Self {
        self.emoji = Some(Emoji::unicode(emoji));
        self
    }

    pub fn custom_id(mut self, custom_id: &str) -> Self {
        self.custom_id = Some(custom_id.to_string());
        self.url = None;
        if self.style == button_style::LINK {
            self.style = button_style::PRIMARY;
        }
        self
    }

    pub fn url(mut self, url: &str) -> Self {
        self.url = Some(url.to_string());
        self.custom_id = None;
        self.style = button_style::LINK;
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = Some(disabled);
        self
    }

    fn normalize(mut self) -> Self {
        if self.url.is_some() {
            self.custom_id = None;
            self.style = button_style::LINK;
        } else if self.style == button_style::LINK {
            self.style = button_style::PRIMARY;
        }
        self
    }

    pub fn build_typed(self) -> Self {
        self.normalize()
    }

    pub fn build_value(self) -> Value {
        to_json_value(self.build_typed())
    }

    pub fn build(self) -> Value {
        self.build_value()
    }
}

#[derive(Clone, Serialize, Deserialize, Default)]
pub struct ActionRowBuilder {
    #[serde(rename = "type")]
    component_type: u8,
    components: Vec<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    id: Option<u32>,
}

impl ActionRowBuilder {
    pub fn new() -> Self {
        Self {
            component_type: component_type::ACTION_ROW,
            components: Vec::new(),
            id: None,
        }
    }

    pub fn add_button(mut self, button: ButtonBuilder) -> Self {
        self.components.push(button.build());
        self
    }

    pub fn add_select_menu(mut self, select_menu: SelectMenuBuilder) -> Self {
        self.components.push(select_menu.build());
        self
    }

    pub fn add_text_input(mut self, input: TextInputBuilder) -> Self {
        self.components.push(input.build());
        self
    }

    pub fn add_component(mut self, component: Value) -> Self {
        self.components.push(component);
        self
    }

    pub fn id(mut self, id: u32) -> Self {
        self.id = Some(id);
        self
    }

    pub fn build_typed(self) -> Self {
        self
    }

    pub fn build_value(self) -> Value {
        to_json_value(self.build_typed())
    }

    pub fn build(self) -> Value {
        self.build_value()
    }
}

#[derive(Clone, Serialize, Deserialize, Default)]
pub struct SelectMenuBuilder {
    #[serde(rename = "type")]
    component_type: u8,
    custom_id: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    options: Vec<SelectOption>,
    #[serde(skip_serializing_if = "Option::is_none")]
    channel_types: Option<Vec<u8>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    placeholder: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    min_values: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_values: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    disabled: Option<bool>,
}

impl SelectMenuBuilder {
    pub fn string(custom_id: &str) -> Self {
        Self {
            component_type: component_type::STRING_SELECT,
            custom_id: custom_id.to_string(),
            options: Vec::new(),
            channel_types: None,
            placeholder: None,
            min_values: None,
            max_values: None,
            disabled: None,
        }
    }

    pub fn role(custom_id: &str) -> Self {
        Self {
            component_type: component_type::ROLE_SELECT,
            custom_id: custom_id.to_string(),
            options: Vec::new(),
            channel_types: None,
            placeholder: None,
            min_values: None,
            max_values: None,
            disabled: None,
        }
    }

    pub fn channel(custom_id: &str) -> Self {
        Self {
            component_type: component_type::CHANNEL_SELECT,
            custom_id: custom_id.to_string(),
            options: Vec::new(),
            channel_types: None,
            placeholder: None,
            min_values: None,
            max_values: None,
            disabled: None,
        }
    }

    pub fn user(custom_id: &str) -> Self {
        Self {
            component_type: component_type::USER_SELECT,
            custom_id: custom_id.to_string(),
            options: Vec::new(),
            channel_types: None,
            placeholder: None,
            min_values: None,
            max_values: None,
            disabled: None,
        }
    }

    pub fn mentionable(custom_id: &str) -> Self {
        Self {
            component_type: component_type::MENTIONABLE_SELECT,
            custom_id: custom_id.to_string(),
            options: Vec::new(),
            channel_types: None,
            placeholder: None,
            min_values: None,
            max_values: None,
            disabled: None,
        }
    }

    pub fn placeholder(mut self, placeholder: &str) -> Self {
        self.placeholder = Some(placeholder.to_string());
        self
    }

    pub fn add_option(mut self, option: SelectOption) -> Self {
        self.options.push(option);
        self
    }

    pub fn add_options(mut self, options: Vec<SelectOption>) -> Self {
        self.options.extend(options);
        self
    }

    pub fn channel_types(mut self, channel_types: Vec<u8>) -> Self {
        self.channel_types = Some(channel_types);
        self
    }

    pub fn min_values(mut self, min: u8) -> Self {
        self.min_values = Some(min);
        self
    }

    pub fn max_values(mut self, max: u8) -> Self {
        self.max_values = Some(max);
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = Some(disabled);
        self
    }

    fn normalize(mut self) -> Self {
        match self.component_type {
            component_type::STRING_SELECT => {
                self.channel_types = None;
            }
            component_type::CHANNEL_SELECT => {
                self.options.clear();
            }
            _ => {
                self.options.clear();
                self.channel_types = None;
            }
        }
        self
    }

    pub fn build_typed(self) -> Self {
        self.normalize()
    }

    pub fn build_value(self) -> Value {
        to_json_value(self.build_typed())
    }

    pub fn build(self) -> Value {
        self.build_value()
    }
}

pub struct ComponentsV2Message {
    components: Vec<Value>,
}

impl Default for ComponentsV2Message {
    fn default() -> Self {
        Self::new()
    }
}

impl ComponentsV2Message {
    pub fn new() -> Self {
        Self {
            components: Vec::new(),
        }
    }

    pub fn add_container(mut self, container: ContainerBuilder) -> Self {
        self.components.push(container.build());
        self
    }

    pub fn add_text_display(mut self, text: TextDisplayBuilder) -> Self {
        self.components.push(text.build());
        self
    }

    pub fn add_media_gallery(mut self, gallery: MediaGalleryBuilder) -> Self {
        self.components.push(gallery.build());
        self
    }

    pub fn add_separator(mut self, separator: SeparatorBuilder) -> Self {
        self.components.push(separator.build());
        self
    }

    pub fn add_section(mut self, section: SectionBuilder) -> Self {
        self.components.push(section.build());
        self
    }

    pub fn add_file(mut self, file: FileBuilder) -> Self {
        self.components.push(file.build());
        self
    }

    pub fn add_action_row(mut self, row: ActionRowBuilder) -> Self {
        self.components.push(row.build());
        self
    }

    pub fn add_component(mut self, component: Value) -> Self {
        self.components.push(component);
        self
    }

    pub fn build(self) -> Vec<Value> {
        self.components
    }
}

#[cfg(test)]
mod tests {
    use super::{ActionRowBuilder, ButtonBuilder, ComponentsV2Message, SelectMenuBuilder};
    use crate::builders::container::TextDisplayBuilder;
    use crate::builders::modal::TextInputBuilder;
    use crate::builders::{
        ContainerBuilder, FileBuilder, MediaGalleryBuilder, SectionBuilder, SeparatorBuilder,
    };
    use crate::constants::{button_style, component_type, text_input_style};
    use crate::types::{Emoji, MediaGalleryItem, SelectOption};
    use serde_json::json;

    #[test]
    fn button_url_omits_custom_id_and_forces_link_style() {
        let payload = ButtonBuilder::new()
            .custom_id("button")
            .url("https://example.com")
            .build();

        assert_eq!(
            payload.get("style").and_then(|value| value.as_u64()),
            Some(button_style::LINK as u64)
        );
        assert!(payload.get("custom_id").is_none());
        assert_eq!(
            payload.get("url").and_then(|value| value.as_str()),
            Some("https://example.com")
        );
    }

    #[test]
    fn button_custom_id_omits_url_and_clears_link_style() {
        let payload = ButtonBuilder::new()
            .url("https://example.com")
            .custom_id("button")
            .build();

        assert_eq!(
            payload.get("style").and_then(|value| value.as_u64()),
            Some(button_style::PRIMARY as u64)
        );
        assert_eq!(
            payload.get("custom_id").and_then(|value| value.as_str()),
            Some("button")
        );
        assert!(payload.get("url").is_none());
    }

    #[test]
    fn button_link_style_without_url_falls_back_to_primary() {
        let payload = ButtonBuilder::new().style(button_style::LINK).build();

        assert_eq!(
            payload.get("style").and_then(|value| value.as_u64()),
            Some(button_style::PRIMARY as u64)
        );
        assert!(payload.get("url").is_none());
    }

    #[test]
    fn component_build_typed_preserves_structs_before_json_conversion() {
        let button = ButtonBuilder::new()
            .url("https://example.com")
            .custom_id("next")
            .build_typed();
        assert_eq!(button.custom_id.as_deref(), Some("next"));
        assert_eq!(button.url, None);
        assert_eq!(button.style, button_style::PRIMARY);

        let select = SelectMenuBuilder::channel("channels")
            .add_option(SelectOption::new("Ignored", "ignored"))
            .channel_types(vec![0, 2])
            .build_typed();
        assert!(select.options.is_empty());
        assert_eq!(select.channel_types, Some(vec![0, 2]));

        let row = ActionRowBuilder::new()
            .add_component(json!({ "type": 99 }))
            .id(42)
            .build_typed();
        assert_eq!(row.id, Some(42));
        assert_eq!(row.components.len(), 1);
    }

    #[test]
    fn string_select_omits_channel_types() {
        let payload = SelectMenuBuilder::string("menu")
            .add_option(SelectOption::new("One", "one"))
            .channel_types(vec![0, 2])
            .build();

        assert_eq!(
            payload.get("type").and_then(|value| value.as_u64()),
            Some(component_type::STRING_SELECT as u64)
        );
        assert!(payload.get("options").is_some());
        assert!(payload.get("channel_types").is_none());
    }

    #[test]
    fn channel_select_omits_options() {
        let payload = SelectMenuBuilder::channel("menu")
            .add_option(SelectOption::new("One", "one"))
            .channel_types(vec![0, 2])
            .build();

        assert_eq!(
            payload.get("type").and_then(|value| value.as_u64()),
            Some(component_type::CHANNEL_SELECT as u64)
        );
        assert!(payload.get("options").is_none());
        assert_eq!(
            payload
                .get("channel_types")
                .and_then(|value| value.as_array())
                .map(|value| value.len()),
            Some(2)
        );
    }

    #[test]
    fn non_string_non_channel_select_omits_variant_specific_fields() {
        let payload = SelectMenuBuilder::role("menu")
            .add_option(SelectOption::new("One", "one"))
            .channel_types(vec![0, 2])
            .build();

        assert_eq!(
            payload.get("type").and_then(|value| value.as_u64()),
            Some(component_type::ROLE_SELECT as u64)
        );
        assert!(payload.get("options").is_none());
        assert!(payload.get("channel_types").is_none());
    }

    #[test]
    fn button_serializes_optional_fields() {
        let payload = ButtonBuilder::new()
            .style(button_style::SUCCESS)
            .label("Ship")
            .emoji(Emoji::custom("party", "123", true))
            .disabled(true)
            .build();

        assert_eq!(
            payload.get("type").and_then(|value| value.as_u64()),
            Some(component_type::BUTTON as u64)
        );
        assert_eq!(
            payload.get("style").and_then(|value| value.as_u64()),
            Some(button_style::SUCCESS as u64)
        );
        assert_eq!(
            payload.get("label").and_then(|value| value.as_str()),
            Some("Ship")
        );
        assert_eq!(
            payload
                .get("emoji")
                .and_then(|value| value.get("name"))
                .and_then(|value| value.as_str()),
            Some("party")
        );
        assert_eq!(
            payload
                .get("emoji")
                .and_then(|value| value.get("id"))
                .and_then(|value| value.as_str()),
            Some("123")
        );
        assert_eq!(
            payload
                .get("emoji")
                .and_then(|value| value.get("animated"))
                .and_then(|value| value.as_bool()),
            Some(true)
        );
        assert_eq!(
            payload.get("disabled").and_then(|value| value.as_bool()),
            Some(true)
        );
    }

    #[test]
    fn button_emoji_unicode_sets_name_only() {
        let payload = ButtonBuilder::new().emoji_unicode("🔥").build();

        assert_eq!(
            payload
                .get("emoji")
                .and_then(|value| value.get("name"))
                .and_then(|value| value.as_str()),
            Some("🔥")
        );
        assert!(payload
            .get("emoji")
            .and_then(|value| value.get("id"))
            .is_none());
    }

    #[test]
    fn action_row_builds_mixed_components_with_id() {
        let payload = ActionRowBuilder::new()
            .add_button(ButtonBuilder::new().label("Go").custom_id("go"))
            .add_select_menu(
                SelectMenuBuilder::string("menu")
                    .placeholder("Pick")
                    .min_values(1)
                    .max_values(2)
                    .disabled(true)
                    .add_options(vec![
                        SelectOption::new("One", "one"),
                        SelectOption::new("Two", "two"),
                    ]),
            )
            .add_text_input(
                TextInputBuilder::short("topic", "Topic")
                    .placeholder("Tell me")
                    .required(true),
            )
            .add_component(json!({ "type": 99, "custom": "raw" }))
            .id(7)
            .build();

        let components = payload
            .get("components")
            .and_then(|value| value.as_array())
            .expect("components array");

        assert_eq!(
            payload.get("type").and_then(|value| value.as_u64()),
            Some(component_type::ACTION_ROW as u64)
        );
        assert_eq!(payload.get("id").and_then(|value| value.as_u64()), Some(7));
        assert_eq!(components.len(), 4);
        assert_eq!(
            components[0].get("label").and_then(|value| value.as_str()),
            Some("Go")
        );
        assert_eq!(
            components[1].get("type").and_then(|value| value.as_u64()),
            Some(component_type::STRING_SELECT as u64)
        );
        assert_eq!(
            components[1]
                .get("options")
                .and_then(|value| value.as_array())
                .map(|value| value.len()),
            Some(2)
        );
        assert_eq!(
            components[1]
                .get("placeholder")
                .and_then(|value| value.as_str()),
            Some("Pick")
        );
        assert_eq!(
            components[1]
                .get("min_values")
                .and_then(|value| value.as_u64()),
            Some(1)
        );
        assert_eq!(
            components[1]
                .get("max_values")
                .and_then(|value| value.as_u64()),
            Some(2)
        );
        assert_eq!(
            components[1]
                .get("disabled")
                .and_then(|value| value.as_bool()),
            Some(true)
        );
        assert_eq!(
            components[2].get("type").and_then(|value| value.as_u64()),
            Some(component_type::TEXT_INPUT as u64)
        );
        assert_eq!(
            components[2].get("style").and_then(|value| value.as_u64()),
            Some(text_input_style::SHORT as u64)
        );
        assert_eq!(
            components[3].get("custom").and_then(|value| value.as_str()),
            Some("raw")
        );
    }

    #[test]
    fn mentionable_select_keeps_shared_fields_and_omits_variant_specific_fields() {
        let payload = SelectMenuBuilder::mentionable("menu")
            .placeholder("Pick a target")
            .add_option(SelectOption::new("One", "one"))
            .channel_types(vec![0, 2])
            .min_values(1)
            .max_values(3)
            .disabled(true)
            .build();

        assert_eq!(
            payload.get("type").and_then(|value| value.as_u64()),
            Some(component_type::MENTIONABLE_SELECT as u64)
        );
        assert_eq!(
            payload.get("placeholder").and_then(|value| value.as_str()),
            Some("Pick a target")
        );
        assert_eq!(
            payload.get("min_values").and_then(|value| value.as_u64()),
            Some(1)
        );
        assert_eq!(
            payload.get("max_values").and_then(|value| value.as_u64()),
            Some(3)
        );
        assert_eq!(
            payload.get("disabled").and_then(|value| value.as_bool()),
            Some(true)
        );
        assert!(payload.get("options").is_none());
        assert!(payload.get("channel_types").is_none());
    }

    #[test]
    fn user_select_builder_sets_expected_component_type() {
        let payload = SelectMenuBuilder::user("menu").build();

        assert_eq!(
            payload.get("type").and_then(|value| value.as_u64()),
            Some(component_type::USER_SELECT as u64)
        );
    }

    #[test]
    fn components_v2_message_preserves_component_order() {
        let payload = ComponentsV2Message::new()
            .add_text_display(TextDisplayBuilder::new("Intro").id(1))
            .add_action_row(
                ActionRowBuilder::new()
                    .add_button(ButtonBuilder::new().label("Continue").custom_id("continue")),
            )
            .add_component(json!({ "type": 255, "marker": "raw" }))
            .build();

        assert_eq!(payload.len(), 3);
        assert_eq!(
            payload[0].get("type").and_then(|value| value.as_u64()),
            Some(component_type::TEXT_DISPLAY as u64)
        );
        assert_eq!(
            payload[0].get("content").and_then(|value| value.as_str()),
            Some("Intro")
        );
        assert_eq!(
            payload[1].get("type").and_then(|value| value.as_u64()),
            Some(component_type::ACTION_ROW as u64)
        );
        assert_eq!(
            payload[2].get("marker").and_then(|value| value.as_str()),
            Some("raw")
        );
    }

    #[test]
    fn components_v2_message_supports_all_builder_entry_points() {
        let payload = ComponentsV2Message::new()
            .add_container(
                ContainerBuilder::new().add_text_display(TextDisplayBuilder::new("inside")),
            )
            .add_media_gallery(
                MediaGalleryBuilder::new()
                    .add_item(MediaGalleryItem::new("https://example.com/image.png")),
            )
            .add_separator(SeparatorBuilder::new())
            .add_section(SectionBuilder::new().add_text_display(TextDisplayBuilder::new("section")))
            .add_file(FileBuilder::new("https://example.com/file.txt"))
            .build();

        assert_eq!(
            payload
                .iter()
                .map(|component| component.get("type").and_then(|value| value.as_u64()))
                .collect::<Vec<_>>(),
            vec![
                Some(component_type::CONTAINER as u64),
                Some(component_type::MEDIA_GALLERY as u64),
                Some(component_type::SEPARATOR as u64),
                Some(component_type::SECTION as u64),
                Some(component_type::FILE as u64),
            ]
        );
    }
}
