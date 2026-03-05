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
use crate::parsers::{
    parse_interaction_context, parse_raw_interaction, value_to_u8, InteractionContext, RawInteraction,
};
use crate::types::{invalid_data_error, Error};

pub enum InteractionResponse {
    Pong,
    ChannelMessage(Value),
    DeferredMessage,
    Modal(ModalBuilder),
    UpdateMessage(Value),
    Raw(Value),
}

#[async_trait]
pub trait InteractionHandler: Send + Sync {
    async fn handle(&self, ctx: InteractionContext, interaction: RawInteraction) -> InteractionResponse;
}

#[derive(Clone)]
struct InteractionsState<H> {
    public_key: String,
    handler: H,
}

fn decode_verifying_key(public_key: &str) -> Result<VerifyingKey, Error> {
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
) -> Result<(), Error> {
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
    public_key: &str,
    headers: &HeaderMap,
    body: &[u8],
) -> Result<(), StatusCode> {
    let signature = header_value(headers, "x-signature-ed25519")?;
    let timestamp = header_value(headers, "x-signature-timestamp")?;
    let verifying_key = decode_verifying_key(public_key).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    verify_signature_with_key(&verifying_key, signature, timestamp, body)
        .map_err(|_| StatusCode::UNAUTHORIZED)
}

pub fn verify_discord_signature(
    public_key: &str,
    signature: &str,
    timestamp: &str,
    body: &[u8],
) -> Result<(), Error> {
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
        InteractionResponse::Modal(modal) => serde_json::json!({
            "type": 9,
            "data": modal.build(),
        }),
        InteractionResponse::UpdateMessage(data) => serde_json::json!({
            "type": 7,
            "data": data,
        }),
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
    if let Err(status) = verify_discord_request_signature(&state.public_key, &headers, body.as_ref()) {
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

pub fn interactions_endpoint<H>(public_key: &str, handler: H) -> Router
where
    H: InteractionHandler + Clone + Send + Sync + 'static,
{
    Router::new()
        .route("/interactions", post(handle_interactions_request::<H>))
        .with_state(InteractionsState {
            public_key: public_key.to_string(),
            handler,
        })
}
