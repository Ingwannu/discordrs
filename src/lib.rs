//! Components v2 빌더 모듈
//!
//! 왜 필요한지: Discord Components v2 (Container, MediaGallery, TextDisplay 등)를 serenity에서 지원하지 않아 수동 구현
//! 어떤 코드와 연계: commands와 events에서 메시지 생성 시 사용
//! 역할: Components v2 JSON 구조를 생성하는 빌더 패턴 구현

use serde::{Deserialize, Serialize};
use serde_json::Value;

pub type Error = Box<dyn std::error::Error + Send + Sync>;

fn to_json_value<T: Serialize>(value: T) -> Value {
    serde_json::to_value(value).expect("failed to serialize components v2 value")
}

/// Discord Components v2 타입 번호
/// 왜 필요한지: Discord API에서 각 컴포넌트 타입을 구분하는 데 사용
/// 어떤 코드와 연계: 모든 컴포넌트 빌더에서 type 필드에 사용
/// 역할: Discord API 스펙에 맞는 타입 번호 정의
pub mod component_type {
    pub const ACTION_ROW: u8 = 1;
    pub const BUTTON: u8 = 2;
    pub const STRING_SELECT: u8 = 3;
    pub const TEXT_INPUT: u8 = 4;
    pub const USER_SELECT: u8 = 5;
    pub const ROLE_SELECT: u8 = 6;
    pub const MENTIONABLE_SELECT: u8 = 7;
    pub const CHANNEL_SELECT: u8 = 8;
    pub const SECTION: u8 = 9;
    pub const TEXT_DISPLAY: u8 = 10;
    pub const THUMBNAIL: u8 = 11;
    pub const MEDIA_GALLERY: u8 = 12;
    pub const FILE: u8 = 13;
    pub const SEPARATOR: u8 = 14;
    pub const CONTENT_INVENTORY_ENTRY: u8 = 16;
    pub const CONTAINER: u8 = 17;
    pub const LABEL: u8 = 18;
    pub const FILE_UPLOAD: u8 = 19;
    pub const RADIO_GROUP: u8 = 21;
    pub const CHECKBOX_GROUP: u8 = 22;
    pub const CHECKBOX: u8 = 23;
}

/// 버튼 스타일
/// 왜 필요한지: Discord 버튼의 시각적 스타일을 지정
/// 어떤 코드와 연계: ButtonBuilder에서 사용
/// 역할: 버튼 색상/스타일 정의
pub mod button_style {
    pub const PRIMARY: u8 = 1; // 파란색
    pub const SECONDARY: u8 = 2; // 회색
    pub const SUCCESS: u8 = 3; // 초록색
    pub const DANGER: u8 = 4; // 빨간색
    pub const LINK: u8 = 5; // URL 링크
}

/// Separator 간격 크기
/// 왜 필요한지: Separator 컴포넌트의 간격을 지정
/// 어떤 코드와 연계: SeparatorBuilder에서 사용
/// 역할: Discord API 스펙에 맞는 간격 크기 정의
pub mod separator_spacing {
    pub const SMALL: u8 = 1;
    pub const LARGE: u8 = 2;
}

/// IS_COMPONENTS_V2 메시지 플래그
/// 왜 필요한지: Components v2를 사용하려면 이 플래그를 설정해야 함
/// 어떤 코드와 연계: 메시지 전송 시 flags 필드에 사용
/// 역할: Discord에게 Components v2 메시지임을 알림
pub const MESSAGE_FLAG_IS_COMPONENTS_V2: u64 = 1 << 15; // 32768

/// 이모지 구조체
/// 왜 필요한지: 버튼에 이모지를 표시하기 위해
/// 어떤 코드와 연계: ButtonBuilder에서 사용
/// 역할: 유니코드 이모지 또는 커스텀 이모지 표현
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
    /// 유니코드 이모지 생성
    pub fn unicode(emoji: &str) -> Self {
        Self {
            name: Some(emoji.to_string()),
            id: None,
            animated: None,
        }
    }

    /// 커스텀 이모지 생성
    pub fn custom(name: &str, id: &str, animated: bool) -> Self {
        Self {
            name: Some(name.to_string()),
            id: Some(id.to_string()),
            animated: Some(animated),
        }
    }
}

