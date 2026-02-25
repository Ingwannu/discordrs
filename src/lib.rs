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

/// Content Inventory Entry 빌더 (type 16)
/// Discord 최신 컴포넌트 타입 상수를 실제로 사용할 수 있게 제공
#[derive(Clone, Serialize, Deserialize, Default)]
pub struct ContentInventoryEntryBuilder {
    #[serde(rename = "type")]
    component_type: u8,
    id: String,
}

impl ContentInventoryEntryBuilder {
    pub fn new(id: &str) -> Self {
        Self {
            component_type: component_type::CONTENT_INVENTORY_ENTRY,
            id: id.to_string(),
        }
    }

    pub fn build(self) -> Value {
        to_json_value(self)
    }
}

/// Slash command option type constants based on Discord API.
pub mod command_option_type {
    pub const SUB_COMMAND: u8 = 1;
    pub const SUB_COMMAND_GROUP: u8 = 2;
    pub const STRING: u8 = 3;
    pub const INTEGER: u8 = 4;
    pub const BOOLEAN: u8 = 5;
    pub const USER: u8 = 6;
    pub const CHANNEL: u8 = 7;
    pub const ROLE: u8 = 8;
    pub const MENTIONABLE: u8 = 9;
    pub const NUMBER: u8 = 10;
    pub const ATTACHMENT: u8 = 11;
}

#[derive(Clone, Serialize, Deserialize, Default)]
pub struct CommandOptionChoice {
    pub name: String,
    pub value: Value,
}

impl CommandOptionChoice {
    pub fn string(name: &str, value: &str) -> Self {
        Self {
            name: name.to_string(),
            value: Value::String(value.to_string()),
        }
    }

    pub fn integer(name: &str, value: i64) -> Self {
        Self {
            name: name.to_string(),
            value: Value::Number(value.into()),
        }
    }

    pub fn number(name: &str, value: f64) -> Self {
        let number =
            serde_json::Number::from_f64(value).expect("number choice value must be finite");
        Self {
            name: name.to_string(),
            value: Value::Number(number),
        }
    }
}

#[derive(Clone, Serialize, Deserialize, Default)]
pub struct CommandOptionBuilder {
    #[serde(rename = "type")]
    option_type: u8,
    name: String,
    description: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    required: Option<bool>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    choices: Vec<CommandOptionChoice>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    options: Vec<CommandOptionBuilder>,
    #[serde(skip_serializing_if = "Option::is_none")]
    channel_types: Option<Vec<u8>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    min_value: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_value: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    min_length: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_length: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    autocomplete: Option<bool>,
}

impl CommandOptionBuilder {
    pub fn new(option_type: u8, name: &str, description: &str) -> Self {
        Self {
            option_type,
            name: name.to_string(),
            description: description.to_string(),
            required: None,
            choices: Vec::new(),
            options: Vec::new(),
            channel_types: None,
            min_value: None,
            max_value: None,
            min_length: None,
            max_length: None,
            autocomplete: None,
        }
    }

    pub fn string(name: &str, description: &str) -> Self {
        Self::new(command_option_type::STRING, name, description)
    }

    pub fn integer(name: &str, description: &str) -> Self {
        Self::new(command_option_type::INTEGER, name, description)
    }

    pub fn boolean(name: &str, description: &str) -> Self {
        Self::new(command_option_type::BOOLEAN, name, description)
    }

    pub fn user(name: &str, description: &str) -> Self {
        Self::new(command_option_type::USER, name, description)
    }

    pub fn channel(name: &str, description: &str) -> Self {
        Self::new(command_option_type::CHANNEL, name, description)
    }

    pub fn role(name: &str, description: &str) -> Self {
        Self::new(command_option_type::ROLE, name, description)
    }

    pub fn mentionable(name: &str, description: &str) -> Self {
        Self::new(command_option_type::MENTIONABLE, name, description)
    }

    pub fn number(name: &str, description: &str) -> Self {
        Self::new(command_option_type::NUMBER, name, description)
    }

    pub fn attachment(name: &str, description: &str) -> Self {
        Self::new(command_option_type::ATTACHMENT, name, description)
    }

    pub fn sub_command(name: &str, description: &str) -> Self {
        Self::new(command_option_type::SUB_COMMAND, name, description)
    }

    pub fn sub_command_group(name: &str, description: &str) -> Self {
        Self::new(command_option_type::SUB_COMMAND_GROUP, name, description)
    }

    pub fn required(mut self, required: bool) -> Self {
        self.required = Some(required);
        self
    }

    pub fn add_choice(mut self, choice: CommandOptionChoice) -> Self {
        self.choices.push(choice);
        self
    }

    pub fn add_option(mut self, option: CommandOptionBuilder) -> Self {
        self.options.push(option);
        self
    }

    pub fn channel_types(mut self, channel_types: Vec<u8>) -> Self {
        self.channel_types = Some(channel_types);
        self
    }

    pub fn min_value_i64(mut self, min: i64) -> Self {
        self.min_value = Some(Value::Number(min.into()));
        self
    }

