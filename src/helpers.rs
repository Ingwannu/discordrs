use serde_json::Value;

use crate::builders::{create_container, ComponentsV2Message, ContainerBuilder, ModalBuilder};
use crate::constants::MESSAGE_FLAG_IS_COMPONENTS_V2;
use crate::error::DiscordError;
use crate::http::DiscordHttpClient;
use crate::model::{
    ApplicationCommandOptionChoice, CreateMessage, InteractionCallbackResponse,
    InteractionContextData, Message, Snowflake,
};
use crate::types::ButtonConfig;

pub const INTERACTION_RESPONSE_CHANNEL_MESSAGE: u8 = 4;
pub const INTERACTION_RESPONSE_DEFERRED_CHANNEL_MESSAGE: u8 = 5;
pub const INTERACTION_RESPONSE_DEFERRED_UPDATE_MESSAGE: u8 = 6;
pub const INTERACTION_RESPONSE_UPDATE_MESSAGE: u8 = 7;
pub const INTERACTION_RESPONSE_AUTOCOMPLETE_RESULT: u8 = 8;
pub const INTERACTION_RESPONSE_MODAL: u8 = 9;
pub const INTERACTION_RESPONSE_LAUNCH_ACTIVITY: u8 = 12;

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

    fn into_value(self) -> Value {
        serde_json::json!({
            "components": self.components,
            "flags": components_v2_flags(self.ephemeral),
        })
    }
}

pub async fn send_container_message(
    http: &DiscordHttpClient,
    channel_id: u64,
    container: ContainerBuilder,
) -> Result<Value, DiscordError> {
    let body = ComponentsV2Payload::new(vec![container.build()]).into_value();
    http.send_message_json(channel_id, &body).await
}

pub async fn send_message(
    http: &DiscordHttpClient,
    channel_id: impl Into<Snowflake>,
    body: &CreateMessage,
) -> Result<Message, DiscordError> {
    http.create_message(channel_id, body).await
}

pub async fn send_to_channel(
    http: &DiscordHttpClient,
    channel_id: u64,
    title: &str,
    description: &str,
    buttons: Vec<ButtonConfig>,
    image_url: Option<&str>,
) -> Result<Value, DiscordError> {
    let container = create_container(title, description, buttons, image_url);
    send_container_message(http, channel_id, container).await
}

pub async fn respond_with_container(
    http: &DiscordHttpClient,
    interaction_id: &str,
    interaction_token: &str,
    container: ContainerBuilder,
    ephemeral: bool,
) -> Result<(), DiscordError> {
    let data = ComponentsV2Payload::new(vec![container.build()])
        .ephemeral(ephemeral)
        .into_value();
    let response = serde_json::json!({
        "type": INTERACTION_RESPONSE_CHANNEL_MESSAGE,
        "data": data,
    });

    http.create_interaction_response_json(interaction_id, interaction_token, &response)
        .await
}

pub async fn respond_to_interaction(
    http: &DiscordHttpClient,
    context: &InteractionContextData,
    response: InteractionCallbackResponse,
) -> Result<(), DiscordError> {
    http.create_interaction_response_typed(context.id.clone(), &context.token, &response)
        .await
}

pub async fn respond_with_message(
    http: &DiscordHttpClient,
    context: &InteractionContextData,
    mut message: CreateMessage,
    ephemeral: bool,
) -> Result<(), DiscordError> {
    if ephemeral {
        message.flags = Some(message.flags.unwrap_or(0) | (1 << 6));
    }

    respond_to_interaction(
        http,
        context,
        InteractionCallbackResponse {
            kind: INTERACTION_RESPONSE_CHANNEL_MESSAGE,
            data: Some(serde_json::to_value(message)?),
        },
    )
    .await
}

pub async fn respond_component_with_container(
    http: &DiscordHttpClient,
    interaction_id: &str,
    interaction_token: &str,
    container: ContainerBuilder,
    ephemeral: bool,
) -> Result<(), DiscordError> {
    let data = ComponentsV2Payload::new(vec![container.build()])
        .ephemeral(ephemeral)
        .into_value();
    let response = serde_json::json!({
        "type": INTERACTION_RESPONSE_CHANNEL_MESSAGE,
        "data": data,
    });

    http.create_interaction_response_json(interaction_id, interaction_token, &response)
        .await
}