/// MediaGallery 아이템 빌더
/// 왜 필요한지: MediaGallery에 이미지/비디오를 추가하기 위해
/// 어떤 코드와 연계: MediaGalleryBuilder에서 사용
/// 역할: 미디어 아이템의 URL과 설명 설정
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

/// MediaGallery 빌더
/// 왜 필요한지: 이미지/비디오 갤러리를 생성하기 위해
/// 어떤 코드와 연계: ContainerBuilder에서 사용, JS의 MediaGalleryBuilder 대응
/// 역할: 여러 미디어 아이템을 담는 갤러리 컴포넌트 생성
#[derive(Clone, Serialize, Deserialize, Default)]
pub struct MediaGalleryBuilder {
    #[serde(rename = "type")]
    component_type: u8,
    items: Vec<MediaGalleryItem>,
    #[serde(skip_serializing_if = "Option::is_none")]
    id: Option<u32>,
}

impl MediaGalleryBuilder {
    pub fn new() -> Self {
        Self {
            component_type: component_type::MEDIA_GALLERY,
            items: Vec::new(),
            id: None,
        }
    }

    pub fn add_item(mut self, item: MediaGalleryItem) -> Self {
        self.items.push(item);
        self
    }

    pub fn add_items(mut self, items: Vec<MediaGalleryItem>) -> Self {
        self.items.extend(items);
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

/// TextDisplay 빌더
/// 왜 필요한지: 텍스트 콘텐츠를 표시하기 위해 (Embed의 description 대체)
/// 어떤 코드와 연계: ContainerBuilder에서 사용, JS의 TextDisplayBuilder 대응
/// 역할: 마크다운 지원 텍스트 컴포넌트 생성
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

/// Separator 빌더
/// 왜 필요한지: 컴포넌트 사이에 구분선을 추가하기 위해
/// 어떤 코드와 연계: ContainerBuilder에서 사용, JS의 SeparatorBuilder 대응
/// 역할: 시각적 구분선 컴포넌트 생성
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

/// Button 빌더
/// 왜 필요한지: 클릭 가능한 버튼을 생성하기 위해
/// 어떤 코드와 연계: ActionRowBuilder에서 사용, JS의 ButtonBuilder 대응
/// 역할: 다양한 스타일의 버튼 컴포넌트 생성
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
        self
    }

    pub fn url(mut self, url: &str) -> Self {
        self.url = Some(url.to_string());
        self.style = button_style::LINK;
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = Some(disabled);
        self
    }

    pub fn build(self) -> Value {
        to_json_value(self)
    }
}

pub mod text_input_style {
    pub const SHORT: u8 = 1;
    pub const PARAGRAPH: u8 = 2;
}

#[derive(Clone, Serialize, Deserialize, Default)]
pub struct TextInputBuilder {
    #[serde(rename = "type")]
    component_type: u8,
    custom_id: String,
    style: u8,
    label: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    placeholder: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    value: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    required: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    min_length: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_length: Option<u16>,
}

impl TextInputBuilder {
    pub fn new(custom_id: &str, label: &str, style: u8) -> Self {
        Self {
            component_type: component_type::TEXT_INPUT,
            custom_id: custom_id.to_string(),
            style,
            label: label.to_string(),
            placeholder: None,
            value: None,
            required: None,
            min_length: None,
            max_length: None,
        }
    }

    pub fn short(custom_id: &str, label: &str) -> Self {
        Self::new(custom_id, label, text_input_style::SHORT)
    }

    pub fn paragraph(custom_id: &str, label: &str) -> Self {
        Self::new(custom_id, label, text_input_style::PARAGRAPH)
    }

    pub fn placeholder(mut self, placeholder: &str) -> Self {
        self.placeholder = Some(placeholder.to_string());
        self
    }

    pub fn value(mut self, value: &str) -> Self {
        self.value = Some(value.to_string());
        self
    }

    pub fn required(mut self, required: bool) -> Self {
        self.required = Some(required);
        self
    }

    pub fn min_length(mut self, min: u16) -> Self {
        self.min_length = Some(min);
        self
    }

    pub fn max_length(mut self, max: u16) -> Self {
        self.max_length = Some(max);
        self
    }

    pub fn build(self) -> Value {
        to_json_value(self)
    }
}

/// ActionRow 빌더
/// 왜 필요한지: 버튼이나 선택 메뉴를 한 행에 배치하기 위해
/// 어떤 코드와 연계: ContainerBuilder에서 사용, JS의 ActionRowBuilder 대응
/// 역할: 최대 5개의 버튼을 담는 행 컴포넌트 생성
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

    pub fn build(self) -> Value {
        to_json_value(self)
    }
}

