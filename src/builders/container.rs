use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::constants::{component_type, separator_spacing};
use crate::types::{to_json_value, ButtonConfig, MediaGalleryItem};

use super::components::{ActionRowBuilder, ButtonBuilder};
use super::media::{FileBuilder, MediaGalleryBuilder, SectionBuilder};

#[derive(Clone, Serialize, Deserialize, Default)]
pub struct TextDisplayBuilder {
    #[serde(rename = "type")]
    component_type: u8,
    content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    id: Option<u32>,
}

impl TextDisplayBuilder {
    pub fn new(content: &str) -> Self {
        Self {
            component_type: component_type::TEXT_DISPLAY,
            content: content.to_string(),
            id: None,
        }
    }

    pub fn content(mut self, content: &str) -> Self {
        self.content = content.to_string();
        self
    }

    pub fn id(mut self, id: u32) -> Self {
        self.id = Some(id);
        self
    }

    pub fn build(self) -> Value {
        to_json_value(self)
    }
}

#[derive(Clone, Serialize, Deserialize, Default)]
pub struct SeparatorBuilder {
    #[serde(rename = "type")]
    component_type: u8,
    #[serde(skip_serializing_if = "Option::is_none")]
    divider: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    spacing: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    id: Option<u32>,
}

impl SeparatorBuilder {
    pub fn new() -> Self {
        Self {
            component_type: component_type::SEPARATOR,
            divider: Some(true),
            spacing: Some(separator_spacing::SMALL),
            id: None,
        }
    }

    pub fn divider(mut self, divider: bool) -> Self {
        self.divider = Some(divider);
        self
    }

    pub fn spacing(mut self, spacing: u8) -> Self {
        self.spacing = Some(spacing);
        self
    }

    pub fn id(mut self, id: u32) -> Self {
        self.id = Some(id);
        self
    }

    pub fn build(self) -> Value {
        to_json_value(self)
    }
}

#[derive(Clone, Serialize, Deserialize, Default)]
pub struct ContainerBuilder {
    #[serde(rename = "type")]
    component_type: u8,
    components: Vec<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    accent_color: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    spoiler: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    id: Option<u32>,
}

impl ContainerBuilder {
    pub fn new() -> Self {
        Self {
            component_type: component_type::CONTAINER,
            components: Vec::new(),
            accent_color: None,
            spoiler: None,
            id: None,
        }
    }

    pub fn accent_color(mut self, color: u32) -> Self {
        self.accent_color = Some(color);
        self
    }

    pub fn spoiler(mut self, spoiler: bool) -> Self {
        self.spoiler = Some(spoiler);
        self
    }

    pub fn id(mut self, id: u32) -> Self {
        self.id = Some(id);
        self
    }

    pub fn add_media_gallery(mut self, gallery: MediaGalleryBuilder) -> Self {
        self.components.push(gallery.build());
        self
    }

    pub fn add_text_display(mut self, text: TextDisplayBuilder) -> Self {
        self.components.push(text.build());
        self
    }

    pub fn add_separator(mut self, separator: SeparatorBuilder) -> Self {
        self.components.push(separator.build());
        self
    }

    pub fn add_action_row(mut self, row: ActionRowBuilder) -> Self {
        self.components.push(row.build());
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

    pub fn add_component(mut self, component: Value) -> Self {
        self.components.push(component);
        self
    }

    pub fn build(self) -> Value {
        to_json_value(self)
    }
}

pub fn create_container(
    title: &str,
    description: &str,
    buttons: Vec<ButtonConfig>,
    image_url: Option<&str>,
) -> ContainerBuilder {
    let mut container = ContainerBuilder::new();

    if let Some(url) = image_url {
        let gallery = MediaGalleryBuilder::new().add_item(MediaGalleryItem::new(url));
        container = container.add_media_gallery(gallery);

        container = container.add_separator(
            SeparatorBuilder::new()
                .divider(true)
                .spacing(separator_spacing::LARGE),
        );
    }

    container = container.add_text_display(TextDisplayBuilder::new(&format!("**{}**", title)));

    if !description.is_empty() {
        container = container.add_separator(
            SeparatorBuilder::new()
                .divider(true)
                .spacing(separator_spacing::SMALL),
        );
        container = container.add_text_display(TextDisplayBuilder::new(description));
    }

    if !buttons.is_empty() {
        container = container.add_separator(
            SeparatorBuilder::new()
                .divider(false)
                .spacing(separator_spacing::SMALL),
        );

        for chunk in buttons.chunks(5) {
            let mut row = ActionRowBuilder::new();
            for btn_config in chunk {
                let mut button = ButtonBuilder::new()
                    .label(&btn_config.label)
                    .style(btn_config.style)
                    .custom_id(&btn_config.custom_id);

                if let Some(ref emoji) = btn_config.emoji {
                    button = button.emoji_unicode(emoji);
                }

                row = row.add_button(button);
            }
            container = container.add_action_row(row);
        }
    }

    container
}

pub fn create_default_buttons(button_type: &str) -> Vec<ButtonConfig> {
    match button_type {
        "general" => vec![ButtonConfig::new("help_menu", "Help")
            .style(crate::constants::button_style::SECONDARY)
            .emoji("❓")],
        "status" => vec![
            ButtonConfig::new("view_work_status", "Work Status")
                .style(crate::constants::button_style::PRIMARY)
                .emoji("📊"),
            ButtonConfig::new("help_menu", "Help")
                .style(crate::constants::button_style::SECONDARY)
                .emoji("❓"),
        ],
        _ => vec![ButtonConfig::new("help_menu", "Help")
            .style(crate::constants::button_style::SECONDARY)
            .emoji("❓")],
    }
}