pub async fn respond_modal_with_container(
    http: &DiscordHttpClient,
    interaction_id: &str,
    interaction_token: &str,
    container: ContainerBuilder,
    ephemeral: bool,
) -> Result<(), DiscordError> {
    let data = ComponentsV2Payload::new(vec![container.build()])
        .ephemeral(ephemeral)
        .into_value();
    let response = serde_json::json!({
        "type": INTERACTION_RESPONSE_CHANNEL_MESSAGE,
        "data": data,
    });

    http.create_interaction_response_json(interaction_id, interaction_token, &response)
        .await
}

pub async fn followup_with_container(
    http: &DiscordHttpClient,
    interaction_token: &str,
    container: ContainerBuilder,
    ephemeral: bool,
) -> Result<Value, DiscordError> {
    let body = ComponentsV2Payload::new(vec![container.build()])
        .ephemeral(ephemeral)
        .into_value();
    http.create_followup_message_json(interaction_token, &body)
        .await
}

pub async fn followup_message(
    http: &DiscordHttpClient,
    context: &InteractionContextData,
    mut message: CreateMessage,
    ephemeral: bool,
) -> Result<Message, DiscordError> {
    if ephemeral {
        message.flags = Some(message.flags.unwrap_or(0) | (1 << 6));
    }

    http.create_followup_message_with_application_id(
        context.application_id.as_str(),
        &context.token,
        &message,
    )
    .await
}

pub async fn get_original_response(
    http: &DiscordHttpClient,
    context: &InteractionContextData,
) -> Result<Message, DiscordError> {
    http.get_original_interaction_response_with_application_id(
        context.application_id.as_str(),
        &context.token,
    )
    .await
}

pub async fn edit_original_response(
    http: &DiscordHttpClient,
    context: &InteractionContextData,
    message: CreateMessage,
) -> Result<Message, DiscordError> {
    http.edit_original_interaction_response_with_application_id(
        context.application_id.as_str(),
        &context.token,
        &message,
    )
    .await
}

pub async fn delete_original_response(
    http: &DiscordHttpClient,
    context: &InteractionContextData,
) -> Result<(), DiscordError> {
    http.delete_original_interaction_response_with_application_id(
        context.application_id.as_str(),
        &context.token,
    )
    .await
}

pub async fn delete_followup_response(
    http: &DiscordHttpClient,
    context: &InteractionContextData,
    message_id: &str,
) -> Result<(), DiscordError> {
    http.delete_followup_message_with_application_id(
        context.application_id.as_str(),
        &context.token,
        message_id,
    )
    .await
}

pub async fn edit_followup_response(
    http: &DiscordHttpClient,
    context: &InteractionContextData,
    message_id: &str,
    message: CreateMessage,
) -> Result<Message, DiscordError> {
    http.edit_followup_message_with_application_id(
        context.application_id.as_str(),
        &context.token,
        message_id,
        &message,
    )
    .await
}

pub async fn edit_message_with_container(
    http: &DiscordHttpClient,
    channel_id: u64,
    message_id: u64,
    container: ContainerBuilder,
) -> Result<Value, DiscordError> {
    let body = ComponentsV2Payload::new(vec![container.build()]).into_value();
    http.edit_message_json(channel_id, message_id, &body).await
}

pub async fn update_component_with_container(
    http: &DiscordHttpClient,
    interaction_id: &str,
    interaction_token: &str,
    container: ContainerBuilder,
) -> Result<(), DiscordError> {
    let data = ComponentsV2Payload::new(vec![container.build()]).into_value();
    let response = serde_json::json!({
        "type": INTERACTION_RESPONSE_UPDATE_MESSAGE,
        "data": data,
    });

    http.create_interaction_response_json(interaction_id, interaction_token, &response)
        .await
}

pub async fn respond_with_modal(
    http: &DiscordHttpClient,
    interaction_id: &str,
    interaction_token: &str,
    modal: ModalBuilder,
) -> Result<(), DiscordError> {
    let response = serde_json::json!({
        "type": INTERACTION_RESPONSE_MODAL,
        "data": modal.build(),
    });

    http.create_interaction_response_json(interaction_id, interaction_token, &response)
        .await
}

pub async fn respond_with_modal_typed(
    http: &DiscordHttpClient,
    context: &InteractionContextData,
    modal: ModalBuilder,
) -> Result<(), DiscordError> {
    respond_with_modal(http, context.id.as_str(), &context.token, modal).await
}