    pub fn max_value_i64(mut self, max: i64) -> Self {
        self.max_value = Some(Value::Number(max.into()));
        self
    }

    pub fn min_value_f64(mut self, min: f64) -> Self {
        self.min_value = Some(Value::Number(
            serde_json::Number::from_f64(min).expect("min value must be finite"),
        ));
        self
    }

    pub fn max_value_f64(mut self, max: f64) -> Self {
        self.max_value = Some(Value::Number(
            serde_json::Number::from_f64(max).expect("max value must be finite"),
        ));
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

    pub fn autocomplete(mut self, autocomplete: bool) -> Self {
        self.autocomplete = Some(autocomplete);
        self
    }

    pub fn build(self) -> Value {
        to_json_value(self)
    }
}

#[derive(Clone, Serialize, Deserialize, Default)]
pub struct SlashCommandBuilder {
    name: String,
    description: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    options: Vec<CommandOptionBuilder>,
    #[serde(skip_serializing_if = "Option::is_none")]
    dm_permission: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    default_member_permissions: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    nsfw: Option<bool>,
}

impl SlashCommandBuilder {
    pub fn new(name: &str, description: &str) -> Self {
        Self {
            name: name.to_string(),
            description: description.to_string(),
            options: Vec::new(),
            dm_permission: None,
            default_member_permissions: None,
            nsfw: None,
        }
    }

    pub fn add_option(mut self, option: CommandOptionBuilder) -> Self {
        self.options.push(option);
        self
    }

    pub fn dm_permission(mut self, allowed: bool) -> Self {
        self.dm_permission = Some(allowed);
        self
    }

    pub fn default_member_permissions(mut self, permissions_bitset: u64) -> Self {
        self.default_member_permissions = Some(permissions_bitset.to_string());
        self
    }

    pub fn nsfw(mut self, nsfw: bool) -> Self {
        self.nsfw = Some(nsfw);
        self
    }

    pub fn build(self) -> Value {
        to_json_value(self)
    }

    /// Command name used as a stable key when managing command collections.
    pub fn name(&self) -> &str {
        &self.name
    }
}

/// Ergonomic collection builder for slash command registration.
#[derive(Clone, Default)]
pub struct SlashCommandSet {
    commands: Vec<SlashCommandBuilder>,
}

impl SlashCommandSet {
    pub fn new() -> Self {
        Self {
            commands: Vec::new(),
        }
    }

    pub fn len(&self) -> usize {
        self.commands.len()
    }

    pub fn is_empty(&self) -> bool {
        self.commands.is_empty()
    }

    pub fn with_command(mut self, command: SlashCommandBuilder) -> Self {
        self.commands.push(command);
        self
    }

    pub fn with_commands<I>(mut self, commands: I) -> Self
    where
        I: IntoIterator<Item = SlashCommandBuilder>,
    {
        self.commands.extend(commands);
        self
    }

    pub fn push(&mut self, command: SlashCommandBuilder) {
        self.commands.push(command);
    }

    /// Insert-or-replace by command name.
    ///
    /// Returns the previously registered command with the same name when replaced.
    pub fn set_command(&mut self, command: SlashCommandBuilder) -> Option<SlashCommandBuilder> {
        if let Some(existing) = self
            .commands
            .iter_mut()
            .find(|existing| existing.name() == command.name())
        {
            return Some(std::mem::replace(existing, command));
        }

        self.commands.push(command);
        None
    }

    /// Builder-style insert-or-replace by command name.
    pub fn with_set_command(mut self, command: SlashCommandBuilder) -> Self {
        self.set_command(command);
        self
    }

    pub fn extend<I>(&mut self, commands: I)
    where
        I: IntoIterator<Item = SlashCommandBuilder>,
    {
        self.commands.extend(commands);
    }

    pub fn clear(&mut self) {
        self.commands.clear();
    }

    /// Iterate command names in insertion order.
    pub fn names(&self) -> impl Iterator<Item = &str> {
        self.commands.iter().map(SlashCommandBuilder::name)
    }

    /// Retain only commands that satisfy the predicate.
    pub fn retain<F>(&mut self, mut keep: F)
    where
        F: FnMut(&SlashCommandBuilder) -> bool,
    {
        self.commands.retain(|command| keep(command));
    }

    /// Check if a command with this name exists.
    pub fn contains(&self, name: &str) -> bool {
        self.commands.iter().any(|command| command.name() == name)
    }

    /// Remove a command by name.
    ///
    /// Returns the removed command when found.
    pub fn remove(&mut self, name: &str) -> Option<SlashCommandBuilder> {
        let index = self
            .commands
            .iter()
            .position(|command| command.name() == name)?;
        Some(self.commands.remove(index))
    }

    /// Build a bulk-overwrite payload while keeping this set intact.
    pub fn payload(&self) -> Vec<Value> {
        self.commands
            .clone()
            .into_iter()
            .map(SlashCommandBuilder::build)
            .collect()
    }

    pub fn into_payload(self) -> Vec<Value> {
        slash_command_registration_payload(self.commands)
    }

    pub async fn register_global(self, http: &Http) -> Result<Vec<serenity::Command>, Error> {
        register_global_slash_commands(http, self.commands).await
    }

