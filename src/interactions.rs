use async_trait::async_trait;
use axum::{
    body::Bytes,
    extract::State,
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    routing::post,
    Json, Router,
};
use ed25519_dalek::{Signature, VerifyingKey};
use serde_json::Value;

use crate::builders::ModalBuilder;
use crate::error::DiscordError;
use crate::model::{ApplicationCommandOptionChoice, Interaction, InteractionContextData};
use crate::parsers::{
    parse_interaction, parse_interaction_context, parse_raw_interaction, value_to_u8,
    InteractionContext, RawInteraction,
};
use crate::types::invalid_data_error;

pub enum InteractionResponse {
    Pong,
    ChannelMessage(Value),
    DeferredMessage,
    DeferredUpdate,
    AutocompleteResult(Vec<ApplicationCommandOptionChoice>),
    Modal(ModalBuilder),
    UpdateMessage(Value),
    LaunchActivity,
    Raw(Value),
}

#[async_trait]
pub trait InteractionHandler: Send + Sync {
    async fn handle(
        &self,
        ctx: InteractionContext,
        interaction: RawInteraction,
    ) -> InteractionResponse;
}

#[async_trait]
pub trait TypedInteractionHandler: Send + Sync {
    async fn handle_typed(
        &self,
        ctx: InteractionContextData,
        interaction: Interaction,
    ) -> InteractionResponse;
}

#[derive(Clone)]
struct InteractionsState<H> {
    verifying_key: VerifyingKey,
    handler: H,
}

fn decode_verifying_key(public_key: &str) -> Result<VerifyingKey, DiscordError> {
    let public_key_bytes = hex::decode(public_key)?;
    let public_key_bytes: [u8; 32] = public_key_bytes
        .as_slice()
        .try_into()
        .map_err(|_| invalid_data_error("public key must be 32 bytes"))?;

    VerifyingKey::from_bytes(&public_key_bytes)
        .map_err(|error| invalid_data_error(format!("invalid public key: {error}")))
}

fn verify_signature_with_key(
    verifying_key: &VerifyingKey,
    signature_hex: &str,
    timestamp: &str,
    body: &[u8],
) -> Result<(), DiscordError> {
    let signature_bytes = hex::decode(signature_hex)?;
    let signature = Signature::from_slice(&signature_bytes)
        .map_err(|error| invalid_data_error(format!("invalid signature bytes: {error}")))?;

    let mut signed_payload = Vec::with_capacity(timestamp.len() + body.len());
    signed_payload.extend_from_slice(timestamp.as_bytes());
    signed_payload.extend_from_slice(body);

    verifying_key
        .verify_strict(&signed_payload, &signature)
        .map_err(|_| invalid_data_error("signature verification failed"))
}

fn header_value<'a>(headers: &'a HeaderMap, name: &str) -> Result<&'a str, StatusCode> {
    headers
        .get(name)
        .ok_or(StatusCode::UNAUTHORIZED)
        .and_then(|value| value.to_str().map_err(|_| StatusCode::UNAUTHORIZED))
}

fn verify_discord_request_signature(
    verifying_key: &VerifyingKey,
    headers: &HeaderMap,
    body: &[u8],
) -> Result<(), StatusCode> {
    let signature = header_value(headers, "x-signature-ed25519")?;
    let timestamp = header_value(headers, "x-signature-timestamp")?;
    verify_signature_with_key(verifying_key, signature, timestamp, body)
        .map_err(|_| StatusCode::UNAUTHORIZED)
}

pub fn verify_discord_signature(
    public_key: &str,
    signature: &str,
    timestamp: &str,
    body: &[u8],
) -> Result<(), DiscordError> {
    let verifying_key = decode_verifying_key(public_key)?;
    verify_signature_with_key(&verifying_key, signature, timestamp, body)
}