/// SelectMenu 옵션
/// 왜 필요한지: 드롭다운 메뉴의 각 선택지를 정의
/// 어떤 코드와 연계: SelectMenuBuilder에서 사용
/// 역할: 선택지의 라벨, 값, 설명, 이모지 설정
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

/// SelectMenu 빌더
/// 왜 필요한지: 드롭다운 선택 메뉴를 생성하기 위해
/// 어떤 코드와 연계: ActionRowBuilder에서 사용, 티켓/역할버튼 수정에 활용
/// 역할: 문자열/역할/채널 등 다양한 타입의 SelectMenu 생성
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
    /// 문자열 SelectMenu 생성 (커스텀 옵션)
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

    /// 역할 SelectMenu 생성 (서버 역할 선택)
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

    /// 채널 SelectMenu 생성 (서버 채널 선택)
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

    /// 사용자 SelectMenu 생성 (서버 멤버 선택)
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

    /// Mentionable SelectMenu 생성 (유저 + 역할 모두 선택 가능)
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

    pub fn build(self) -> Value {
        to_json_value(self)
    }
}

#[derive(Clone, Serialize, Deserialize, Default)]
pub struct RadioGroupBuilder {
    #[serde(rename = "type")]
    component_type: u8,
    custom_id: String,
    options: Vec<SelectOption>,
    #[serde(skip_serializing_if = "Option::is_none")]
    required: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    disabled: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    id: Option<u32>,
}

impl RadioGroupBuilder {
    pub fn new(custom_id: &str) -> Self {
        Self {
            component_type: component_type::RADIO_GROUP,
            custom_id: custom_id.to_string(),
            options: Vec::new(),
            required: None,
            disabled: None,
            id: None,
        }
    }

    pub fn add_option(mut self, option: SelectOption) -> Self {
        self.options.push(option);
        self
    }

    pub fn add_options(mut self, options: Vec<SelectOption>) -> Self {
        self.options.extend(options);
        self
    }