    /// Register without consuming this set.
    pub async fn register_global_ref(&self, http: &Http) -> Result<Vec<serenity::Command>, Error> {
        register_global_slash_commands(http, self.commands.clone()).await
    }

    pub async fn register_guild(
        self,
        http: &Http,
        guild_id: serenity::GuildId,
    ) -> Result<Vec<serenity::Command>, Error> {
        register_guild_slash_commands(http, guild_id, self.commands).await
    }

    /// Register without consuming this set.
    pub async fn register_guild_ref(
        &self,
        http: &Http,
        guild_id: serenity::GuildId,
    ) -> Result<Vec<serenity::Command>, Error> {
        register_guild_slash_commands(http, guild_id, self.commands.clone()).await
    }

    pub async fn register(
        self,
        http: &Http,
        scope: SlashCommandScope,
    ) -> Result<Vec<serenity::Command>, Error> {
        register_slash_commands(http, scope, self.commands).await
    }

    /// Register without consuming this set.
    pub async fn register_ref(
        &self,
        http: &Http,
        scope: SlashCommandScope,
    ) -> Result<Vec<serenity::Command>, Error> {
        register_slash_commands(http, scope, self.commands.clone()).await
    }
}

impl From<Vec<SlashCommandBuilder>> for SlashCommandSet {
    fn from(commands: Vec<SlashCommandBuilder>) -> Self {
        Self { commands }
    }
}

impl FromIterator<SlashCommandBuilder> for SlashCommandSet {
    fn from_iter<T: IntoIterator<Item = SlashCommandBuilder>>(iter: T) -> Self {
        Self {
            commands: iter.into_iter().collect(),
        }
    }
}

impl Extend<SlashCommandBuilder> for SlashCommandSet {
    fn extend<T: IntoIterator<Item = SlashCommandBuilder>>(&mut self, iter: T) {
        self.commands.extend(iter);
    }
}

impl IntoIterator for SlashCommandSet {
    type Item = SlashCommandBuilder;
    type IntoIter = std::vec::IntoIter<SlashCommandBuilder>;

    fn into_iter(self) -> Self::IntoIter {
        self.commands.into_iter()
    }
}

impl<'a> IntoIterator for &'a SlashCommandSet {
    type Item = &'a SlashCommandBuilder;
    type IntoIter = std::slice::Iter<'a, SlashCommandBuilder>;

    fn into_iter(self) -> Self::IntoIter {
        self.commands.iter()
    }
}

/// Bulk overwrite payload for global/guild slash command registration.
pub fn slash_command_registration_payload<I>(commands: I) -> Vec<Value>
where
    I: IntoIterator<Item = SlashCommandBuilder>,
{
    commands
        .into_iter()
        .map(SlashCommandBuilder::build)
        .collect()
}

/// Register global slash commands via Discord bulk overwrite endpoint.
///
/// This wraps serenity's raw HTTP call so you can keep using `SlashCommandBuilder`.
pub async fn register_global_slash_commands<I>(
    http: &Http,
    commands: I,
) -> Result<Vec<serenity::Command>, Error>
where
    I: IntoIterator<Item = SlashCommandBuilder>,
{
    let payload = slash_command_registration_payload(commands);
    let created = http.create_global_commands(&payload).await?;
    Ok(created)
}

/// Register guild slash commands via Discord bulk overwrite endpoint.
///
/// Useful for fast iteration since guild commands update quickly.
pub async fn register_guild_slash_commands<I>(
    http: &Http,
    guild_id: serenity::GuildId,
    commands: I,
) -> Result<Vec<serenity::Command>, Error>
where
    I: IntoIterator<Item = SlashCommandBuilder>,
{
    let payload = slash_command_registration_payload(commands);
    let created = http.create_guild_commands(guild_id, &payload).await?;
    Ok(created)
}

/// Target scope for slash command bulk registration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SlashCommandScope {
    Global,
    Guild(serenity::GuildId),
}