fn interaction_response_payload(response: InteractionResponse) -> Value {
    match response {
        InteractionResponse::Pong => serde_json::json!({ "type": 1 }),
        InteractionResponse::ChannelMessage(data) => serde_json::json!({
            "type": 4,
            "data": data,
        }),
        InteractionResponse::DeferredMessage => serde_json::json!({ "type": 5 }),
        InteractionResponse::DeferredUpdate => serde_json::json!({ "type": 6 }),
        InteractionResponse::AutocompleteResult(choices) => serde_json::json!({
            "type": 8,
            "data": { "choices": choices },
        }),
        InteractionResponse::Modal(modal) => serde_json::json!({
            "type": 9,
            "data": modal.build(),
        }),
        InteractionResponse::UpdateMessage(data) => serde_json::json!({
            "type": 7,
            "data": data,
        }),
        InteractionResponse::LaunchActivity => serde_json::json!({ "type": 12 }),
        InteractionResponse::Raw(data) => data,
    }
}

async fn handle_interactions_request<H>(
    State(state): State<InteractionsState<H>>,
    headers: HeaderMap,
    body: Bytes,
) -> impl IntoResponse
where
    H: InteractionHandler + Clone + Send + Sync + 'static,
{
    if let Err(status) =
        verify_discord_request_signature(&state.verifying_key, &headers, body.as_ref())
    {
        return status.into_response();
    }

    let payload: Value = match serde_json::from_slice(body.as_ref()) {
        Ok(payload) => payload,
        Err(error) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({ "error": error.to_string() })),
            )
                .into_response();
        }
    };

    let interaction_type = match payload.get("type").and_then(value_to_u8) {
        Some(interaction_type) => interaction_type,
        None => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({ "error": "missing or invalid interaction.type" })),
            )
                .into_response();
        }
    };

    if interaction_type == 1 {
        return (StatusCode::OK, Json(serde_json::json!({ "type": 1 }))).into_response();
    }

    let context = match parse_interaction_context(&payload) {
        Ok(context) => context,
        Err(error) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({ "error": error.to_string() })),
            )
                .into_response();
        }
    };

    let interaction = match parse_raw_interaction(&payload) {
        Ok(interaction) => interaction,
        Err(error) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({ "error": error.to_string() })),
            )
                .into_response();
        }
    };

    let response = state.handler.handle(context, interaction).await;
    let response_payload = interaction_response_payload(response);
    (StatusCode::OK, Json(response_payload)).into_response()
}

pub fn try_interactions_endpoint<H>(public_key: &str, handler: H) -> Result<Router, DiscordError>
where
    H: InteractionHandler + Clone + Send + Sync + 'static,
{
    let verifying_key = decode_verifying_key(public_key)?;

    Ok(Router::new()
        .route("/interactions", post(handle_interactions_request::<H>))
        .with_state(InteractionsState {
            verifying_key,
            handler,
        }))
}

pub fn interactions_endpoint<H>(public_key: &str, handler: H) -> Router
where
    H: InteractionHandler + Clone + Send + Sync + 'static,
{
    try_interactions_endpoint(public_key, handler)
        .expect("invalid Discord public key for interactions endpoint")
}

async fn handle_typed_interactions_request<H>(
    State(state): State<InteractionsState<H>>,
    headers: HeaderMap,
    body: Bytes,
) -> impl IntoResponse
where
    H: TypedInteractionHandler + Clone + Send + Sync + 'static,
{
    if let Err(status) =
        verify_discord_request_signature(&state.verifying_key, &headers, body.as_ref())
    {
        return status.into_response();
    }

    let payload: Value = match serde_json::from_slice(body.as_ref()) {
        Ok(payload) => payload,
        Err(error) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({ "error": error.to_string() })),
            )
                .into_response();
        }
    };

    let interaction_type = match payload.get("type").and_then(value_to_u8) {
        Some(interaction_type) => interaction_type,
        None => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({ "error": "missing or invalid interaction.type" })),
            )
                .into_response();
        }
    };

    if interaction_type == 1 {
        return (StatusCode::OK, Json(serde_json::json!({ "type": 1 }))).into_response();
    }

    let interaction = match parse_interaction(&payload) {
        Ok(interaction) => interaction,
        Err(error) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({ "error": error.to_string() })),
            )
                .into_response();
        }
    };
    let context: InteractionContextData = interaction.context().clone();

    let response = state.handler.handle_typed(context, interaction).await;
    let response_payload = interaction_response_payload(response);
    (StatusCode::OK, Json(response_payload)).into_response()
}