    pub fn required(mut self, required: bool) -> Self {
        self.required = Some(required);
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = Some(disabled);
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
pub struct CheckboxGroupBuilder {
    #[serde(rename = "type")]
    component_type: u8,
    custom_id: String,
    options: Vec<SelectOption>,
    #[serde(skip_serializing_if = "Option::is_none")]
    min_values: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_values: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    required: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    disabled: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    id: Option<u32>,
}

impl CheckboxGroupBuilder {
    pub fn new(custom_id: &str) -> Self {
        Self {
            component_type: component_type::CHECKBOX_GROUP,
            custom_id: custom_id.to_string(),
            options: Vec::new(),
            min_values: None,
            max_values: None,
            required: None,
            disabled: None,
            id: None,
        }
    }

    pub fn add_option(mut self, option: SelectOption) -> Self {
        self.options.push(option);
        self
    }

    pub fn add_options(mut self, options: Vec<SelectOption>) -> Self {
        self.options.extend(options);
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

    pub fn required(mut self, required: bool) -> Self {
        self.required = Some(required);
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = Some(disabled);
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
pub struct CheckboxBuilder {
    #[serde(rename = "type")]
    component_type: u8,
    custom_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    checked: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    required: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    disabled: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    id: Option<u32>,
}

impl CheckboxBuilder {
    pub fn new(custom_id: &str) -> Self {
        Self {
            component_type: component_type::CHECKBOX,
            custom_id: custom_id.to_string(),
            checked: None,
            required: None,
            disabled: None,
            id: None,
        }
    }

    pub fn checked(mut self, checked: bool) -> Self {
        self.checked = Some(checked);
        self
    }

    pub fn required(mut self, required: bool) -> Self {
        self.required = Some(required);
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = Some(disabled);
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
pub struct ThumbnailBuilder {
    #[serde(rename = "type")]
    component_type: u8,
    media: MediaInfo,
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    spoiler: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    id: Option<u32>,
}

impl ThumbnailBuilder {
    pub fn new(url: &str) -> Self {
        Self {
            component_type: component_type::THUMBNAIL,
            media: MediaInfo {
                url: url.to_string(),
            },
            description: None,
            spoiler: None,
            id: None,
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

    pub fn id(mut self, id: u32) -> Self {
        self.id = Some(id);
        self
    }

    pub fn build(self) -> Value {
        to_json_value(self)
    }
}

#[derive(Clone, Serialize, Deserialize, Default)]
pub struct FileBuilder {
    #[serde(rename = "type")]
    component_type: u8,
    file: MediaInfo,
    #[serde(skip_serializing_if = "Option::is_none")]
    spoiler: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    id: Option<u32>,
}

impl FileBuilder {
    pub fn new(url: &str) -> Self {
        Self {
            component_type: component_type::FILE,
            file: MediaInfo {
                url: url.to_string(),
            },
            spoiler: None,
            id: None,
        }
    }

    pub fn spoiler(mut self, spoiler: bool) -> Self {
        self.spoiler = Some(spoiler);
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
pub struct LabelBuilder {
    #[serde(rename = "type")]
    component_type: u8,
    label: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<String>,
    component: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    id: Option<u32>,
}

impl LabelBuilder {
    pub fn with_select_menu(label: &str, select: SelectMenuBuilder) -> Self {
        Self {
            component_type: component_type::LABEL,
            label: label.to_string(),
            description: None,
            component: select.build(),
            id: None,
        }
    }

    pub fn with_file_upload(label: &str, file_upload: FileUploadBuilder) -> Self {
        Self {
            component_type: component_type::LABEL,
            label: label.to_string(),
            description: None,
            component: file_upload.build(),
            id: None,
        }
    }

    pub fn with_radio_group(label: &str, radio_group: RadioGroupBuilder) -> Self {
        Self {
            component_type: component_type::LABEL,
            label: label.to_string(),
            description: None,
            component: radio_group.build(),
            id: None,
        }
    }

    pub fn with_checkbox_group(label: &str, checkbox_group: CheckboxGroupBuilder) -> Self {
        Self {
            component_type: component_type::LABEL,
            label: label.to_string(),
            description: None,
            component: checkbox_group.build(),
            id: None,
        }
    }

    pub fn with_checkbox(label: &str, checkbox: CheckboxBuilder) -> Self {
        Self {
            component_type: component_type::LABEL,
            label: label.to_string(),
            description: None,
            component: checkbox.build(),
            id: None,
        }
    }

    pub fn description(mut self, desc: &str) -> Self {
        self.description = Some(desc.to_string());
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
pub struct FileUploadBuilder {
    #[serde(rename = "type")]
    component_type: u8,
    custom_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    min_values: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_values: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    required: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    id: Option<u32>,
}

impl FileUploadBuilder {
    pub fn new(custom_id: &str) -> Self {
        Self {
            component_type: component_type::FILE_UPLOAD,
            custom_id: custom_id.to_string(),
            min_values: None,
            max_values: None,
            required: None,
            id: None,
        }
    }

    pub fn min_values(mut self, min: u8) -> Self {
        self.min_values = Some(min);
        self
    }

    pub fn max_values(mut self, max: u8) -> Self {
        self.max_values = Some(max);
        self
    }

    pub fn required(mut self, required: bool) -> Self {
        self.required = Some(required);
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
pub struct SectionBuilder {
    #[serde(rename = "type")]
    component_type: u8,
    components: Vec<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    accessory: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    id: Option<u32>,
}

impl SectionBuilder {
    pub fn new() -> Self {
        Self {
            component_type: component_type::SECTION,
            components: Vec::new(),
            accessory: None,
            id: None,
        }
    }

    pub fn add_text_display(mut self, text: TextDisplayBuilder) -> Self {
        self.components.push(text.build());
        self
    }

    pub fn set_thumbnail_accessory(mut self, thumbnail: ThumbnailBuilder) -> Self {
        self.accessory = Some(thumbnail.build());
        self
    }

    pub fn set_button_accessory(mut self, button: ButtonBuilder) -> Self {
        self.accessory = Some(button.build());
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

/// Container 빌더
/// 왜 필요한지: Components v2의 최상위 컨테이너를 생성하기 위해
/// 어떤 코드와 연계: 모든 메시지 전송에서 사용, JS의 ContainerBuilder 대응
/// 역할: 여러 컴포넌트를 담는 최상위 컨테이너 생성
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

    /// 배경 색상 설정 (hex 색상 코드, 예: 0x2b2d31)
    pub fn accent_color(mut self, color: u32) -> Self {
        self.accent_color = Some(color);
        self
    }

    /// 스포일러 설정
    pub fn spoiler(mut self, spoiler: bool) -> Self {
        self.spoiler = Some(spoiler);
        self
    }

    pub fn id(mut self, id: u32) -> Self {
        self.id = Some(id);
        self
    }

    /// MediaGallery 추가
    pub fn add_media_gallery(mut self, gallery: MediaGalleryBuilder) -> Self {
        self.components.push(gallery.build());
        self
    }

    /// TextDisplay 추가
    pub fn add_text_display(mut self, text: TextDisplayBuilder) -> Self {
        self.components.push(text.build());
        self
    }

    /// Separator 추가
    pub fn add_separator(mut self, separator: SeparatorBuilder) -> Self {
        self.components.push(separator.build());
        self
    }

    /// ActionRow 추가
    pub fn add_action_row(mut self, row: ActionRowBuilder) -> Self {
        self.components.push(row.build());
        self
    }

    /// Section 추가
    pub fn add_section(mut self, section: SectionBuilder) -> Self {
        self.components.push(section.build());
        self
    }

    /// File 추가
    pub fn add_file(mut self, file: FileBuilder) -> Self {
        self.components.push(file.build());
        self
    }

    /// 임의의 컴포넌트 추가
    pub fn add_component(mut self, component: Value) -> Self {
        self.components.push(component);
        self
    }

    pub fn build(self) -> Value {
        to_json_value(self)
    }
}

#[derive(Clone, Serialize, Deserialize, Default)]
pub struct ModalBuilder {
    custom_id: String,
    title: String,
    components: Vec<Value>,
}

impl ModalBuilder {
    fn with_optional_description(label: LabelBuilder, description: Option<&str>) -> LabelBuilder {
        if let Some(description) = description {
            label.description(description)
        } else {
            label
        }
    }

    pub fn new(custom_id: &str, title: &str) -> Self {
        Self {
            custom_id: custom_id.to_string(),
            title: title.to_string(),
            components: Vec::new(),
        }
    }

    pub fn add_text_input(mut self, input: TextInputBuilder) -> Self {
        let row = ActionRowBuilder::new().add_component(input.build());
        self.components.push(row.build());
        self
    }

    pub fn add_select_menu(
        mut self,
        label: &str,
        description: Option<&str>,
        select: SelectMenuBuilder,
    ) -> Self {
        self.components.push(
            Self::with_optional_description(
                LabelBuilder::with_select_menu(label, select),
                description,
            )
            .build(),
        );
        self
    }

    pub fn add_file_upload(
        mut self,
        label: &str,
        description: Option<&str>,
        file_upload: FileUploadBuilder,
    ) -> Self {
        self.components.push(
            Self::with_optional_description(
                LabelBuilder::with_file_upload(label, file_upload),
                description,
            )
            .build(),
        );
        self
    }

    pub fn add_radio_group(
        mut self,
        label: &str,
        description: Option<&str>,
        radio_group: RadioGroupBuilder,
    ) -> Self {
        self.components.push(
            Self::with_optional_description(
                LabelBuilder::with_radio_group(label, radio_group),
                description,
            )
            .build(),
        );
        self
    }

    pub fn add_checkbox_group(
        mut self,
        label: &str,
        description: Option<&str>,
        checkbox_group: CheckboxGroupBuilder,
    ) -> Self {
        self.components.push(
            Self::with_optional_description(
                LabelBuilder::with_checkbox_group(label, checkbox_group),
                description,
            )
            .build(),
        );
        self
    }

    pub fn add_checkbox(
        mut self,
        label: &str,
        description: Option<&str>,
        checkbox: CheckboxBuilder,
    ) -> Self {
        self.components.push(
            Self::with_optional_description(
                LabelBuilder::with_checkbox(label, checkbox),
                description,
            )
            .build(),
        );
        self
    }

    pub fn add_label(mut self, label: LabelBuilder) -> Self {
        self.components.push(label.build());
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

    pub fn build(self) -> Value {
        to_json_value(self)
    }
}

/// 톱레벨 컴포넌트 배치용 빌더
/// Container 없이 TextDisplay, Section, MediaGallery, Separator, File, ActionRow 등을
/// 바로 최상위에 배치할 수 있음. Container도 여러 개 함께 배치 가능.
pub struct ComponentsV2Message {
    components: Vec<Value>,
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

/// JS의 createContainer 함수와 동일한 편의 함수
/// 왜 필요한지: JS 코드와 동일한 인터페이스 제공
/// 어떤 코드와 연계: 모든 명령어에서 사용
/// 역할: 제목, 설명, 버튼, 이미지를 포함한 표준 컨테이너 생성
pub fn create_container(
    title: &str,
    description: &str,
    buttons: Vec<ButtonConfig>,
    image_url: Option<&str>,
) -> ContainerBuilder {
    let mut container = ContainerBuilder::new();

    // 이미지가 있으면 MediaGallery 추가
    if let Some(url) = image_url {
        let gallery = MediaGalleryBuilder::new().add_item(MediaGalleryItem::new(url));
        container = container.add_media_gallery(gallery);

        // 이미지와 제목 사이 구분선
        container = container.add_separator(
            SeparatorBuilder::new()
                .divider(true)
                .spacing(separator_spacing::LARGE),
        );
    }

    // 제목 추가
    container = container.add_text_display(TextDisplayBuilder::new(&format!("**{}**", title)));

    // 설명이 있으면 추가
    if !description.is_empty() {
        container = container.add_separator(
            SeparatorBuilder::new()
                .divider(true)
                .spacing(separator_spacing::SMALL),
        );
        container = container.add_text_display(TextDisplayBuilder::new(description));
    }

    // 버튼이 있으면 ActionRow로 추가
    if !buttons.is_empty() {
        container = container.add_separator(
            SeparatorBuilder::new()
                .divider(false)
                .spacing(separator_spacing::SMALL),
        );

        // 버튼을 5개씩 묶어서 ActionRow 생성 (Discord 제한)
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

/// 버튼 설정 구조체
/// 왜 필요한지: create_container 함수에 버튼 정보를 전달하기 위해
/// 어떤 코드와 연계: create_container 함수에서 사용
/// 역할: 버튼의 라벨, 스타일, 이모지, custom_id 저장
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

/// 기본 버튼 생성 (JS의 createDefaultButtons 대응)
/// 왜 필요한지: 공통 버튼 세트를 쉽게 생성하기 위해
/// 어떤 코드와 연계: 여러 명령어에서 공통 버튼으로 사용
/// 역할: 도움말, 새로고침 등 공통 버튼 생성
pub fn create_default_buttons(button_type: &str) -> Vec<ButtonConfig> {
    match button_type {
        "general" => vec![ButtonConfig::new("help_menu", "도움말")
            .style(button_style::SECONDARY)
            .emoji("❓")],
        "status" => vec![
            ButtonConfig::new("view_work_status", "근무 상태")
                .style(button_style::PRIMARY)
                .emoji("📊"),
            ButtonConfig::new("help_menu", "도움말")
                .style(button_style::SECONDARY)
                .emoji("❓"),
        ],
        _ => vec![ButtonConfig::new("help_menu", "도움말")
            .style(button_style::SECONDARY)
            .emoji("❓")],
    }
}

use serenity::all as serenity;
use serenity::all::ChannelId;
use serenity::http::Http;

// MessageFlags::IS_COMPONENTS_V2 is not available in serenity, use our constant instead

/// JSON Value를 serenity의 CreateActionRow로 변환
/// 왜 필요한지: serenity의 components 메서드가 CreateActionRow 벡터를 받음
/// 어떤 코드와 연계: send_components_v2_message 등에서 사용
/// 역할: JSON 컴포넌트를 serenity 타입으로 변환
#[allow(dead_code)]
fn deserialize_components(_components: &[Value]) -> Vec<serenity::CreateActionRow> {
    // Components v2에서는 Container가 최상위 컴포넌트이므로
    // serenity의 기존 구조와 맞지 않음
    // 이 함수는 사용되지 않고, send_container_message를 사용해야 함
    vec![]
}

fn components_v2_flags(ephemeral: bool) -> u64 {
    let mut flags = MESSAGE_FLAG_IS_COMPONENTS_V2;
    if ephemeral {
        flags |= 1 << 6;
    }
    flags
}

struct ComponentsV2Payload {
    components: Vec<Value>,
    ephemeral: bool,
}

impl ComponentsV2Payload {
    fn new(components: Vec<Value>) -> Self {
        Self {
            components,
            ephemeral: false,
        }
    }

    fn ephemeral(mut self, ephemeral: bool) -> Self {
        self.ephemeral = ephemeral;
        self
    }

    fn into_map(self) -> Result<serde_json::Map<String, Value>, Error> {
        let mut map = serde_json::Map::new();
        map.insert(
            "components".to_string(),
            serde_json::to_value(&self.components)?,
        );
        map.insert(
            "flags".to_string(),
            serde_json::Value::Number(components_v2_flags(self.ephemeral).into()),
        );
        Ok(map)
    }
}

/// Raw HTTP 요청으로 Components v2 메시지 전송
/// 왜 필요한지: serenity의 기본 빌더가 Components v2 Container를 지원하지 않음
/// 어떤 코드와 연계: 모든 Components v2 메시지 전송에서 사용
/// 역할: Discord API에 직접 HTTP 요청을 보내 Components v2 메시지 전송
pub async fn send_container_message(
    http: &Http,
    channel_id: ChannelId,
    container: ContainerBuilder,
) -> Result<serenity::Message, Error> {
    // serenity의 fire 메서드를 사용하여 raw 요청 전송
    // Components v2는 components 배열에 Container를 직접 포함
    let map = ComponentsV2Payload::new(vec![container.build()]).into_map()?;

    let result = http.send_message(channel_id, vec![], &map).await?;
    Ok(result)
}

/// Channel에서 Components v2 메시지 전송 (편의 함수)
/// 왜 필요한지: GuildChannel에서 직접 Container 메시지를 보내기 위해
/// 어떤 코드와 연계: 명령어에서 channel.send_container() 형태로 사용
/// 역할: send_container_message를 래핑하여 더 간단한 API 제공
pub async fn send_to_channel(
    http: &Http,
    channel_id: ChannelId,
    title: &str,
    description: &str,
    buttons: Vec<ButtonConfig>,
    image_url: Option<&str>,
) -> Result<serenity::Message, Error> {
    let container = create_container(title, description, buttons, image_url);
    send_container_message(http, channel_id, container).await
}

/// Interaction 응답으로 Components v2 전송
/// 왜 필요한지: slash command 응답에서 Container를 사용하기 위해
/// 어떤 코드와 연계: 명령어 핸들러에서 interaction 응답 시 사용
/// 역할: CreateInteractionResponse에 Components v2 포함
pub async fn respond_with_container(
    http: &Http,
    interaction: &serenity::CommandInteraction,
    container: ContainerBuilder,
    ephemeral: bool,
) -> Result<(), Error> {
    let map = ComponentsV2Payload::new(vec![container.build()])
        .ephemeral(ephemeral)
        .into_map()?;

    let response_data = serde_json::json!({
        "type": 4, // CHANNEL_MESSAGE_WITH_SOURCE
        "data": map
    });

    http.create_interaction_response(interaction.id, &interaction.token, &response_data, vec![])
        .await?;

    Ok(())
}

/// Component Interaction 응답으로 Components v2 전송
/// 왜 필요한지: 버튼 클릭 등의 응답에서 Container를 사용하기 위해  
/// 어떤 코드와 연계: events.rs에서 component interaction 응답 시 사용
/// 역할: ComponentInteraction 응답에 Components v2 포함
pub async fn respond_component_with_container(
    http: &Http,
    interaction: &serenity::ComponentInteraction,
    container: ContainerBuilder,
    ephemeral: bool,
) -> Result<(), Error> {
    let map = ComponentsV2Payload::new(vec![container.build()])
        .ephemeral(ephemeral)
        .into_map()?;

    let response_data = serde_json::json!({
        "type": 4, // CHANNEL_MESSAGE_WITH_SOURCE
        "data": map
    });

    http.create_interaction_response(interaction.id, &interaction.token, &response_data, vec![])
        .await?;

    Ok(())
}

/// Deferred 응답 후 Container로 followup 전송
pub async fn respond_modal_with_container(
    http: &Http,
    interaction: &serenity::ModalInteraction,
    container: ContainerBuilder,
    ephemeral: bool,
) -> Result<(), Error> {
    let map = ComponentsV2Payload::new(vec![container.build()])
        .ephemeral(ephemeral)
        .into_map()?;

    let response_data = serde_json::json!({
        "type": 4,
        "data": map
    });

    http.create_interaction_response(interaction.id, &interaction.token, &response_data, vec![])
        .await?;

    Ok(())
}

pub async fn followup_with_container(
    http: &Http,
    interaction_token: &str,
    _application_id: u64,
    container: ContainerBuilder,
    ephemeral: bool,
) -> Result<serenity::Message, Error> {
    let map = ComponentsV2Payload::new(vec![container.build()])
        .ephemeral(ephemeral)
        .into_map()?;

    let result = http
        .create_followup_message(interaction_token, &map, vec![])
        .await?;
    Ok(result)
}

/// 기존 메시지를 Container로 수정 (edit)
pub async fn edit_message_with_container(
    http: &Http,
    channel_id: ChannelId,
    message_id: serenity::MessageId,
    container: ContainerBuilder,
) -> Result<serenity::Message, Error> {
    let map = ComponentsV2Payload::new(vec![container.build()]).into_map()?;

    let result = http
        .edit_message(channel_id, message_id, &map, vec![])
        .await?;
    Ok(result)
}

/// Component Interaction 응답을 메시지 수정(UPDATE)으로 처리
pub async fn update_component_with_container(
    http: &Http,
    interaction: &serenity::ComponentInteraction,
    container: ContainerBuilder,
) -> Result<(), Error> {
    let map = ComponentsV2Payload::new(vec![container.build()]).into_map()?;

    let response_data = serde_json::json!({
        "type": 7, // UPDATE_MESSAGE
        "data": map
    });

    http.create_interaction_response(interaction.id, &interaction.token, &response_data, vec![])
        .await?;

    Ok(())
}

/// Modal 응답으로 Components V2 ModalBuilder 전송
pub async fn respond_with_modal(
    http: &Http,
    interaction_id: serenity::InteractionId,
    interaction_token: &str,
    modal: ModalBuilder,
) -> Result<(), Error> {
    let response_data = serde_json::json!({
        "type": 9, // MODAL
        "data": modal.build()
    });

    http.create_interaction_response(interaction_id, interaction_token, &response_data, vec![])
        .await?;

    Ok(())
}

/// CommandInteraction에서 deferred 응답 후 Container로 followup (편의 함수)
pub async fn defer_and_followup_container(
    http: &Http,
    interaction: &serenity::CommandInteraction,
    container: ContainerBuilder,
    ephemeral: bool,
) -> Result<serenity::Message, Error> {
    let mut flags: u64 = 0;
    if ephemeral {
        flags |= 1 << 6;
    }

    let defer_data = serde_json::json!({
        "type": 5, // DEFERRED_CHANNEL_MESSAGE_WITH_SOURCE
        "data": { "flags": flags }
    });

    http.create_interaction_response(interaction.id, &interaction.token, &defer_data, vec![])
        .await?;

    followup_with_container(
        http,
        &interaction.token,
        interaction.application_id.get(),
        container,
        ephemeral,
    )
    .await
}

/// 톱레벨 컴포넌트 배열을 채널에 전송 (Container 없이 자유 배치)
pub async fn send_components_v2(
    http: &Http,
    channel_id: ChannelId,
    message: ComponentsV2Message,
) -> Result<serenity::Message, Error> {
    let map = ComponentsV2Payload::new(message.build()).into_map()?;

    let result = http.send_message(channel_id, vec![], &map).await?;
    Ok(result)
}

/// 톱레벨 컴포넌트 배열로 Interaction 응답
pub async fn respond_with_components_v2(
    http: &Http,
    interaction: &serenity::CommandInteraction,
    message: ComponentsV2Message,
    ephemeral: bool,
) -> Result<(), Error> {
    let map = ComponentsV2Payload::new(message.build())
        .ephemeral(ephemeral)
        .into_map()?;

    let response_data = serde_json::json!({
        "type": 4,
        "data": map
    });

    http.create_interaction_response(interaction.id, &interaction.token, &response_data, vec![])
        .await?;

    Ok(())
}

/// 톱레벨 컴포넌트 배열로 Component Interaction 응답
pub async fn respond_component_with_components_v2(
    http: &Http,
    interaction: &serenity::ComponentInteraction,
    message: ComponentsV2Message,
    ephemeral: bool,
) -> Result<(), Error> {
    let map = ComponentsV2Payload::new(message.build())
        .ephemeral(ephemeral)
        .into_map()?;

    let response_data = serde_json::json!({
        "type": 4,
        "data": map
    });

    http.create_interaction_response(interaction.id, &interaction.token, &response_data, vec![])
        .await?;

    Ok(())
}