/// Register slash commands for either global or guild scope.
pub async fn register_slash_commands<I>(
    http: &Http,
    scope: SlashCommandScope,
    commands: I,
) -> Result<Vec<serenity::Command>, Error>
where
    I: IntoIterator<Item = SlashCommandBuilder>,
{
    match scope {
        SlashCommandScope::Global => register_global_slash_commands(http, commands).await,
        SlashCommandScope::Guild(guild_id) => {
            register_guild_slash_commands(http, guild_id, commands).await
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DispatchKind {
    Command,
    Component,
    Modal,
}

#[derive(Clone, PartialEq, Eq)]
enum RoutePattern {
    Exact(String),
    Prefix(String),
}

#[derive(Clone)]
struct Route<T> {
    kind: DispatchKind,
    pattern: RoutePattern,
    value: T,
}

pub struct DispatchMatch<'a, T> {
    pub kind: DispatchKind,
    pub key: &'a str,
    pub value: &'a T,
}

/// Simple route table for command/component/modal dispatch.
///
/// - `insert_*`: append routes without replacing existing ones.
/// - `set_*`: upsert (replace same kind+pattern route if it exists).
/// - `remove_*`: remove exact kind+pattern routes.
pub struct InteractionRouter<T> {
    routes: Vec<Route<T>>,
    command_fallback: Option<T>,
    component_fallback: Option<T>,
    modal_fallback: Option<T>,
}

impl<T> Default for InteractionRouter<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> InteractionRouter<T> {
    fn push_route(&mut self, kind: DispatchKind, pattern: RoutePattern, value: T) {
        self.routes.push(Route {
            kind,
            pattern,
            value,
        });
    }

    fn set_route(&mut self, kind: DispatchKind, pattern: RoutePattern, value: T) {
        if let Some(route) = self
            .routes
            .iter_mut()
            .find(|route| route.kind == kind && route.pattern == pattern)
        {
            route.value = value;
            return;
        }

        self.push_route(kind, pattern, value);
    }

    fn remove_route(&mut self, kind: DispatchKind, pattern: RoutePattern) -> bool {
        if let Some(idx) = self
            .routes
            .iter()
            .position(|route| route.kind == kind && route.pattern == pattern)
        {
            self.routes.remove(idx);
            true
        } else {
            false
        }
    }

    pub fn new() -> Self {
        Self {
            routes: Vec::new(),
            command_fallback: None,
            component_fallback: None,
            modal_fallback: None,
        }
    }

    pub fn len(&self) -> usize {
        self.routes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.routes.is_empty()
    }

    pub fn clear(&mut self) {
        self.routes.clear();
        self.command_fallback = None;
        self.component_fallback = None;
        self.modal_fallback = None;
    }

    pub fn set_command_fallback(&mut self, value: T) -> Option<T> {
        self.command_fallback.replace(value)
    }

    pub fn remove_command_fallback(&mut self) -> Option<T> {
        self.command_fallback.take()
    }

    pub fn has_command_fallback(&self) -> bool {
        self.command_fallback.is_some()
    }

    pub fn set_component_fallback(&mut self, value: T) -> Option<T> {
        self.component_fallback.replace(value)
    }

    pub fn remove_component_fallback(&mut self) -> Option<T> {
        self.component_fallback.take()
    }

    pub fn has_component_fallback(&self) -> bool {
        self.component_fallback.is_some()
    }

    pub fn set_modal_fallback(&mut self, value: T) -> Option<T> {
        self.modal_fallback.replace(value)
    }

    pub fn remove_modal_fallback(&mut self) -> Option<T> {
        self.modal_fallback.take()
    }

    pub fn has_modal_fallback(&self) -> bool {
        self.modal_fallback.is_some()
    }

    pub fn insert_command(&mut self, name: &str, value: T) {
        self.push_route(
            DispatchKind::Command,
            RoutePattern::Exact(name.to_string()),
            value,
        );
    }

    pub fn set_command(&mut self, name: &str, value: T) {
        self.set_route(
            DispatchKind::Command,
            RoutePattern::Exact(name.to_string()),
            value,
        );
    }

    pub fn remove_command(&mut self, name: &str) -> bool {
        self.remove_route(DispatchKind::Command, RoutePattern::Exact(name.to_string()))
    }

    pub fn on_command(mut self, name: &str, value: T) -> Self {
        self.insert_command(name, value);
        self
    }

    pub fn insert_component(&mut self, custom_id: &str, value: T) {
        self.push_route(
            DispatchKind::Component,
            RoutePattern::Exact(custom_id.to_string()),
            value,
        );
    }

    pub fn set_component(&mut self, custom_id: &str, value: T) {
        self.set_route(
            DispatchKind::Component,
            RoutePattern::Exact(custom_id.to_string()),
            value,
        );
    }

    pub fn remove_component(&mut self, custom_id: &str) -> bool {
        self.remove_route(
            DispatchKind::Component,
            RoutePattern::Exact(custom_id.to_string()),
        )
    }

    pub fn on_component(mut self, custom_id: &str, value: T) -> Self {
        self.insert_component(custom_id, value);
        self
    }

    pub fn insert_component_prefix(&mut self, prefix: &str, value: T) {
        self.push_route(
            DispatchKind::Component,
            RoutePattern::Prefix(prefix.to_string()),
            value,
        );
    }

    pub fn set_component_prefix(&mut self, prefix: &str, value: T) {
        self.set_route(
            DispatchKind::Component,
            RoutePattern::Prefix(prefix.to_string()),
            value,
        );
    }

    pub fn remove_component_prefix(&mut self, prefix: &str) -> bool {
        self.remove_route(
            DispatchKind::Component,
            RoutePattern::Prefix(prefix.to_string()),
        )
    }

    pub fn on_component_prefix(mut self, prefix: &str, value: T) -> Self {
        self.insert_component_prefix(prefix, value);
        self
    }

    pub fn insert_modal(&mut self, custom_id: &str, value: T) {
        self.push_route(
            DispatchKind::Modal,
            RoutePattern::Exact(custom_id.to_string()),
            value,
        );
    }

    pub fn set_modal(&mut self, custom_id: &str, value: T) {
        self.set_route(
            DispatchKind::Modal,
            RoutePattern::Exact(custom_id.to_string()),
            value,
        );
    }

    pub fn remove_modal(&mut self, custom_id: &str) -> bool {
        self.remove_route(
            DispatchKind::Modal,
            RoutePattern::Exact(custom_id.to_string()),
        )
    }

    pub fn on_modal(mut self, custom_id: &str, value: T) -> Self {
        self.insert_modal(custom_id, value);
        self
    }

    pub fn insert_modal_prefix(&mut self, prefix: &str, value: T) {
        self.push_route(
            DispatchKind::Modal,
            RoutePattern::Prefix(prefix.to_string()),
            value,
        );
    }

    pub fn set_modal_prefix(&mut self, prefix: &str, value: T) {
        self.set_route(
            DispatchKind::Modal,
            RoutePattern::Prefix(prefix.to_string()),
            value,
        );
    }

    pub fn remove_modal_prefix(&mut self, prefix: &str) -> bool {
        self.remove_route(
            DispatchKind::Modal,
            RoutePattern::Prefix(prefix.to_string()),
        )
    }

    pub fn on_modal_prefix(mut self, prefix: &str, value: T) -> Self {
        self.insert_modal_prefix(prefix, value);
        self
    }

    pub fn with_command_fallback(mut self, value: T) -> Self {
        self.command_fallback = Some(value);
        self
    }

    pub fn with_component_fallback(mut self, value: T) -> Self {
        self.component_fallback = Some(value);
        self
    }

    pub fn with_modal_fallback(mut self, value: T) -> Self {
        self.modal_fallback = Some(value);
        self
    }

    pub fn resolve_command(&self, name: &str) -> Option<&T> {
        self.resolve(DispatchKind::Command, name)
    }

    pub fn contains_command(&self, name: &str) -> bool {
        self.resolve_command(name).is_some()
    }

    pub fn resolve_component(&self, custom_id: &str) -> Option<&T> {
        self.resolve(DispatchKind::Component, custom_id)
    }

    pub fn contains_component(&self, custom_id: &str) -> bool {
        self.resolve_component(custom_id).is_some()
    }

    pub fn resolve_modal(&self, custom_id: &str) -> Option<&T> {
        self.resolve(DispatchKind::Modal, custom_id)
    }

    pub fn contains_modal(&self, custom_id: &str) -> bool {
        self.resolve_modal(custom_id).is_some()
    }

    pub fn resolve_interaction(&self, interaction: &serenity::Interaction) -> Option<&T> {
        let (kind, key) = interaction_dispatch_key(interaction)?;
        self.resolve(kind, key)
    }

    pub fn resolve_interaction_match<'a>(
        &'a self,
        interaction: &'a serenity::Interaction,
    ) -> Option<DispatchMatch<'a, T>> {
        let (kind, key) = interaction_dispatch_key(interaction)?;
        self.resolve_match(kind, key)
    }

    pub fn resolve(&self, kind: DispatchKind, key: &str) -> Option<&T> {
        self.resolve_route(kind, key)
            .map(|route| &route.value)
            .or_else(|| self.fallback(kind))
    }

    pub fn resolve_match<'a>(
        &'a self,
        kind: DispatchKind,
        key: &'a str,
    ) -> Option<DispatchMatch<'a, T>> {
        self.resolve_route(kind, key)
            .map(|route| DispatchMatch {
                kind,
                key,
                value: &route.value,
            })
            .or_else(|| {
                self.fallback(kind)
                    .map(|value| DispatchMatch { kind, key, value })
            })
    }

    fn fallback(&self, kind: DispatchKind) -> Option<&T> {
        match kind {
            DispatchKind::Command => self.command_fallback.as_ref(),
            DispatchKind::Component => self.component_fallback.as_ref(),
            DispatchKind::Modal => self.modal_fallback.as_ref(),
        }
    }

    fn resolve_route(&self, kind: DispatchKind, key: &str) -> Option<&Route<T>> {
        let exact = self.routes.iter().rfind(|route| {
            route.kind == kind
                && matches!(&route.pattern, RoutePattern::Exact(exact) if exact == key)
        });
        if exact.is_some() {
            return exact;
        }

        self.routes
            .iter()
            .enumerate()
            .filter(|route| {
                route.1.kind == kind
                    && matches!(&route.1.pattern, RoutePattern::Prefix(prefix) if key.starts_with(prefix))
            })
            // Prefer the longest prefix; for ties, prefer the most recently inserted route.
            .max_by_key(|(index, route)| match &route.pattern {
                RoutePattern::Prefix(prefix) => (prefix.len(), *index),
                RoutePattern::Exact(_) => (0, *index),
            })
            .map(|(_, route)| route)
    }
}