pub async fn defer_interaction(
    http: &DiscordHttpClient,
    context: &InteractionContextData,
    ephemeral: bool,
) -> Result<(), DiscordError> {
    let mut flags: u64 = 0;
    if ephemeral {
        flags |= 1 << 6;
    }

    respond_to_interaction(
        http,
        context,
        InteractionCallbackResponse {
            kind: INTERACTION_RESPONSE_DEFERRED_CHANNEL_MESSAGE,
            data: Some(serde_json::json!({ "flags": flags })),
        },
    )
    .await
}

pub async fn defer_update_interaction(
    http: &DiscordHttpClient,
    context: &InteractionContextData,
) -> Result<(), DiscordError> {
    respond_to_interaction(
        http,
        context,
        InteractionCallbackResponse {
            kind: INTERACTION_RESPONSE_DEFERRED_UPDATE_MESSAGE,
            data: None,
        },
    )
    .await
}

pub async fn update_interaction_message(
    http: &DiscordHttpClient,
    context: &InteractionContextData,
    message: CreateMessage,
) -> Result<(), DiscordError> {
    respond_to_interaction(
        http,
        context,
        InteractionCallbackResponse {
            kind: INTERACTION_RESPONSE_UPDATE_MESSAGE,
            data: Some(serde_json::to_value(message)?),
        },
    )
    .await
}

pub async fn respond_with_autocomplete_choices(
    http: &DiscordHttpClient,
    context: &InteractionContextData,
    choices: Vec<ApplicationCommandOptionChoice>,
) -> Result<(), DiscordError> {
    respond_to_interaction(
        http,
        context,
        InteractionCallbackResponse {
            kind: INTERACTION_RESPONSE_AUTOCOMPLETE_RESULT,
            data: Some(serde_json::json!({ "choices": choices })),
        },
    )
    .await
}

pub async fn launch_activity(
    http: &DiscordHttpClient,
    context: &InteractionContextData,
) -> Result<(), DiscordError> {
    respond_to_interaction(
        http,
        context,
        InteractionCallbackResponse {
            kind: INTERACTION_RESPONSE_LAUNCH_ACTIVITY,
            data: None,
        },
    )
    .await
}

pub async fn defer_and_followup_container(
    http: &DiscordHttpClient,
    interaction_id: &str,
    interaction_token: &str,
    container: ContainerBuilder,
    ephemeral: bool,
) -> Result<Value, DiscordError> {
    let mut flags: u64 = 0;
    if ephemeral {
        flags |= 1 << 6;
    }

    let defer_data = serde_json::json!({
        "type": INTERACTION_RESPONSE_DEFERRED_CHANNEL_MESSAGE,
        "data": { "flags": flags },
    });

    http.create_interaction_response_json(interaction_id, interaction_token, &defer_data)
        .await?;

    followup_with_container(http, interaction_token, container, ephemeral).await
}

pub async fn send_components_v2(
    http: &DiscordHttpClient,
    channel_id: u64,
    message: ComponentsV2Message,
) -> Result<Value, DiscordError> {
    let body = ComponentsV2Payload::new(message.build()).into_value();
    http.send_message_json(channel_id, &body).await
}

pub async fn respond_with_components_v2(
    http: &DiscordHttpClient,
    interaction_id: &str,
    interaction_token: &str,
    message: ComponentsV2Message,
    ephemeral: bool,
) -> Result<(), DiscordError> {
    let data = ComponentsV2Payload::new(message.build())
        .ephemeral(ephemeral)
        .into_value();
    let response = serde_json::json!({
        "type": INTERACTION_RESPONSE_CHANNEL_MESSAGE,
        "data": data,
    });

    http.create_interaction_response_json(interaction_id, interaction_token, &response)
        .await
}