pub fn try_typed_interactions_endpoint<H>(
    public_key: &str,
    handler: H,
) -> Result<Router, DiscordError>
where
    H: TypedInteractionHandler + Clone + Send + Sync + 'static,
{
    let verifying_key = decode_verifying_key(public_key)?;

    Ok(Router::new()
        .route(
            "/interactions",
            post(handle_typed_interactions_request::<H>),
        )
        .with_state(InteractionsState {
            verifying_key,
            handler,
        }))
}

pub fn typed_interactions_endpoint<H>(public_key: &str, handler: H) -> Router
where
    H: TypedInteractionHandler + Clone + Send + Sync + 'static,
{
    try_typed_interactions_endpoint(public_key, handler)
        .expect("invalid Discord public key for typed interactions endpoint")
}

#[cfg(test)]
mod tests {
    use async_trait::async_trait;
    use serde_json::Value;

    use super::{
        interaction_response_payload, try_interactions_endpoint, try_typed_interactions_endpoint,
        InteractionHandler, InteractionResponse, TypedInteractionHandler,
    };
    use crate::model::ApplicationCommandOptionChoice;
    use crate::model::{Interaction, InteractionContextData};
    use crate::parsers::{InteractionContext, RawInteraction};

    #[derive(Clone)]
    struct TestHandler;

    #[async_trait]
    impl InteractionHandler for TestHandler {
        async fn handle(
            &self,
            _ctx: InteractionContext,
            _interaction: RawInteraction,
        ) -> InteractionResponse {
            InteractionResponse::Raw(Value::Null)
        }
    }

    #[test]
    fn try_interactions_endpoint_rejects_invalid_public_key() {
        assert!(try_interactions_endpoint("bad-key", TestHandler).is_err());
    }

    #[test]
    fn try_interactions_endpoint_accepts_valid_public_key() {
        let public_key = "0000000000000000000000000000000000000000000000000000000000000000";
        assert!(try_interactions_endpoint(public_key, TestHandler).is_ok());
    }

    #[derive(Clone)]
    struct TypedTestHandler;

    #[async_trait]
    impl TypedInteractionHandler for TypedTestHandler {
        async fn handle_typed(
            &self,
            _ctx: InteractionContextData,
            _interaction: Interaction,
        ) -> InteractionResponse {
            InteractionResponse::Raw(Value::Null)
        }
    }

    #[test]
    fn try_typed_interactions_endpoint_accepts_valid_public_key() {
        let public_key = "0000000000000000000000000000000000000000000000000000000000000000";
        assert!(try_typed_interactions_endpoint(public_key, TypedTestHandler).is_ok());
    }

    #[test]
    fn interaction_response_payload_supports_deferred_update() {
        let payload = interaction_response_payload(InteractionResponse::DeferredUpdate);
        assert_eq!(payload["type"], 6);
    }

    #[test]
    fn interaction_response_payload_supports_autocomplete_choices() {
        let payload = interaction_response_payload(InteractionResponse::AutocompleteResult(vec![
            ApplicationCommandOptionChoice {
                name: "Support".to_string(),
                value: Value::String("support".to_string()),
            },
        ]));

        assert_eq!(payload["type"], 8);
        assert_eq!(payload["data"]["choices"][0]["name"], "Support");
        assert_eq!(payload["data"]["choices"][0]["value"], "support");
    }

    #[test]
    fn interaction_response_payload_supports_launch_activity() {
        let payload = interaction_response_payload(InteractionResponse::LaunchActivity);
        assert_eq!(payload["type"], 12);
    }
}