pub fn interaction_dispatch_key(
    interaction: &serenity::Interaction,
) -> Option<(DispatchKind, &str)> {
    match interaction {
        serenity::Interaction::Command(command) => {
            Some((DispatchKind::Command, command.data.name.as_str()))
        }
        serenity::Interaction::Component(component) => {
            Some((DispatchKind::Component, component.data.custom_id.as_str()))
        }
        serenity::Interaction::Modal(modal) => {
            Some((DispatchKind::Modal, modal.data.custom_id.as_str()))
        }
        _ => None,
    }
}

pub fn dispatch_interaction<'a, T>(
    router: &'a InteractionRouter<T>,
    interaction: &serenity::Interaction,
) -> Option<&'a T> {
    router.resolve_interaction(interaction)
}

pub fn dispatch_interaction_match<'a, T>(
    router: &'a InteractionRouter<T>,
    interaction: &'a serenity::Interaction,
) -> Option<DispatchMatch<'a, T>> {
    router.resolve_interaction_match(interaction)
}

pub mod channel_type {
    pub const GUILD_TEXT: u8 = 0;
    pub const DM: u8 = 1;
    pub const GUILD_VOICE: u8 = 2;
    pub const GROUP_DM: u8 = 3;
    pub const GUILD_CATEGORY: u8 = 4;
    pub const GUILD_ANNOUNCEMENT: u8 = 5;
    pub const ANNOUNCEMENT_THREAD: u8 = 10;
    pub const PUBLIC_THREAD: u8 = 11;
    pub const PRIVATE_THREAD: u8 = 12;
    pub const GUILD_STAGE_VOICE: u8 = 13;
    pub const GUILD_FORUM: u8 = 15;
    pub const GUILD_MEDIA: u8 = 16;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_container_chunks_buttons_into_rows_of_five() {
        let buttons = (0..7)
            .map(|i| ButtonConfig::new(&format!("id_{i}"), &format!("btn_{i}")))
            .collect::<Vec<_>>();

        let v = create_container("title", "desc", buttons, None).build();
        let components = v
            .get("components")
            .and_then(Value::as_array)
            .expect("components should be array");

        let action_rows = components
            .iter()
            .filter(|c| {
                c.get("type").and_then(Value::as_u64) == Some(component_type::ACTION_ROW as u64)
            })
            .collect::<Vec<_>>();

        assert_eq!(action_rows.len(), 2);

        let first_row_count = action_rows[0]
            .get("components")
            .and_then(Value::as_array)
            .map(|v| v.len())
            .unwrap_or(0);
        let second_row_count = action_rows[1]
            .get("components")
            .and_then(Value::as_array)
            .map(|v| v.len())
            .unwrap_or(0);

        assert_eq!(first_row_count, 5);
        assert_eq!(second_row_count, 2);
    }

