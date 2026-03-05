use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::constants::button_style;

pub type Error = Box<dyn std::error::Error + Send + Sync>;

pub(crate) fn to_json_value<T: Serialize>(value: T) -> Value {
    serde_json::to_value(value).expect("failed to serialize components v2 value")
}

pub(crate) fn invalid_data_error(message: impl Into<String>) -> Error {
    std::io::Error::new(std::io::ErrorKind::InvalidData, message.into()).into()
}

#[derive(Clone, Serialize, Deserialize, Default)]
pub struct Emoji {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub animated: Option<bool>,
}

impl Emoji {
    pub fn unicode(emoji: &str) -> Self {
        Self {
            name: Some(emoji.to_string()),
            id: None,
            animated: None,
        }
    }

    pub fn custom(name: &str, id: &str, animated: bool) -> Self {
        Self {
            name: Some(name.to_string()),
            id: Some(id.to_string()),
            animated: Some(animated),
        }
    }
}

#[derive(Clone, Serialize, Deserialize, Default)]
pub struct MediaGalleryItem {
    pub media: MediaInfo,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub spoiler: Option<bool>,
}

#[derive(Clone, Serialize, Deserialize, Default)]
pub struct MediaInfo {
    pub url: String,
}

impl MediaGalleryItem {
    pub fn new(url: &str) -> Self {
        Self {
            media: MediaInfo {
                url: url.to_string(),
            },
            description: None,
            spoiler: None,
        }
    }

    pub fn description(mut self, desc: &str) -> Self {
        self.description = Some(desc.to_string());
        self
    }

    pub fn spoiler(mut self, spoiler: bool) -> Self {
        self.spoiler = Some(spoiler);
        self
    }
}

#[derive(Clone, Serialize, Deserialize, Default)]
pub struct SelectOption {
    pub label: String,
    pub value: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub emoji: Option<Emoji>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default: Option<bool>,
}

impl SelectOption {
    pub fn new(label: &str, value: &str) -> Self {
        Self {
            label: label.to_string(),
            value: value.to_string(),
            description: None,
            emoji: None,
            default: None,
        }
    }

    pub fn description(mut self, desc: &str) -> Self {
        self.description = Some(desc.to_string());
        self
    }

    pub fn emoji(mut self, emoji: &str) -> Self {
        self.emoji = Some(Emoji::unicode(emoji));
        self
    }

    pub fn default_selected(mut self, default: bool) -> Self {
        self.default = Some(default);
        self
    }
}

#[derive(Clone, Default)]
pub struct ButtonConfig {
    pub custom_id: String,
    pub label: String,
    pub style: u8,
    pub emoji: Option<String>,
}

impl ButtonConfig {
    pub fn new(custom_id: &str, label: &str) -> Self {
        Self {
            custom_id: custom_id.to_string(),
            label: label.to_string(),
            style: button_style::PRIMARY,
            emoji: None,
        }
    }

    pub fn style(mut self, style: u8) -> Self {
        self.style = style;
        self
    }

    pub fn emoji(mut self, emoji: &str) -> Self {
        self.emoji = Some(emoji.to_string());
        self
    }
}