pub async fn respond_component_with_components_v2(
    http: &DiscordHttpClient,
    interaction_id: &str,
    interaction_token: &str,
    message: ComponentsV2Message,
    ephemeral: bool,
) -> Result<(), DiscordError> {
    let data = ComponentsV2Payload::new(message.build())
        .ephemeral(ephemeral)
        .into_value();
    let response = serde_json::json!({
        "type": INTERACTION_RESPONSE_CHANNEL_MESSAGE,
        "data": data,
    });

    http.create_interaction_response_json(interaction_id, interaction_token, &response)
        .await
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{
        components_v2_flags, defer_and_followup_container, defer_interaction,
        defer_update_interaction, delete_followup_response, delete_original_response,
        edit_original_response, followup_message, followup_with_container, get_original_response,
        launch_activity, respond_component_with_components_v2, respond_component_with_container,
        respond_modal_with_container, respond_to_interaction, respond_with_autocomplete_choices,
        respond_with_components_v2, respond_with_container, respond_with_message,
        respond_with_modal, respond_with_modal_typed, update_component_with_container,
        update_interaction_message, ComponentsV2Payload, INTERACTION_RESPONSE_AUTOCOMPLETE_RESULT,
        INTERACTION_RESPONSE_DEFERRED_UPDATE_MESSAGE, INTERACTION_RESPONSE_LAUNCH_ACTIVITY,
        INTERACTION_RESPONSE_UPDATE_MESSAGE,
    };
    use crate::builders::{
        ComponentsV2Message, ContainerBuilder, ModalBuilder, TextDisplayBuilder, TextInputBuilder,
    };
    use crate::constants::MESSAGE_FLAG_IS_COMPONENTS_V2;
    use crate::error::DiscordError;
    use crate::http::DiscordHttpClient;
    use crate::model::{
        ApplicationCommandOptionChoice, CreateMessage, InteractionCallbackResponse,
        InteractionContextData, Snowflake,
    };

    fn sample_http(application_id: u64) -> DiscordHttpClient {
        DiscordHttpClient::new("test-token", application_id)
    }

    fn sample_context(application_id: &str, token: &str) -> InteractionContextData {
        InteractionContextData {
            id: Snowflake::from("111"),
            application_id: Snowflake::from(application_id),
            token: token.to_string(),
            ..InteractionContextData::default()
        }
    }

    fn sample_container() -> ContainerBuilder {
        ContainerBuilder::new().add_text_display(TextDisplayBuilder::new("hello helpers"))
    }

    fn sample_modal() -> ModalBuilder {
        ModalBuilder::new("helper-modal", "Helper Modal")
            .add_text_input(TextInputBuilder::short("name", "Name"))
    }

    fn sample_components_message() -> ComponentsV2Message {
        ComponentsV2Message::new().add_text_display(TextDisplayBuilder::new("hello helpers"))
    }

    fn sample_message() -> CreateMessage {
        CreateMessage {
            content: Some("hello".to_string()),
            flags: Some(8),
            ..CreateMessage::default()
        }
    }

    fn assert_model_error_contains(error: DiscordError, expected: &str) {
        match error {
            DiscordError::Model { message } => {
                assert!(
                    message.contains(expected),
                    "expected `{expected}` in `{message}`"
                );
            }
            other => panic!("expected model error containing `{expected}`, got {other:?}"),
        }
    }

    #[test]
    fn helper_constants_cover_new_callback_types() {
        assert_eq!(INTERACTION_RESPONSE_DEFERRED_UPDATE_MESSAGE, 6);
        assert_eq!(INTERACTION_RESPONSE_AUTOCOMPLETE_RESULT, 8);
        assert_eq!(INTERACTION_RESPONSE_LAUNCH_ACTIVITY, 12);
    }

    #[test]
    fn components_v2_flags_only_include_ephemeral_when_requested() {
        assert_eq!(components_v2_flags(false), MESSAGE_FLAG_IS_COMPONENTS_V2);
        assert_eq!(
            components_v2_flags(true),
            MESSAGE_FLAG_IS_COMPONENTS_V2 | (1 << 6)
        );
    }

    #[test]
    fn components_v2_payload_serializes_components_and_flags() {
        let component = json!({ "type": 17, "content": "hello" });

        let standard = ComponentsV2Payload::new(vec![component.clone()]).into_value();
        assert_eq!(
            standard,
            json!({
                "components": [component.clone()],
                "flags": MESSAGE_FLAG_IS_COMPONENTS_V2,
            })
        );

        let ephemeral = ComponentsV2Payload::new(vec![component.clone()])
            .ephemeral(true)
            .into_value();
        assert_eq!(
            ephemeral,
            json!({
                "components": [component],
                "flags": MESSAGE_FLAG_IS_COMPONENTS_V2 | (1 << 6),
            })
        );
    }

    #[tokio::test]
    async fn json_interaction_helpers_validate_bad_tokens_before_network() {
        let http = sample_http(123);

        assert_model_error_contains(
            respond_with_container(&http, "111", "bad/token", sample_container(), true)
                .await
                .unwrap_err(),
            "interaction_token",
        );
        assert_model_error_contains(
            respond_component_with_container(&http, "111", "bad/token", sample_container(), false)
                .await
                .unwrap_err(),
            "interaction_token",
        );
        assert_model_error_contains(
            respond_modal_with_container(&http, "111", "bad/token", sample_container(), true)
                .await
                .unwrap_err(),
            "interaction_token",
        );
        assert_model_error_contains(
            update_component_with_container(&http, "111", "bad/token", sample_container())
                .await
                .unwrap_err(),
            "interaction_token",
        );
        assert_model_error_contains(
            respond_with_modal(&http, "111", "bad/token", sample_modal())
                .await
                .unwrap_err(),
            "interaction_token",
        );
        assert_model_error_contains(
            respond_with_components_v2(
                &http,
                "111",
                "bad/token",
                sample_components_message(),
                true,
            )
            .await
            .unwrap_err(),
            "interaction_token",
        );
        assert_model_error_contains(
            respond_component_with_components_v2(
                &http,
                "111",
                "bad/token",
                sample_components_message(),
                false,
            )
            .await
            .unwrap_err(),
            "interaction_token",
        );
        assert_model_error_contains(
            defer_and_followup_container(&http, "111", "bad/token", sample_container(), true)
                .await
                .unwrap_err(),
            "interaction_token",
        );
    }

    #[tokio::test]
    async fn typed_interaction_helpers_validate_bad_context_tokens_before_network() {
        let http = sample_http(123);
        let context = sample_context("456", "bad/token");

        assert_model_error_contains(
            respond_to_interaction(
                &http,
                &context,
                InteractionCallbackResponse {
                    kind: INTERACTION_RESPONSE_UPDATE_MESSAGE,
                    data: Some(json!({ "content": "hello" })),
                },
            )
            .await
            .unwrap_err(),
            "interaction_token",
        );
        assert_model_error_contains(
            respond_with_message(&http, &context, sample_message(), true)
                .await
                .unwrap_err(),
            "interaction_token",
        );
        assert_model_error_contains(
            respond_with_modal_typed(&http, &context, sample_modal())
                .await
                .unwrap_err(),
            "interaction_token",
        );
        assert_model_error_contains(
            defer_interaction(&http, &context, true).await.unwrap_err(),
            "interaction_token",
        );
        assert_model_error_contains(
            defer_update_interaction(&http, &context).await.unwrap_err(),
            "interaction_token",
        );
        assert_model_error_contains(
            update_interaction_message(&http, &context, sample_message())
                .await
                .unwrap_err(),
            "interaction_token",
        );
        assert_model_error_contains(
            respond_with_autocomplete_choices(
                &http,
                &context,
                vec![ApplicationCommandOptionChoice::new("One", "1")],
            )
            .await
            .unwrap_err(),
            "interaction_token",
        );
        assert_model_error_contains(
            launch_activity(&http, &context).await.unwrap_err(),
            "interaction_token",
        );
    }

    #[tokio::test]
    async fn followup_helpers_validate_application_id_and_path_segments_before_network() {
        let http_without_application_id = sample_http(0);
        let http = sample_http(123);
        let zero_application_context = sample_context("0", "followup-token");
        let bad_token_context = sample_context("456", "bad/token");
        let good_context = sample_context("456", "followup-token");

        assert_model_error_contains(
            followup_with_container(
                &http_without_application_id,
                "followup-token",
                sample_container(),
                true,
            )
            .await
            .unwrap_err(),
            "application_id must be set",
        );
        assert_model_error_contains(
            followup_message(&http, &zero_application_context, sample_message(), false)
                .await
                .unwrap_err(),
            "application_id must be set",
        );
        assert_model_error_contains(
            followup_message(&http, &bad_token_context, sample_message(), true)
                .await
                .unwrap_err(),
            "interaction_token",
        );
        assert_model_error_contains(
            get_original_response(&http, &zero_application_context)
                .await
                .unwrap_err(),
            "application_id must be set",
        );
        assert_model_error_contains(
            edit_original_response(&http, &bad_token_context, sample_message())
                .await
                .unwrap_err(),
            "interaction_token",
        );
        assert_model_error_contains(
            delete_original_response(&http, &bad_token_context)
                .await
                .unwrap_err(),
            "interaction_token",
        );
        assert_model_error_contains(
            super::edit_followup_response(&http, &good_context, "bad/id", sample_message())
                .await
                .unwrap_err(),
            "message_id",
        );
        assert_model_error_contains(
            delete_followup_response(&http, &good_context, "bad/id")
                .await
                .unwrap_err(),
            "message_id",
        );
    }
}