    #[test]
    fn modal_radio_checkbox_builds_labels() {
        let modal = ModalBuilder::new("prefs", "Preferences")
            .add_radio_group(
                "Theme",
                Some("pick one"),
                RadioGroupBuilder::new("theme").add_option(SelectOption::new("Dark", "dark")),
            )
            .add_checkbox("Agree", None, CheckboxBuilder::new("agree").required(true))
            .build();

        let components = modal
            .get("components")
            .and_then(Value::as_array)
            .expect("modal components should be array");

        assert_eq!(components.len(), 2);
        for c in components {
            assert_eq!(
                c.get("type").and_then(Value::as_u64),
                Some(component_type::LABEL as u64)
            );
        }
    }

    #[test]
    fn components_v2_payload_sets_flags() {
        let map = ComponentsV2Payload::new(vec![])
            .ephemeral(true)
            .into_map()
            .expect("payload map");

        let flags = map
            .get("flags")
            .and_then(Value::as_u64)
            .expect("flags should be u64");

        assert_eq!(
            flags & MESSAGE_FLAG_IS_COMPONENTS_V2,
            MESSAGE_FLAG_IS_COMPONENTS_V2
        );
        assert_eq!(flags & (1 << 6), 1 << 6);
    }

    #[test]
    fn slash_command_registration_payload_builds_expected_shape() {
        let payload = slash_command_registration_payload(vec![SlashCommandBuilder::new(
            "ping",
            "Latency check",
        )
        .dm_permission(false)
        .add_option(
            CommandOptionBuilder::string("target", "who to ping")
                .required(true)
                .min_length(2)
                .max_length(16)
                .add_choice(CommandOptionChoice::string("all", "all")),
        )]);

        assert_eq!(payload.len(), 1);
        assert_eq!(payload[0].get("name").and_then(Value::as_str), Some("ping"));
        assert_eq!(
            payload[0].get("description").and_then(Value::as_str),
            Some("Latency check")
        );
        assert_eq!(
            payload[0].get("dm_permission").and_then(Value::as_bool),
            Some(false)
        );

        let options = payload[0]
            .get("options")
            .and_then(Value::as_array)
            .expect("options array");
        assert_eq!(options.len(), 1);
        assert_eq!(
            options[0].get("name").and_then(Value::as_str),
            Some("target")
        );
    }

    #[test]
    fn interaction_router_prefers_exact_then_longest_prefix() {
        let router = InteractionRouter::new()
            .on_component_prefix("ticket:", 1)
            .on_component_prefix("ticket:close:", 2)
            .on_component("ticket:close:now", 3)
            .on_command("ping", 4);

        assert_eq!(
            router.resolve(DispatchKind::Component, "ticket:close:now"),
            Some(&3)
        );
        assert_eq!(
            router.resolve(DispatchKind::Component, "ticket:close:later"),
            Some(&2)
        );
        assert_eq!(
            router.resolve(DispatchKind::Component, "ticket:open:later"),
            Some(&1)
        );
        assert_eq!(router.resolve(DispatchKind::Command, "ping"), Some(&4));
        assert_eq!(router.resolve(DispatchKind::Modal, "ticket:open"), None);
    }

    #[test]
    fn interaction_router_insert_len_and_clear() {
        let mut router = InteractionRouter::new();
        assert!(router.is_empty());

        router.insert_command("ping", 1);
        router.insert_component_prefix("ticket:", 2);
        router.insert_modal("prefs", 3);

        assert_eq!(router.len(), 3);
        assert_eq!(router.resolve(DispatchKind::Command, "ping"), Some(&1));
        assert_eq!(router.resolve_command("ping"), Some(&1));
        assert!(router.contains_command("ping"));
        assert_eq!(
            router.resolve(DispatchKind::Component, "ticket:new"),
            Some(&2)
        );
        assert_eq!(router.resolve_component("ticket:new"), Some(&2));
        assert!(router.contains_component("ticket:new"));
        assert_eq!(router.resolve(DispatchKind::Modal, "prefs"), Some(&3));
        assert_eq!(router.resolve_modal("prefs"), Some(&3));
        assert!(router.contains_modal("prefs"));

        router.clear();
        assert!(router.is_empty());
    }

    #[test]
    fn slash_registration_payload_empty_is_valid() {
        let payload = slash_command_registration_payload(vec![]);
        assert!(payload.is_empty());
    }

    #[test]
    fn slash_command_set_builds_payload_and_supports_clear() {
        let mut set = SlashCommandSet::new()
            .with_command(SlashCommandBuilder::new("ping", "Latency check"))
            .with_command(SlashCommandBuilder::new("echo", "Echo input"));

        assert_eq!(set.len(), 2);
        assert!(!set.is_empty());

        set.push(SlashCommandBuilder::new("about", "About bot"));
        assert_eq!(set.len(), 3);

        let payload = set.payload();
        assert_eq!(payload.len(), 3);
        assert_eq!(payload[0].get("name").and_then(Value::as_str), Some("ping"));

        let into_payload = set.clone().into_payload();
        assert_eq!(into_payload.len(), 3);

        set.clear();
        assert!(set.is_empty());
    }

    #[test]
    fn slash_command_set_supports_name_based_upsert_and_remove() {
        let mut set = SlashCommandSet::new()
            .with_command(SlashCommandBuilder::new("ping", "Latency check"))
            .with_set_command(SlashCommandBuilder::new("echo", "Echo input"));

        assert!(set.contains("ping"));
        assert!(set.contains("echo"));
        assert!(!set.contains("about"));

        let replaced = set
            .set_command(SlashCommandBuilder::new("ping", "Updated ping"))
            .expect("ping should be replaced");
        assert_eq!(replaced.name(), "ping");
        assert_eq!(set.len(), 2);

        let payload = set.payload();
        assert_eq!(payload.len(), 2);
        assert_eq!(payload[0].get("name").and_then(Value::as_str), Some("ping"));
        assert_eq!(
            payload[0].get("description").and_then(Value::as_str),
            Some("Updated ping")
        );

        let removed = set.remove("echo").expect("echo should be removed");
        assert_eq!(removed.name(), "echo");
        assert!(!set.contains("echo"));
        assert_eq!(set.len(), 1);
        assert!(set.remove("missing").is_none());
    }

    #[test]
    fn slash_command_set_supports_bulk_builders() {
        let extras = vec![
            SlashCommandBuilder::new("about", "About bot"),
            SlashCommandBuilder::new("help", "Help"),
        ];

        let mut set = SlashCommandSet::new()
            .with_command(SlashCommandBuilder::new("ping", "Latency check"))
            .with_commands(extras.clone());

        assert_eq!(set.len(), 3);

        set.extend(vec![SlashCommandBuilder::new("echo", "Echo")]);
        assert_eq!(set.len(), 4);

        let from_iter: SlashCommandSet = extras.clone().into_iter().collect();
        assert_eq!(from_iter.len(), 2);

        let from_vec = SlashCommandSet::from(extras);
        assert_eq!(from_vec.len(), 2);
    }

    #[test]
    fn slash_command_set_names_and_retain_are_ergonomic() {
        let mut set = SlashCommandSet::new()
            .with_command(SlashCommandBuilder::new("ping", "Latency"))
            .with_command(SlashCommandBuilder::new("echo", "Echo"))
            .with_command(SlashCommandBuilder::new("admin-ban", "Ban member"));

        let names = set.names().collect::<Vec<_>>();
        assert_eq!(names, vec!["ping", "echo", "admin-ban"]);

        set.retain(|command| !command.name().starts_with("admin-"));
        assert_eq!(set.names().collect::<Vec<_>>(), vec!["ping", "echo"]);
    }

    #[test]
    fn slash_command_set_supports_std_extend_and_into_iter() {
        let mut set = SlashCommandSet::new();
        set.extend([
            SlashCommandBuilder::new("ping", "Latency"),
            SlashCommandBuilder::new("echo", "Echo"),
        ]);

        let names = set
            .clone()
            .into_iter()
            .map(|command| {
                command.build()["name"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string()
            })
            .collect::<Vec<_>>();
        assert_eq!(names, vec!["ping", "echo"]);

        let borrowed_count = (&set).into_iter().count();
        assert_eq!(borrowed_count, 2);
    }

    #[test]
    fn slash_command_scope_is_copy_and_eq() {
        let guild_id = serenity::GuildId::new(42);
        assert_eq!(SlashCommandScope::Global, SlashCommandScope::Global);
        assert_eq!(
            SlashCommandScope::Guild(guild_id),
            SlashCommandScope::Guild(guild_id)
        );
    }

    #[test]
    fn interaction_router_set_methods_upsert_existing_routes() {
        let mut router = InteractionRouter::new();

        router.insert_command("ping", 1);
        router.set_command("ping", 2);

        router.insert_component("ticket:open", 10);
        router.set_component("ticket:open", 11);

        router.insert_component_prefix("ticket:", 20);
        router.set_component_prefix("ticket:", 21);

        router.insert_modal("prefs", 30);
        router.set_modal("prefs", 31);

        router.insert_modal_prefix("prefs:", 40);
        router.set_modal_prefix("prefs:", 41);

        assert_eq!(router.resolve_command("ping"), Some(&2));
        assert_eq!(router.resolve_component("ticket:open"), Some(&11));
        assert_eq!(router.resolve_component("ticket:new"), Some(&21));
        assert_eq!(router.resolve_modal("prefs"), Some(&31));
        assert_eq!(router.resolve_modal("prefs:general"), Some(&41));

        assert_eq!(router.len(), 5);
    }

    #[test]
    fn interaction_router_prefers_latest_route_for_same_specificity() {
        let mut router = InteractionRouter::new();

        router.insert_command("ping", 1);
        router.insert_command("ping", 2); // same exact key, inserted later

        router.insert_component_prefix("ticket:", 10);
        router.insert_component_prefix("ticket:", 11); // same prefix length, inserted later

        assert_eq!(router.resolve_command("ping"), Some(&2));
        assert_eq!(router.resolve_component("ticket:new"), Some(&11));
    }

    #[test]
    fn interaction_router_remove_methods_delete_matching_routes() {
        let mut router = InteractionRouter::new()
            .on_command("ping", 1)
            .on_component("ticket:open", 2)
            .on_component_prefix("ticket:", 3)
            .on_modal("prefs", 4)
            .on_modal_prefix("prefs:", 5);

        assert!(router.remove_command("ping"));
        assert!(router.remove_component("ticket:open"));
        assert!(router.remove_component_prefix("ticket:"));
        assert!(router.remove_modal("prefs"));
        assert!(router.remove_modal_prefix("prefs:"));

        assert!(!router.remove_command("ping"));
        assert!(router.is_empty());
    }

    #[test]
    fn interaction_router_fallbacks_resolve_and_clear() {
        let mut router = InteractionRouter::new()
            .on_command("ping", 1)
            .with_command_fallback(10)
            .with_component_fallback(20)
            .with_modal_fallback(30);

        assert!(router.has_command_fallback());
        assert!(router.has_component_fallback());
        assert!(router.has_modal_fallback());

        assert_eq!(router.resolve_command("unknown"), Some(&10));
        assert_eq!(router.resolve_component("ticket:unknown"), Some(&20));
        assert_eq!(router.resolve_modal("prefs:unknown"), Some(&30));

        assert_eq!(router.set_command_fallback(11), Some(10));
        assert_eq!(router.resolve_command("still-unknown"), Some(&11));
        assert_eq!(router.remove_component_fallback(), Some(20));
        assert!(!router.has_component_fallback());
        assert_eq!(router.resolve_component("ticket:unknown"), None);

        router.clear();
        assert!(router.is_empty());
        assert!(!router.has_command_fallback());
        assert!(!router.has_modal_fallback());
    }

    #[test]
    fn interaction_router_resolve_match_keeps_kind_and_key() {
        let router = InteractionRouter::new()
            .on_command("ping", 10)
            .on_modal_prefix("prefs:", 20)
            .with_component_fallback(99);

        let command = router
            .resolve_match(DispatchKind::Command, "ping")
            .expect("command route");
        assert_eq!(command.kind, DispatchKind::Command);
        assert_eq!(command.key, "ping");
        assert_eq!(*command.value, 10);

        let modal = router
            .resolve_match(DispatchKind::Modal, "prefs:general")
            .expect("modal route");
        assert_eq!(modal.kind, DispatchKind::Modal);
        assert_eq!(modal.key, "prefs:general");
        assert_eq!(*modal.value, 20);

        let fallback = router
            .resolve_match(DispatchKind::Component, "ticket:missing")
            .expect("component fallback");
        assert_eq!(fallback.kind, DispatchKind::Component);
        assert_eq!(fallback.key, "ticket:missing");
        assert_eq!(*fallback.value, 99);
    }
}
