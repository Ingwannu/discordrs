use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use super::body::{header_string, parse_body_value};
use super::paths::{
    archived_threads_query, configured_application_id, execute_webhook_path, followup_webhook_path,
    global_commands_path, interaction_callback_path, invite_query, joined_archived_threads_query,
    poll_answer_voters_query, rate_limit_route_key, request_uses_bot_authorization,
    subscription_query, thread_member_query, validate_token_path_segment,
};
use super::rate_limit::RateLimitState;
use super::{
    discord_api_error, discord_rate_limit_error, sleep_for_retry_after, FileAttachment, RestClient,
};
use crate::command::{command_type, CommandDefinition};
use crate::error::DiscordError;
use crate::model::{
    ApplicationRoleConnectionMetadata, ArchivedThreadsQuery, BulkGuildBanRequest, CreateMessage,
    CreateTestEntitlement, EntitlementQuery, InteractionCallbackResponse,
    JoinedArchivedThreadsQuery, Snowflake, SubscriptionQuery, ThreadMemberQuery,
};
use reqwest::header::{HeaderMap, HeaderValue};
use reqwest::{Method, StatusCode};
use serde_json::json;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::task::JoinHandle;

#[derive(Debug)]
struct PlannedResponse {
    status: StatusCode,
    headers: Vec<(String, String)>,
    body: String,
}

impl PlannedResponse {
    fn json(status: StatusCode, body: serde_json::Value) -> Self {
        Self {
            status,
            headers: vec![("Content-Type".to_string(), "application/json".to_string())],
            body: body.to_string(),
        }
    }

    fn text(status: StatusCode, body: impl Into<String>) -> Self {
        Self {
            status,
            headers: vec![("Content-Type".to_string(), "text/plain".to_string())],
            body: body.into(),
        }
    }

    fn empty(status: StatusCode) -> Self {
        Self {
            status,
            headers: Vec::new(),
            body: String::new(),
        }
    }
}

#[derive(Debug, Clone)]
struct RecordedRequest {
    method: String,
    path: String,
    headers: HashMap<String, String>,
    body: String,
}

impl RecordedRequest {
    fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .get(&name.to_ascii_lowercase())
            .map(String::as_str)
    }
}

fn find_bytes(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}

async fn read_recorded_request(stream: &mut tokio::net::TcpStream) -> RecordedRequest {
    let mut buffer = Vec::new();
    let mut header_end = None;
    let mut content_length = 0usize;

    loop {
        let mut chunk = [0u8; 2048];
        let read = stream.read(&mut chunk).await.expect("read request");
        assert!(read > 0, "client disconnected before sending request");
        buffer.extend_from_slice(&chunk[..read]);

        if header_end.is_none() {
            if let Some(index) = find_bytes(&buffer, b"\r\n\r\n") {
                header_end = Some(index + 4);
                let header_text = String::from_utf8_lossy(&buffer[..index]).to_string();
                content_length = header_text
                    .split("\r\n")
                    .find_map(|line| {
                        let (name, value) = line.split_once(':')?;
                        if name.eq_ignore_ascii_case("content-length") {
                            value.trim().parse::<usize>().ok()
                        } else {
                            None
                        }
                    })
                    .unwrap_or(0);
            }
        }

        if let Some(end) = header_end {
            if buffer.len() >= end + content_length {
                let header_text = String::from_utf8_lossy(&buffer[..end - 4]).to_string();
                let mut lines = header_text.split("\r\n");
                let request_line = lines.next().expect("request line");
                let mut parts = request_line.split_whitespace();
                let method = parts.next().expect("method").to_string();
                let path = parts.next().expect("path").to_string();
                let headers = lines
                    .filter_map(|line| {
                        let (name, value) = line.split_once(':')?;
                        Some((name.trim().to_ascii_lowercase(), value.trim().to_string()))
                    })
                    .collect::<HashMap<_, _>>();
                let body = String::from_utf8_lossy(&buffer[end..end + content_length]).to_string();

                return RecordedRequest {
                    method,
                    path,
                    headers,
                    body,
                };
            }
        }
    }
}

async fn write_planned_response(stream: &mut tokio::net::TcpStream, response: PlannedResponse) {
    let status_text = response.status.canonical_reason().unwrap_or("OK");
    let mut raw = format!(
        "HTTP/1.1 {} {}\r\nContent-Length: {}\r\nConnection: close\r\n",
        response.status.as_u16(),
        status_text,
        response.body.len()
    );
    for (name, value) in response.headers {
        raw.push_str(&format!("{name}: {value}\r\n"));
    }
    raw.push_str("\r\n");

    stream
        .write_all(raw.as_bytes())
        .await
        .expect("write headers");
    if !response.body.is_empty() {
        stream
            .write_all(response.body.as_bytes())
            .await
            .expect("write body");
    }
}

async fn spawn_test_server(
    responses: Vec<PlannedResponse>,
) -> (String, Arc<Mutex<Vec<RecordedRequest>>>, JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind test server");
    let base_url = format!("http://{}", listener.local_addr().expect("local addr"));
    let captured = Arc::new(Mutex::new(Vec::new()));
    let captured_for_task = Arc::clone(&captured);

    let task = tokio::spawn(async move {
        for response in responses {
            let (mut stream, _) = listener.accept().await.expect("accept request");
            let request = read_recorded_request(&mut stream).await;
            captured_for_task
                .lock()
                .expect("capture mutex")
                .push(request);
            write_planned_response(&mut stream, response).await;
        }
    });

    (base_url, captured, task)
}

fn message_payload(id: &str, channel_id: &str, content: &str) -> serde_json::Value {
    json!({
        "id": id,
        "channel_id": channel_id,
        "content": content
    })
}

fn channel_payload(id: &str, kind: u8, name: Option<&str>) -> serde_json::Value {
    let mut channel = json!({
        "id": id,
        "type": kind
    });
    if let Some(name) = name {
        channel["name"] = json!(name);
    }
    channel
}

fn guild_payload(id: &str, name: &str) -> serde_json::Value {
    json!({
        "id": id,
        "name": name
    })
}

fn current_user_guild_payload(id: &str, name: &str) -> serde_json::Value {
    json!({
        "id": id,
        "name": name,
        "owner": false,
        "permissions": "8",
        "features": ["COMMUNITY"]
    })
}

fn member_payload(user_id: &str, username: &str) -> serde_json::Value {
    json!({
        "user": {
            "id": user_id,
            "username": username,
            "discriminator": "0000",
            "bot": false
        },
        "roles": []
    })
}

fn webhook_payload(id: &str, name: &str) -> serde_json::Value {
    json!({
        "id": id,
        "type": 1,
        "name": name,
        "channel_id": "100",
        "token": "token"
    })
}

fn sticker_payload(id: &str) -> serde_json::Value {
    json!({
        "id": id,
        "name": "sticker",
        "tags": "tag",
        "type": 2,
        "format_type": 1
    })
}

fn stage_payload(channel_id: &str) -> serde_json::Value {
    json!({
        "id": "9000",
        "guild_id": "200",
        "channel_id": channel_id,
        "topic": "town hall",
        "privacy_level": 2
    })
}

fn welcome_screen_payload() -> serde_json::Value {
    json!({
        "description": "welcome",
        "welcome_channels": [{
            "channel_id": "300",
            "description": "rules",
            "emoji_name": "wave"
        }]
    })
}

fn onboarding_payload() -> serde_json::Value {
    json!({
        "guild_id": "200",
        "prompts": [],
        "default_channel_ids": ["300"],
        "enabled": true,
        "mode": 1
    })
}

fn template_payload(code: &str) -> serde_json::Value {
    json!({
        "code": code,
        "name": "template",
        "usage_count": 0,
        "created_at": "2026-01-01T00:00:00.000000+00:00",
        "updated_at": "2026-01-01T00:00:00.000000+00:00"
    })
}

fn scheduled_event_payload(id: &str) -> serde_json::Value {
    json!({
        "id": id,
        "guild_id": "200",
        "channel_id": "300",
        "creator_id": "400",
        "name": "community night",
        "description": "games",
        "scheduled_start_time": "2026-05-01T00:00:00.000000+00:00",
        "scheduled_end_time": "2026-05-01T01:00:00.000000+00:00",
        "privacy_level": 2,
        "status": 1,
        "entity_type": 2,
        "entity_metadata": { "location": "Stage" },
        "recurrence_rule": {
            "start": "2026-05-01T00:00:00.000000+00:00",
            "frequency": 2,
            "interval": 1
        },
        "user_count": 5
    })
}

fn sku_payload(id: &str) -> serde_json::Value {
    json!({
        "id": id,
        "type": 5,
        "application_id": "555",
        "name": "Premium",
        "slug": "premium",
        "flags": 128
    })
}

fn entitlement_payload(id: &str) -> serde_json::Value {
    json!({
        "id": id,
        "sku_id": "900",
        "application_id": "555",
        "user_id": "777",
        "type": 8,
        "deleted": false,
        "consumed": false,
        "starts_at": "2026-01-01T00:00:00.000000+00:00",
        "ends_at": "2026-02-01T00:00:00.000000+00:00",
        "guild_id": "200"
    })
}

fn subscription_payload(id: &str) -> serde_json::Value {
    json!({
        "id": id,
        "user_id": "777",
        "sku_ids": ["900"],
        "entitlement_ids": ["901"],
        "current_period_start": "2026-04-01T00:00:00.000000+00:00",
        "current_period_end": "2026-05-01T00:00:00.000000+00:00",
        "status": 0,
        "canceled_at": null
    })
}

fn soundboard_payload(id: &str) -> serde_json::Value {
    json!({
        "name": "quack",
        "sound_id": id,
        "volume": 1.0,
        "emoji_name": "duck",
        "guild_id": "200",
        "available": true
    })
}

fn role_payload(id: &str, name: &str) -> serde_json::Value {
    json!({
        "id": id,
        "name": name
    })
}

fn emoji_payload(id: &str, name: &str) -> serde_json::Value {
    json!({
        "id": id,
        "name": name,
        "animated": false
    })
}

fn auto_moderation_rule_payload(id: &str) -> serde_json::Value {
    json!({
        "id": id,
        "guild_id": "200",
        "name": "block bad words",
        "creator_id": "42",
        "event_type": 1,
        "trigger_type": 1,
        "trigger_metadata": {
            "keyword_filter": ["bad"],
            "allow_list": []
        },
        "actions": [{ "type": 1 }],
        "enabled": true,
        "exempt_roles": [],
        "exempt_channels": []
    })
}

fn guild_preview_payload() -> serde_json::Value {
    json!({
        "id": "200",
        "name": "preview guild",
        "emojis": [],
        "features": ["COMMUNITY"],
        "approximate_member_count": 100,
        "approximate_presence_count": 10,
        "description": "typed preview",
        "stickers": []
    })
}

fn command_payload(id: &str, name: &str, description: &str) -> serde_json::Value {
    json!({
        "id": id,
        "type": 1,
        "name": name,
        "description": description
    })
}

fn application_payload(id: &str, name: &str) -> serde_json::Value {
    json!({
        "id": id,
        "name": name,
        "description": "application",
        "bot_public": true,
        "bot_require_code_grant": false
    })
}

fn role_connection_metadata_payload(key: &str) -> serde_json::Value {
    json!({
        "type": 1,
        "key": key,
        "name": "Score",
        "description": "User score"
    })
}

fn gateway_payload() -> serde_json::Value {
    json!({
        "url": "wss://gateway.discord.gg",
        "shards": 1,
        "session_start_limit": {
            "total": 10,
            "remaining": 9,
            "reset_after": 1000,
            "max_concurrency": 1
        }
    })
}

fn assert_request_basics(
    request: &RecordedRequest,
    method: &str,
    path: &str,
    expected_authorization: Option<&str>,
) {
    assert_eq!(request.method, method);
    assert_eq!(request.path, path);
    assert_eq!(request.header("authorization"), expected_authorization);
    assert_eq!(
        request.header("user-agent"),
        Some(concat!(
            "DiscordBot (discordrs, ",
            env!("CARGO_PKG_VERSION"),
            ")"
        ))
    );
    assert_eq!(request.header("content-type"), Some("application/json"));
}

fn assert_multipart_request(
    request: &RecordedRequest,
    method: &str,
    path: &str,
    expected_authorization: Option<&str>,
) {
    assert_eq!(request.method, method);
    assert_eq!(request.path, path);
    assert_eq!(request.header("authorization"), expected_authorization);
    assert_eq!(
        request.header("user-agent"),
        Some(concat!(
            "DiscordBot (discordrs, ",
            env!("CARGO_PKG_VERSION"),
            ")"
        ))
    );
    assert!(
        request
            .header("content-type")
            .is_some_and(|value| value.starts_with("multipart/form-data; boundary=")),
        "expected multipart content-type, got {:?}",
        request.header("content-type")
    );
    assert!(request.body.contains(r#"name="payload_json""#));
    assert!(request.body.contains(r#"name="files[0]""#));
}

fn sample_command() -> CommandDefinition {
    CommandDefinition {
        kind: command_type::CHAT_INPUT,
        name: "ping".to_string(),
        description: "pong".to_string(),
        ..CommandDefinition::default()
    }
}

fn sample_message() -> CreateMessage {
    CreateMessage {
        content: Some("hello".to_string()),
        ..CreateMessage::default()
    }
}

fn sample_interaction_response() -> InteractionCallbackResponse {
    InteractionCallbackResponse {
        kind: 4,
        data: Some(json!({ "content": "ack" })),
    }
}

fn sample_file(name: &str, data: &str) -> FileAttachment {
    FileAttachment::new(name, data.as_bytes().to_vec()).with_content_type("text/plain")
}

fn assert_model_error_contains(error: DiscordError, expected: &str) {
    match error {
        DiscordError::Model { message } => {
            assert!(
                message.contains(expected),
                "expected `{expected}` in `{message}`"
            );
        }
        other => panic!("unexpected error variant: {other:?}"),
    }
}

#[test]
fn configured_application_id_rejects_zero() {
    let error = configured_application_id(0).unwrap_err();
    assert!(error.to_string().contains("application_id must be set"));
}

#[test]
fn global_commands_path_rejects_zero_application_id() {
    let error = global_commands_path(0).unwrap_err();
    assert!(error.to_string().contains("application_id must be set"));
}

#[test]
fn global_commands_path_uses_configured_application_id() {
    assert_eq!(
        global_commands_path(123).unwrap(),
        "/applications/123/commands"
    );
}

#[test]
fn followup_webhook_path_uses_explicit_application_id() {
    let path = followup_webhook_path("123", "token", None).unwrap();
    assert_eq!(path, "/webhooks/123/token");
}

#[test]
fn edit_followup_webhook_path_includes_message_id() {
    let path = followup_webhook_path("123", "token", Some("456")).unwrap();
    assert_eq!(path, "/webhooks/123/token/messages/456");
}

#[test]
fn original_interaction_response_path_uses_original_message_marker() {
    let path = followup_webhook_path("123", "token", Some("@original")).unwrap();
    assert_eq!(path, "/webhooks/123/token/messages/@original");
}

#[test]
fn followup_webhook_path_rejects_zero_application_id() {
    let error = followup_webhook_path("0", "token", None).unwrap_err();
    assert!(error.to_string().contains("application_id must be set"));
}

#[test]
fn followup_webhook_path_rejects_empty_or_unsafe_segments() {
    let token_error = followup_webhook_path("123", "", None).unwrap_err();
    assert!(token_error.to_string().contains("interaction_token"));

    let token_separator_error = followup_webhook_path("123", "token/part", None).unwrap_err();
    assert!(token_separator_error
        .to_string()
        .contains("interaction_token"));

    let application_id_error = followup_webhook_path("12/3", "token", None).unwrap_err();
    assert!(application_id_error.to_string().contains("application_id"));

    let message_error = followup_webhook_path("123", "token", Some("bad/id")).unwrap_err();
    assert!(message_error.to_string().contains("message_id"));
}

#[test]
fn interaction_callback_path_rejects_unsafe_tokens() {
    let error = interaction_callback_path(Snowflake::from("123"), "bad/token").unwrap_err();
    assert!(error.to_string().contains("interaction_token"));
}

#[test]
fn interaction_and_webhook_paths_accept_safe_segments() {
    assert_eq!(
        interaction_callback_path(Snowflake::from("123"), "safe-token").unwrap(),
        "/interactions/123/safe-token/callback"
    );
    assert_eq!(
        execute_webhook_path(Snowflake::from("456"), "safe-token").unwrap(),
        "/webhooks/456/safe-token?wait=true"
    );
}

#[test]
fn execute_webhook_path_rejects_unsafe_tokens() {
    let error = execute_webhook_path(Snowflake::from("123"), "bad/token").unwrap_err();
    assert!(error.to_string().contains("webhook_token"));
}

#[test]
fn request_uses_bot_authorization_skips_tokenized_callback_paths() {
    assert!(request_uses_bot_authorization("/channels/123/messages"));
    assert!(!request_uses_bot_authorization("/webhooks/123/token"));
    assert!(!request_uses_bot_authorization(
        "/interactions/123/token/callback"
    ));
    assert!(!request_uses_bot_authorization(
        "/webhooks/123/token/messages/@original"
    ));
}

#[test]
fn discord_api_error_preserves_status_and_code() {
    let error = discord_api_error(
        StatusCode::BAD_REQUEST,
        r#"{"code":50035,"message":"Invalid Form Body"}"#,
    );

    match error {
        DiscordError::Api {
            status,
            code,
            message,
        } => {
            assert_eq!(status, 400);
            assert_eq!(code, Some(50035));
            assert_eq!(message, "Invalid Form Body");
        }
        other => panic!("unexpected error variant: {other:?}"),
    }
}

#[test]
fn discord_rate_limit_error_preserves_route_and_retry_after() {
    let error = discord_rate_limit_error(
        "POST:webhooks/123/token",
        r#"{"message":"You are being rate limited.","retry_after":2.5,"global":false}"#,
    );

    match error {
        DiscordError::RateLimit { route, retry_after } => {
            assert_eq!(route, "POST:webhooks/123/token");
            assert_eq!(retry_after, 2.5);
        }
        other => panic!("unexpected error variant: {other:?}"),
    }
}

#[test]
fn discord_api_error_uses_string_and_object_fallback_messages() {
    match discord_api_error(StatusCode::BAD_REQUEST, r#""plain string""#) {
        DiscordError::Api { message, .. } => {
            assert_eq!(message, "plain string");
        }
        other => panic!("unexpected error variant: {other:?}"),
    }

    match discord_api_error(StatusCode::BAD_REQUEST, r#"{"code":7}"#) {
        DiscordError::Api { code, message, .. } => {
            assert_eq!(code, Some(7));
            assert_eq!(message, r#"{"code":7}"#);
        }
        other => panic!("unexpected error variant: {other:?}"),
    }
}

#[test]
fn rate_limit_route_key_preserves_major_parameters() {
    assert_eq!(
        rate_limit_route_key(&Method::GET, "/channels/123/messages/456"),
        "GET:channels/123/messages/:id"
    );
    assert_eq!(
        rate_limit_route_key(&Method::GET, "/guilds/789/members/456"),
        "GET:guilds/789/members/:id"
    );
    assert_eq!(
        rate_limit_route_key(&Method::GET, "/webhooks/111/222/messages/333"),
        "GET:webhooks/111/222/messages/:id"
    );
}

#[test]
fn rate_limit_route_key_keeps_application_and_guild_major_ids() {
    assert_eq!(
        rate_limit_route_key(&Method::POST, "/applications/123/guilds/456/commands/789"),
        "POST:applications/123/guilds/456/commands/:id"
    );
}

#[test]
fn rate_limit_state_reports_wait_duration() {
    let state = RateLimitState::default();
    state.blocked_until.lock().unwrap().insert(
        "GET:channels/:id".to_string(),
        Instant::now() + Duration::from_secs(1),
    );

    assert!(state.wait_duration("GET:channels/:id").is_some());
}

#[test]
fn rate_limit_state_keeps_major_parameters_distinct() {
    let state = RateLimitState::default();
    let blocked_route = rate_limit_route_key(&Method::GET, "/channels/123/messages/456");
    let other_route = rate_limit_route_key(&Method::GET, "/channels/999/messages/456");

    state.blocked_until.lock().unwrap().insert(
        blocked_route.clone(),
        Instant::now() + Duration::from_secs(1),
    );

    assert!(state.wait_duration(&blocked_route).is_some());
    assert!(state.wait_duration(&other_route).is_none());
}

#[test]
fn parse_helpers_handle_empty_invalid_and_string_body_shapes() {
    assert_eq!(parse_body_value(String::new()), serde_json::Value::Null);
    assert_eq!(
        parse_body_value(String::from("plain text")),
        serde_json::Value::String(String::from("plain text"))
    );
    assert_eq!(
        parse_body_value(String::from(r#"{"message":"ok"}"#)),
        serde_json::json!({ "message": "ok" })
    );

    let header = HeaderValue::from_static("bucket-1");
    assert_eq!(header_string(Some(&header)), Some(String::from("bucket-1")));
    assert_eq!(header_string(None), None);
}

#[test]
fn validate_token_path_segment_handles_original_marker_and_control_characters() {
    validate_token_path_segment("message_id", "@original", true).unwrap();
    validate_token_path_segment("token", "safe-token", false).unwrap();

    let backslash = validate_token_path_segment("token", r"bad\token", false).unwrap_err();
    assert!(backslash.to_string().contains("token"));

    let query = validate_token_path_segment("token", "bad?token", false).unwrap_err();
    assert!(query.to_string().contains("token"));
}

#[test]
fn authorization_and_error_helpers_cover_query_and_fallback_cases() {
    assert!(request_uses_bot_authorization(
        "/channels/123/messages?wait=true"
    ));
    assert!(!request_uses_bot_authorization(
        "/webhooks/123/token?wait=true"
    ));

    match discord_api_error(StatusCode::BAD_REQUEST, "plain body") {
        DiscordError::Api {
            status,
            code,
            message,
        } => {
            assert_eq!(status, 400);
            assert_eq!(code, None);
            assert_eq!(message, "plain body");
        }
        other => panic!("unexpected error variant: {other:?}"),
    }

    match discord_rate_limit_error("GET:channels/123", r#"{"message":"limited"}"#) {
        DiscordError::RateLimit { route, retry_after } => {
            assert_eq!(route, "GET:channels/123");
            assert_eq!(retry_after, 1.0);
        }
        other => panic!("unexpected error variant: {other:?}"),
    }

    assert_eq!(
        rate_limit_route_key(&Method::PATCH, "/channels/123/messages/456?wait=true"),
        "PATCH:channels/123/messages/:id"
    );
}

#[test]
fn new_coverage_query_helpers_build_expected_paths() {
    assert_eq!(
        thread_member_query(&ThreadMemberQuery {
            with_member: Some(true),
            after: Some(Snowflake::from("10")),
            limit: Some(25),
        }),
        "?with_member=true&after=10&limit=25"
    );
    assert_eq!(
        archived_threads_query(&ArchivedThreadsQuery {
            before: Some("2026-04-29T00:00:00Z".to_string()),
            limit: Some(50),
        }),
        "?before=2026-04-29T00:00:00Z&limit=50"
    );
    assert_eq!(
        joined_archived_threads_query(&JoinedArchivedThreadsQuery {
            before: Some(Snowflake::from("11")),
            limit: Some(10),
        }),
        "?before=11&limit=10"
    );
    assert_eq!(
        subscription_query(&SubscriptionQuery {
            before: Some(Snowflake::from("20")),
            after: Some(Snowflake::from("21")),
            limit: Some(100),
            user_id: Some(Snowflake::from("22")),
        }),
        "?before=20&after=21&limit=100&user_id=22"
    );
    assert_eq!(
        invite_query(Some(true), Some(false), Some(Snowflake::from("30"))),
        "?with_counts=true&with_expiration=false&guild_scheduled_event_id=30"
    );
    assert_eq!(
        poll_answer_voters_query(Some(Snowflake::from("40")), Some(15)),
        "?after=40&limit=15"
    );
}

#[test]
fn rate_limit_state_observe_tracks_buckets_and_global_limits() {
    let state = RateLimitState::default();
    let route_key = "POST:channels/123/messages";
    let mut headers = HeaderMap::new();
    headers.insert("x-ratelimit-bucket", HeaderValue::from_static("bucket-42"));
    headers.insert("x-ratelimit-remaining", HeaderValue::from_static("0"));
    headers.insert("x-ratelimit-reset-after", HeaderValue::from_static("0.05"));

    state.observe(route_key, &headers, StatusCode::OK, "");
    assert!(state.route_buckets.lock().unwrap().contains_key(route_key));
    assert!(state.wait_duration(route_key).is_some());

    let global_headers = HeaderMap::new();
    state.observe(
        route_key,
        &global_headers,
        StatusCode::TOO_MANY_REQUESTS,
        r#"{"retry_after":0.05,"global":true}"#,
    );
    assert!(state.wait_duration("GET:anything").is_some());
}

#[test]
fn rate_limit_state_shares_bucket_blocks_across_routes_after_429() {
    let state = RateLimitState::default();
    let route_a = rate_limit_route_key(&Method::GET, "/channels/123/messages");
    let route_b = rate_limit_route_key(&Method::POST, "/channels/456/messages");

    let mut bucket_headers = HeaderMap::new();
    bucket_headers.insert(
        "x-ratelimit-bucket",
        HeaderValue::from_static("shared-bucket"),
    );

    state.observe(&route_a, &bucket_headers, StatusCode::OK, "");
    state.observe(&route_b, &bucket_headers, StatusCode::OK, "");
    state.observe(
        &route_a,
        &HeaderMap::new(),
        StatusCode::TOO_MANY_REQUESTS,
        r#"{"retry_after":0.05,"global":false}"#,
    );

    assert!(state.wait_duration(&route_a).is_some());
    assert!(state.wait_duration(&route_b).is_some());
}

#[test]
fn rate_limit_state_ignores_expired_route_and_global_blocks() {
    let state = RateLimitState::default();
    state
        .blocked_until
        .lock()
        .unwrap()
        .insert("GET:channels/123/messages".to_string(), Instant::now());
    *state.global_blocked_until.lock().unwrap() = Some(Instant::now());

    assert!(state.wait_duration("GET:channels/123/messages").is_none());
    assert!(state.wait_duration("GET:anything").is_none());
    assert!(state.blocked_until.lock().unwrap().is_empty());
    assert!(state.global_blocked_until.lock().unwrap().is_none());
}

#[test]
fn rate_limit_state_cleans_stale_bucket_metadata() {
    let state = RateLimitState::default();
    state.route_buckets.lock().unwrap().insert(
        "GET:channels/1/messages".to_string(),
        "bucket-old".to_string(),
    );
    state.bucket_last_seen.lock().unwrap().insert(
        "bucket-old".to_string(),
        Instant::now() - super::RATE_LIMIT_BUCKET_RETENTION - Duration::from_secs(1),
    );

    assert!(state.wait_duration("GET:channels/1/messages").is_none());
    assert!(state.route_buckets.lock().unwrap().is_empty());
    assert!(state.bucket_last_seen.lock().unwrap().is_empty());
}

#[tokio::test]
async fn sleep_for_retry_after_waits_without_panicking() {
    let start = Instant::now();
    sleep_for_retry_after(0.01).await;
    assert!(start.elapsed() >= Duration::from_millis(5));
}

#[tokio::test]
async fn channel_message_file_helpers_send_multipart_payloads() {
    let responses = vec![
        PlannedResponse::json(StatusCode::OK, message_payload("701", "100", "created")),
        PlannedResponse::json(StatusCode::OK, message_payload("702", "100", "updated")),
    ];
    let (base_url, captured, server) = spawn_test_server(responses).await;
    let client = RestClient::new_with_base_url("file-token", 123, base_url);
    let body = sample_message();
    let files = vec![sample_file("hello.txt", "hello file")];

    assert_eq!(
        client
            .create_message_with_files(Snowflake::from("100"), &body, &files)
            .await
            .expect("create message with files")
            .content,
        "created"
    );
    assert_eq!(
        client
            .update_message_with_files(
                Snowflake::from("100"),
                Snowflake::from("701"),
                &body,
                &files,
            )
            .await
            .expect("update message with files")
            .content,
        "updated"
    );

    server.await.expect("server finished");
    let requests = captured.lock().expect("captured requests");
    assert_eq!(requests.len(), 2);
    assert_multipart_request(
        &requests[0],
        "POST",
        "/channels/100/messages",
        Some("Bot file-token"),
    );
    assert!(requests[0].body.contains(r#"{"content":"hello"}"#));
    assert!(requests[0].body.contains(r#"filename="hello.txt""#));
    assert!(requests[0].body.contains("Content-Type: text/plain"));
    assert!(requests[0].body.contains("hello file"));
    assert_multipart_request(
        &requests[1],
        "PATCH",
        "/channels/100/messages/701",
        Some("Bot file-token"),
    );
    assert!(requests[1].body.contains(r#"filename="hello.txt""#));
}

#[tokio::test]
async fn tokenized_file_helpers_send_multipart_without_bot_authorization() {
    let responses = vec![
        PlannedResponse::json(StatusCode::OK, json!({ "id": "800" })),
        PlannedResponse::empty(StatusCode::NO_CONTENT),
        PlannedResponse::json(StatusCode::OK, message_payload("801", "500", "followup")),
        PlannedResponse::json(StatusCode::OK, message_payload("802", "500", "original")),
        PlannedResponse::json(
            StatusCode::OK,
            message_payload("803", "500", "followup-edit"),
        ),
    ];
    let (base_url, captured, server) = spawn_test_server(responses).await;
    let client = RestClient::new_with_base_url("bot-token", 123, base_url);
    let body = sample_message();
    let files = vec![sample_file("tokenized.txt", "tokenized file")];

    assert_eq!(
        client
            .execute_webhook_with_files(
                Snowflake::from("777"),
                "token",
                &json!({ "content": "webhook" }),
                &files,
            )
            .await
            .expect("execute webhook with files")["id"],
        json!("800")
    );
    client
        .create_interaction_response_with_files(
            Snowflake::from("778"),
            "token",
            &sample_interaction_response(),
            &files,
        )
        .await
        .expect("create interaction response with files");
    assert_eq!(
        client
            .create_followup_message_with_files("token", &body, &files)
            .await
            .expect("create followup with files")
            .content,
        "followup"
    );
    assert_eq!(
        client
            .edit_original_interaction_response_with_files("token", &body, &files)
            .await
            .expect("edit original with files")
            .content,
        "original"
    );
    assert_eq!(
        client
            .edit_followup_message_with_files("token", "55", &body, &files)
            .await
            .expect("edit followup with files")
            .content,
        "followup-edit"
    );

    server.await.expect("server finished");
    let requests = captured.lock().expect("captured requests");
    assert_eq!(requests.len(), 5);
    assert_multipart_request(&requests[0], "POST", "/webhooks/777/token?wait=true", None);
    assert_multipart_request(
        &requests[1],
        "POST",
        "/interactions/778/token/callback",
        None,
    );
    assert_multipart_request(&requests[2], "POST", "/webhooks/123/token", None);
    assert_multipart_request(
        &requests[3],
        "PATCH",
        "/webhooks/123/token/messages/@original",
        None,
    );
    assert_multipart_request(
        &requests[4],
        "PATCH",
        "/webhooks/123/token/messages/55",
        None,
    );
    assert!(requests[0].body.contains(r#"{"content":"webhook"}"#));
    assert!(requests[1].body.contains(r#""type":4"#));
    assert!(requests[4].body.contains("tokenized file"));
}

#[tokio::test]
async fn webhook_message_crud_uses_tokenized_message_paths() {
    let responses = vec![
        PlannedResponse::json(StatusCode::OK, message_payload("900", "500", "webhook")),
        PlannedResponse::json(StatusCode::OK, message_payload("900", "500", "edited")),
        PlannedResponse::json(StatusCode::OK, message_payload("900", "500", "edited-file")),
        PlannedResponse::empty(StatusCode::NO_CONTENT),
    ];
    let (base_url, captured, server) = spawn_test_server(responses).await;
    let client = RestClient::new_with_base_url("bot-token", 123, base_url);
    let body = sample_message();
    let files = vec![sample_file("webhook.txt", "webhook file")];

    assert_eq!(
        client
            .get_webhook_message(Snowflake::from("777"), "token", "900")
            .await
            .expect("get webhook message")
            .content,
        "webhook"
    );
    assert_eq!(
        client
            .edit_webhook_message(Snowflake::from("777"), "token", "900", &body)
            .await
            .expect("edit webhook message")
            .content,
        "edited"
    );
    assert_eq!(
        client
            .edit_webhook_message_with_files(Snowflake::from("777"), "token", "900", &body, &files,)
            .await
            .expect("edit webhook message with files")
            .content,
        "edited-file"
    );
    client
        .delete_webhook_message(Snowflake::from("777"), "token", "900")
        .await
        .expect("delete webhook message");

    server.await.expect("server finished");
    let requests = captured.lock().expect("captured requests");
    assert_eq!(requests.len(), 4);
    assert_request_basics(
        &requests[0],
        "GET",
        "/webhooks/777/token/messages/900",
        None,
    );
    assert_request_basics(
        &requests[1],
        "PATCH",
        "/webhooks/777/token/messages/900",
        None,
    );
    assert_multipart_request(
        &requests[2],
        "PATCH",
        "/webhooks/777/token/messages/900",
        None,
    );
    assert_request_basics(
        &requests[3],
        "DELETE",
        "/webhooks/777/token/messages/900",
        None,
    );
    assert!(requests[2].body.contains("webhook file"));
}

#[tokio::test]
async fn typed_guild_admin_and_automod_wrappers_hit_expected_paths() {
    let responses = vec![
        PlannedResponse::json(
            StatusCode::OK,
            json!({ "banned_users": ["1"], "failed_users": ["2"] }),
        ),
        PlannedResponse::json(StatusCode::OK, role_payload("9", "admin")),
        PlannedResponse::json(StatusCode::OK, json!([auto_moderation_rule_payload("7")])),
        PlannedResponse::json(StatusCode::OK, auto_moderation_rule_payload("7")),
        PlannedResponse::json(StatusCode::OK, auto_moderation_rule_payload("8")),
        PlannedResponse::json(StatusCode::OK, auto_moderation_rule_payload("8")),
        PlannedResponse::json(StatusCode::OK, guild_preview_payload()),
        PlannedResponse::json(StatusCode::OK, json!({ "pruned": 3 })),
        PlannedResponse::json(StatusCode::OK, json!({ "pruned": 2 })),
        PlannedResponse::json(StatusCode::OK, json!({ "code": "discordrs", "uses": 10 })),
        PlannedResponse::json(
            StatusCode::OK,
            json!([{
                "id": "rotterdam",
                "name": "Rotterdam",
                "optimal": true,
                "deprecated": false,
                "custom": false
            }]),
        ),
    ];
    let (base_url, captured, server) = spawn_test_server(responses).await;
    let client = RestClient::new_with_base_url("typed-admin-token", 555, base_url);

    let ban = client
        .bulk_guild_ban(
            Snowflake::from("200"),
            &BulkGuildBanRequest {
                user_ids: vec![Snowflake::from("1"), Snowflake::from("2")],
                delete_message_seconds: Some(60),
            },
        )
        .await
        .unwrap();
    assert_eq!(ban.banned_users[0].as_str(), "1");
    assert_eq!(
        client
            .get_guild_role(Snowflake::from("200"), Snowflake::from("9"))
            .await
            .unwrap()
            .name,
        "admin"
    );
    assert_eq!(
        client
            .get_auto_moderation_rules_typed(Snowflake::from("200"))
            .await
            .unwrap()[0]
            .id
            .as_str(),
        "7"
    );
    assert_eq!(
        client
            .get_auto_moderation_rule(Snowflake::from("200"), Snowflake::from("7"))
            .await
            .unwrap()
            .name,
        "block bad words"
    );
    assert!(
        client
            .create_auto_moderation_rule_typed(Snowflake::from("200"), &json!({ "name": "new" }))
            .await
            .unwrap()
            .enabled
    );
    assert!(
        client
            .modify_auto_moderation_rule_typed(
                Snowflake::from("200"),
                Snowflake::from("8"),
                &json!({ "name": "updated" }),
            )
            .await
            .unwrap()
            .enabled
    );
    assert_eq!(
        client
            .get_guild_preview_typed(Snowflake::from("200"))
            .await
            .unwrap()
            .approximate_member_count,
        100
    );
    assert_eq!(
        client
            .get_guild_prune_count_typed(Snowflake::from("200"), Some(7), &[Snowflake::from("9")])
            .await
            .unwrap()
            .pruned,
        Some(3)
    );
    assert_eq!(
        client
            .begin_guild_prune_typed(Snowflake::from("200"), Some(7), Some(false), &[])
            .await
            .unwrap()
            .pruned,
        Some(2)
    );
    assert_eq!(
        client
            .get_guild_vanity_url(Snowflake::from("200"))
            .await
            .unwrap()
            .code
            .as_deref(),
        Some("discordrs")
    );
    assert_eq!(
        client.get_voice_regions_typed().await.unwrap()[0].id,
        "rotterdam"
    );

    server.await.expect("server finished");
    let requests = captured.lock().expect("captured requests");
    assert_eq!(requests.len(), 11);
    assert_request_basics(
        &requests[0],
        "POST",
        "/guilds/200/bulk-ban",
        Some("Bot typed-admin-token"),
    );
    assert!(requests[0].body.contains("delete_message_seconds"));
    assert_request_basics(
        &requests[1],
        "GET",
        "/guilds/200/roles/9",
        Some("Bot typed-admin-token"),
    );
    assert_request_basics(
        &requests[2],
        "GET",
        "/guilds/200/auto-moderation/rules",
        Some("Bot typed-admin-token"),
    );
    assert_request_basics(
        &requests[3],
        "GET",
        "/guilds/200/auto-moderation/rules/7",
        Some("Bot typed-admin-token"),
    );
    assert_request_basics(
        &requests[4],
        "POST",
        "/guilds/200/auto-moderation/rules",
        Some("Bot typed-admin-token"),
    );
    assert_request_basics(
        &requests[5],
        "PATCH",
        "/guilds/200/auto-moderation/rules/8",
        Some("Bot typed-admin-token"),
    );
    assert_request_basics(
        &requests[6],
        "GET",
        "/guilds/200/preview",
        Some("Bot typed-admin-token"),
    );
    assert_request_basics(
        &requests[7],
        "GET",
        "/guilds/200/prune?days=7&include_roles=9",
        Some("Bot typed-admin-token"),
    );
    assert_request_basics(
        &requests[8],
        "POST",
        "/guilds/200/prune?days=7&compute_prune_count=false",
        Some("Bot typed-admin-token"),
    );
    assert_request_basics(
        &requests[9],
        "GET",
        "/guilds/200/vanity-url",
        Some("Bot typed-admin-token"),
    );
    assert_request_basics(
        &requests[10],
        "GET",
        "/voice/regions",
        Some("Bot typed-admin-token"),
    );
}

#[tokio::test]
async fn typed_emoji_wrappers_hit_expected_paths() {
    let responses = vec![
        PlannedResponse::json(StatusCode::OK, json!([emoji_payload("1", "guild_one")])),
        PlannedResponse::json(StatusCode::OK, emoji_payload("1", "guild_one")),
        PlannedResponse::json(StatusCode::OK, emoji_payload("2", "guild_two")),
        PlannedResponse::json(StatusCode::OK, emoji_payload("2", "guild_two_edit")),
        PlannedResponse::json(
            StatusCode::OK,
            json!({ "items": [emoji_payload("3", "app_one")] }),
        ),
        PlannedResponse::json(StatusCode::OK, emoji_payload("3", "app_one")),
        PlannedResponse::json(StatusCode::OK, emoji_payload("4", "app_two")),
        PlannedResponse::json(StatusCode::OK, emoji_payload("4", "app_two_edit")),
    ];
    let (base_url, captured, server) = spawn_test_server(responses).await;
    let client = RestClient::new_with_base_url("emoji-token", 555, base_url);
    let body = json!({ "name": "emoji", "image": "data:image/png;base64,AAAA" });

    assert_eq!(
        client
            .get_guild_emojis_typed(Snowflake::from("200"))
            .await
            .unwrap()[0]
            .name
            .as_deref(),
        Some("guild_one")
    );
    assert_eq!(
        client
            .get_guild_emoji_typed(Snowflake::from("200"), Snowflake::from("1"))
            .await
            .unwrap()
            .id
            .as_deref(),
        Some("1")
    );
    assert_eq!(
        client
            .create_guild_emoji_typed(Snowflake::from("200"), &body)
            .await
            .unwrap()
            .name
            .as_deref(),
        Some("guild_two")
    );
    assert_eq!(
        client
            .modify_guild_emoji_typed(Snowflake::from("200"), Snowflake::from("2"), &body)
            .await
            .unwrap()
            .name
            .as_deref(),
        Some("guild_two_edit")
    );
    assert_eq!(
        client.get_application_emojis_typed().await.unwrap()[0]
            .name
            .as_deref(),
        Some("app_one")
    );
    assert_eq!(
        client
            .get_application_emoji_typed(Snowflake::from("3"))
            .await
            .unwrap()
            .id
            .as_deref(),
        Some("3")
    );
    assert_eq!(
        client
            .create_application_emoji_typed(&body)
            .await
            .unwrap()
            .name
            .as_deref(),
        Some("app_two")
    );
    assert_eq!(
        client
            .modify_application_emoji_typed(Snowflake::from("4"), &body)
            .await
            .unwrap()
            .name
            .as_deref(),
        Some("app_two_edit")
    );

    server.await.expect("server finished");
    let requests = captured.lock().expect("captured requests");
    assert_eq!(requests.len(), 8);
    assert_request_basics(
        &requests[0],
        "GET",
        "/guilds/200/emojis",
        Some("Bot emoji-token"),
    );
    assert_request_basics(
        &requests[1],
        "GET",
        "/guilds/200/emojis/1",
        Some("Bot emoji-token"),
    );
    assert_request_basics(
        &requests[2],
        "POST",
        "/guilds/200/emojis",
        Some("Bot emoji-token"),
    );
    assert_request_basics(
        &requests[3],
        "PATCH",
        "/guilds/200/emojis/2",
        Some("Bot emoji-token"),
    );
    assert_request_basics(
        &requests[4],
        "GET",
        "/applications/555/emojis",
        Some("Bot emoji-token"),
    );
    assert_request_basics(
        &requests[5],
        "GET",
        "/applications/555/emojis/3",
        Some("Bot emoji-token"),
    );
    assert_request_basics(
        &requests[6],
        "POST",
        "/applications/555/emojis",
        Some("Bot emoji-token"),
    );
    assert_request_basics(
        &requests[7],
        "PATCH",
        "/applications/555/emojis/4",
        Some("Bot emoji-token"),
    );
}

#[tokio::test]
async fn application_management_wrappers_hit_expected_paths() {
    let responses = vec![
        PlannedResponse::json(StatusCode::OK, application_payload("555", "app")),
        PlannedResponse::json(StatusCode::OK, application_payload("555", "renamed")),
        PlannedResponse::json(
            StatusCode::OK,
            json!([role_connection_metadata_payload("score")]),
        ),
        PlannedResponse::json(
            StatusCode::OK,
            json!([role_connection_metadata_payload("score")]),
        ),
    ];
    let (base_url, captured, server) = spawn_test_server(responses).await;
    let client = RestClient::new_with_base_url("app-token", 555, base_url);
    let records = vec![ApplicationRoleConnectionMetadata {
        kind: 1,
        key: "score".to_string(),
        name: "Score".to_string(),
        description: "User score".to_string(),
        ..ApplicationRoleConnectionMetadata::default()
    }];

    assert_eq!(client.get_current_application().await.unwrap().name, "app");
    assert_eq!(
        client
            .edit_current_application(&json!({ "description": "updated" }))
            .await
            .unwrap()
            .name,
        "renamed"
    );
    assert_eq!(
        client
            .get_application_role_connection_metadata_records()
            .await
            .unwrap()[0]
            .key,
        "score"
    );
    assert_eq!(
        client
            .update_application_role_connection_metadata_records(&records)
            .await
            .unwrap()[0]
            .description,
        "User score"
    );

    server.await.expect("server finished");
    let requests = captured.lock().expect("captured requests");
    assert_eq!(requests.len(), 4);
    assert_request_basics(
        &requests[0],
        "GET",
        "/applications/@me",
        Some("Bot app-token"),
    );
    assert_request_basics(
        &requests[1],
        "PATCH",
        "/applications/@me",
        Some("Bot app-token"),
    );
    assert_request_basics(
        &requests[2],
        "GET",
        "/applications/555/role-connections/metadata",
        Some("Bot app-token"),
    );
    assert_request_basics(
        &requests[3],
        "PUT",
        "/applications/555/role-connections/metadata",
        Some("Bot app-token"),
    );
}

#[tokio::test]
async fn remaining_admin_gap_wrappers_hit_expected_paths() {
    let responses = vec![
        PlannedResponse::json(StatusCode::OK, member_payload("700", "bot-member")),
        PlannedResponse::json(
            StatusCode::OK,
            json!([current_user_guild_payload("200", "guild")]),
        ),
        PlannedResponse::json(StatusCode::OK, webhook_payload("300", "hook")),
        PlannedResponse::json(
            StatusCode::OK,
            json!([{
                "id": "sydney",
                "name": "Sydney",
                "optimal": false,
                "deprecated": false,
                "custom": true
            }]),
        ),
    ];
    let (base_url, captured, server) = spawn_test_server(responses).await;
    let client = RestClient::new_with_base_url("gap-token", 555, base_url);

    assert_eq!(
        client
            .modify_current_member(Snowflake::from("200"), &json!({ "nick": "bot" }))
            .await
            .unwrap()
            .user
            .unwrap()
            .username,
        "bot-member"
    );
    assert_eq!(
        client.get_current_user_guilds_typed().await.unwrap()[0].name,
        "guild"
    );
    assert_eq!(
        client
            .get_webhook_with_token(Snowflake::from("300"), "token")
            .await
            .unwrap()
            .name
            .as_deref(),
        Some("hook")
    );
    assert_eq!(
        client
            .get_guild_voice_regions(Snowflake::from("200"))
            .await
            .unwrap()[0]
            .id,
        "sydney"
    );

    server.await.expect("server finished");
    let requests = captured.lock().expect("captured requests");
    assert_eq!(requests.len(), 4);
    assert_request_basics(
        &requests[0],
        "PATCH",
        "/guilds/200/members/@me",
        Some("Bot gap-token"),
    );
    assert_request_basics(
        &requests[1],
        "GET",
        "/users/@me/guilds",
        Some("Bot gap-token"),
    );
    assert_request_basics(&requests[2], "GET", "/webhooks/300/token", None);
    assert_request_basics(
        &requests[3],
        "GET",
        "/guilds/200/regions",
        Some("Bot gap-token"),
    );
}

#[tokio::test]
async fn sticker_stage_and_guild_admin_wrappers_hit_expected_paths() {
    let responses = vec![
        PlannedResponse::json(StatusCode::OK, json!({ "items": [{ "id": "1" }] })),
        PlannedResponse::json(StatusCode::OK, json!({ "id": "1" })),
        PlannedResponse::json(StatusCode::OK, json!({ "id": "2" })),
        PlannedResponse::json(StatusCode::OK, json!({ "id": "2", "name": "renamed" })),
        PlannedResponse::empty(StatusCode::NO_CONTENT),
        PlannedResponse::json(StatusCode::OK, sticker_payload("10")),
        PlannedResponse::json(
            StatusCode::OK,
            json!({
                "sticker_packs": [{
                    "id": "20",
                    "name": "pack",
                    "stickers": [sticker_payload("10")]
                }]
            }),
        ),
        PlannedResponse::json(StatusCode::OK, json!([sticker_payload("11")])),
        PlannedResponse::json(StatusCode::OK, sticker_payload("11")),
        PlannedResponse::json(StatusCode::OK, sticker_payload("12")),
        PlannedResponse::json(StatusCode::OK, sticker_payload("12")),
        PlannedResponse::empty(StatusCode::NO_CONTENT),
        PlannedResponse::json(StatusCode::OK, json!({ "pruned": 3 })),
        PlannedResponse::json(StatusCode::OK, json!({ "pruned": 2 })),
        PlannedResponse::json(
            StatusCode::OK,
            json!({ "enabled": true, "channel_id": "300" }),
        ),
        PlannedResponse::json(StatusCode::OK, json!({ "enabled": false })),
        PlannedResponse::json(StatusCode::OK, json!({ "id": "200", "name": "widget" })),
        PlannedResponse::json(
            StatusCode::OK,
            json!({ "channel_id": "100", "webhook_id": "101" }),
        ),
        PlannedResponse::json(StatusCode::OK, stage_payload("400")),
        PlannedResponse::json(StatusCode::OK, stage_payload("400")),
        PlannedResponse::json(StatusCode::OK, stage_payload("400")),
        PlannedResponse::empty(StatusCode::NO_CONTENT),
        PlannedResponse::json(StatusCode::OK, welcome_screen_payload()),
        PlannedResponse::json(StatusCode::OK, welcome_screen_payload()),
        PlannedResponse::json(StatusCode::OK, onboarding_payload()),
        PlannedResponse::json(StatusCode::OK, onboarding_payload()),
        PlannedResponse::json(StatusCode::OK, json!([template_payload("tmpl")])),
        PlannedResponse::json(StatusCode::OK, template_payload("tmpl")),
        PlannedResponse::json(StatusCode::OK, template_payload("tmpl")),
        PlannedResponse::json(StatusCode::OK, template_payload("tmpl")),
        PlannedResponse::json(StatusCode::OK, template_payload("tmpl")),
    ];
    let (base_url, captured, server) = spawn_test_server(responses).await;
    let client = RestClient::new_with_base_url("admin-token", 555, base_url);
    let body = json!({ "name": "name", "description": "desc", "tags": "tag" });

    assert_eq!(client.get_application_emojis().await.unwrap().len(), 1);
    assert_eq!(
        client
            .get_application_emoji(Snowflake::from("1"))
            .await
            .unwrap()["id"],
        json!("1")
    );
    assert_eq!(
        client.create_application_emoji(&body).await.unwrap()["id"],
        json!("2")
    );
    assert_eq!(
        client
            .modify_application_emoji(Snowflake::from("2"), &body)
            .await
            .unwrap()["name"],
        json!("renamed")
    );
    client
        .delete_application_emoji(Snowflake::from("2"))
        .await
        .unwrap();
    assert_eq!(
        client
            .get_sticker(Snowflake::from("10"))
            .await
            .unwrap()
            .name,
        "sticker"
    );
    assert_eq!(
        client
            .list_sticker_packs()
            .await
            .unwrap()
            .sticker_packs
            .len(),
        1
    );
    assert_eq!(
        client
            .get_guild_stickers(Snowflake::from("200"))
            .await
            .unwrap()
            .len(),
        1
    );
    assert_eq!(
        client
            .get_guild_sticker(Snowflake::from("200"), Snowflake::from("11"))
            .await
            .unwrap()
            .id
            .as_str(),
        "11"
    );
    assert_eq!(
        client
            .create_guild_sticker(
                Snowflake::from("200"),
                &body,
                sample_file("sticker.png", "png")
            )
            .await
            .unwrap()
            .id
            .as_str(),
        "12"
    );
    assert_eq!(
        client
            .modify_guild_sticker(Snowflake::from("200"), Snowflake::from("12"), &body)
            .await
            .unwrap()
            .id
            .as_str(),
        "12"
    );
    client
        .delete_guild_sticker(Snowflake::from("200"), Snowflake::from("12"))
        .await
        .unwrap();
    assert_eq!(
        client
            .get_guild_prune_count(Snowflake::from("200"), Some(7), &[Snowflake::from("9")])
            .await
            .unwrap()["pruned"],
        json!(3)
    );
    assert_eq!(
        client
            .begin_guild_prune(Snowflake::from("200"), Some(7), Some(false), &[])
            .await
            .unwrap()["pruned"],
        json!(2)
    );
    assert!(
        client
            .get_guild_widget_settings(Snowflake::from("200"))
            .await
            .unwrap()
            .enabled
    );
    assert!(
        !client
            .modify_guild_widget_settings(Snowflake::from("200"), &json!({ "enabled": false }))
            .await
            .unwrap()
            .enabled
    );
    assert_eq!(
        client
            .get_guild_widget(Snowflake::from("200"))
            .await
            .unwrap()["name"],
        json!("widget")
    );
    assert_eq!(
        client
            .follow_announcement_channel(Snowflake::from("100"), Snowflake::from("101"))
            .await
            .unwrap()
            .webhook_id
            .as_str(),
        "101"
    );
    assert_eq!(
        client
            .create_stage_instance(&body)
            .await
            .unwrap()
            .channel_id
            .as_str(),
        "400"
    );
    assert_eq!(
        client
            .get_stage_instance(Snowflake::from("400"))
            .await
            .unwrap()
            .topic,
        "town hall"
    );
    assert_eq!(
        client
            .modify_stage_instance(Snowflake::from("400"), &body)
            .await
            .unwrap()
            .privacy_level,
        2
    );
    client
        .delete_stage_instance(Snowflake::from("400"))
        .await
        .unwrap();
    assert_eq!(
        client
            .get_guild_welcome_screen(Snowflake::from("200"))
            .await
            .unwrap()
            .welcome_channels
            .len(),
        1
    );
    assert_eq!(
        client
            .modify_guild_welcome_screen(Snowflake::from("200"), &welcome_screen_payload())
            .await
            .unwrap()
            .description
            .as_deref(),
        Some("welcome")
    );
    assert!(
        client
            .get_guild_onboarding(Snowflake::from("200"))
            .await
            .unwrap()
            .enabled
    );
    assert!(
        client
            .modify_guild_onboarding(Snowflake::from("200"), &onboarding_payload())
            .await
            .unwrap()
            .enabled
    );
    assert_eq!(
        client
            .get_guild_templates(Snowflake::from("200"))
            .await
            .unwrap()
            .len(),
        1
    );
    assert_eq!(
        client
            .create_guild_template(Snowflake::from("200"), &body)
            .await
            .unwrap()
            .code,
        "tmpl"
    );
    assert_eq!(
        client
            .sync_guild_template(Snowflake::from("200"), "tmpl")
            .await
            .unwrap()
            .code,
        "tmpl"
    );
    assert_eq!(
        client
            .modify_guild_template(Snowflake::from("200"), "tmpl", &body)
            .await
            .unwrap()
            .code,
        "tmpl"
    );
    assert_eq!(
        client
            .delete_guild_template(Snowflake::from("200"), "tmpl")
            .await
            .unwrap()
            .code,
        "tmpl"
    );

    server.await.expect("server finished");
    let requests = captured.lock().expect("captured requests");
    assert_eq!(requests.len(), 31);
    assert_request_basics(
        &requests[0],
        "GET",
        "/applications/555/emojis",
        Some("Bot admin-token"),
    );
    assert_request_basics(
        &requests[4],
        "DELETE",
        "/applications/555/emojis/2",
        Some("Bot admin-token"),
    );
    assert_request_basics(&requests[5], "GET", "/stickers/10", Some("Bot admin-token"));
    assert_request_basics(
        &requests[7],
        "GET",
        "/guilds/200/stickers",
        Some("Bot admin-token"),
    );
    assert_eq!(requests[9].method, "POST");
    assert_eq!(requests[9].path, "/guilds/200/stickers");
    assert_eq!(requests[9].header("authorization"), Some("Bot admin-token"));
    assert!(requests[9]
        .header("content-type")
        .is_some_and(|value| value.starts_with("multipart/form-data; boundary=")));
    assert!(requests[9].body.contains(r#"name="file""#));
    assert_request_basics(
        &requests[12],
        "GET",
        "/guilds/200/prune?days=7&include_roles=9",
        Some("Bot admin-token"),
    );
    assert_request_basics(
        &requests[13],
        "POST",
        "/guilds/200/prune?days=7&compute_prune_count=false",
        Some("Bot admin-token"),
    );
    assert_request_basics(
        &requests[17],
        "POST",
        "/channels/100/followers",
        Some("Bot admin-token"),
    );
    assert_request_basics(
        &requests[18],
        "POST",
        "/stage-instances",
        Some("Bot admin-token"),
    );
    assert_request_basics(
        &requests[22],
        "GET",
        "/guilds/200/welcome-screen",
        Some("Bot admin-token"),
    );
    assert_request_basics(
        &requests[25],
        "PUT",
        "/guilds/200/onboarding",
        Some("Bot admin-token"),
    );
    assert_request_basics(
        &requests[28],
        "PUT",
        "/guilds/200/templates/tmpl",
        Some("Bot admin-token"),
    );
}

#[tokio::test]
async fn scheduled_event_wrappers_return_typed_models() {
    let responses = vec![
        PlannedResponse::json(StatusCode::OK, json!([scheduled_event_payload("1")])),
        PlannedResponse::json(StatusCode::OK, scheduled_event_payload("2")),
        PlannedResponse::json(StatusCode::OK, scheduled_event_payload("2")),
        PlannedResponse::json(StatusCode::OK, scheduled_event_payload("2")),
        PlannedResponse::json(StatusCode::OK, scheduled_event_payload("2")),
        PlannedResponse::empty(StatusCode::NO_CONTENT),
        PlannedResponse::json(
            StatusCode::OK,
            json!([{
                "guild_scheduled_event_id": "2",
                "user": {
                    "id": "500",
                    "username": "attendee",
                    "discriminator": "0000",
                    "bot": false
                }
            }]),
        ),
    ];
    let (base_url, captured, server) = spawn_test_server(responses).await;
    let client = RestClient::new_with_base_url("scheduled-token", 0, base_url);
    let body = json!({ "name": "community night" });

    assert_eq!(
        client
            .get_guild_scheduled_events(Snowflake::from("200"))
            .await
            .unwrap()[0]
            .name,
        "community night"
    );
    assert_eq!(
        client
            .create_guild_scheduled_event_typed(Snowflake::from("200"), &body)
            .await
            .unwrap()
            .entity_metadata
            .and_then(|metadata| metadata.location),
        Some("Stage".to_string())
    );
    assert_eq!(
        client
            .get_guild_scheduled_event(Snowflake::from("200"), Snowflake::from("2"))
            .await
            .unwrap()
            .recurrence_rule
            .map(|rule| rule.interval),
        Some(1)
    );
    assert_eq!(
        client
            .modify_guild_scheduled_event_typed(Snowflake::from("200"), Snowflake::from("2"), &body)
            .await
            .unwrap()
            .status,
        1
    );
    assert_eq!(
        client
            .get_guild_scheduled_event(Snowflake::from("200"), Snowflake::from("2"))
            .await
            .unwrap()
            .user_count,
        Some(5)
    );
    client
        .delete_guild_scheduled_event(Snowflake::from("200"), Snowflake::from("2"))
        .await
        .unwrap();
    assert_eq!(
        client
            .get_guild_scheduled_event_users(Snowflake::from("200"), Snowflake::from("2"), Some(50))
            .await
            .unwrap()[0]
            .user
            .username,
        "attendee"
    );

    server.await.expect("server finished");
    let requests = captured.lock().expect("captured requests");
    assert_eq!(requests.len(), 7);
    assert_request_basics(
        &requests[0],
        "GET",
        "/guilds/200/scheduled-events",
        Some("Bot scheduled-token"),
    );
    assert_request_basics(
        &requests[5],
        "DELETE",
        "/guilds/200/scheduled-events/2",
        Some("Bot scheduled-token"),
    );
    assert_request_basics(
        &requests[6],
        "GET",
        "/guilds/200/scheduled-events/2/users?limit=50",
        Some("Bot scheduled-token"),
    );
}

#[tokio::test]
async fn monetization_and_soundboard_wrappers_hit_expected_paths() {
    let responses = vec![
        PlannedResponse::json(StatusCode::OK, json!([sku_payload("900")])),
        PlannedResponse::json(StatusCode::OK, json!([subscription_payload("950")])),
        PlannedResponse::json(StatusCode::OK, subscription_payload("950")),
        PlannedResponse::json(StatusCode::OK, json!([entitlement_payload("901")])),
        PlannedResponse::json(StatusCode::OK, entitlement_payload("901")),
        PlannedResponse::empty(StatusCode::NO_CONTENT),
        PlannedResponse::json(StatusCode::OK, entitlement_payload("902")),
        PlannedResponse::empty(StatusCode::NO_CONTENT),
        PlannedResponse::empty(StatusCode::NO_CONTENT),
        PlannedResponse::json(StatusCode::OK, json!([soundboard_payload("1")])),
        PlannedResponse::json(
            StatusCode::OK,
            json!({ "items": [soundboard_payload("2")] }),
        ),
        PlannedResponse::json(StatusCode::OK, soundboard_payload("2")),
        PlannedResponse::json(StatusCode::OK, soundboard_payload("3")),
        PlannedResponse::json(StatusCode::OK, soundboard_payload("3")),
        PlannedResponse::empty(StatusCode::NO_CONTENT),
    ];
    let (base_url, captured, server) = spawn_test_server(responses).await;
    let client = RestClient::new_with_base_url("premium-token", 555, base_url);
    let query = EntitlementQuery {
        user_id: Some(Snowflake::from("777")),
        sku_ids: vec![Snowflake::from("900"), Snowflake::from("901")],
        limit: Some(25),
        guild_id: Some(Snowflake::from("200")),
        exclude_ended: Some(true),
        exclude_deleted: Some(false),
        ..EntitlementQuery::default()
    };
    let subscription_query = SubscriptionQuery {
        user_id: Some(Snowflake::from("777")),
        limit: Some(10),
        ..SubscriptionQuery::default()
    };
    let sound_body = json!({ "sound_id": "1", "source_guild_id": "200" });

    assert_eq!(client.get_skus().await.unwrap()[0].slug, "premium");
    assert_eq!(
        client
            .get_sku_subscriptions(Snowflake::from("900"), &subscription_query)
            .await
            .unwrap()[0]
            .id
            .as_str(),
        "950"
    );
    assert_eq!(
        client
            .get_sku_subscription(Snowflake::from("900"), Snowflake::from("950"))
            .await
            .unwrap()
            .user_id
            .as_str(),
        "777"
    );
    assert_eq!(
        client.get_entitlements(&query).await.unwrap()[0]
            .sku_id
            .as_str(),
        "900"
    );
    assert_eq!(
        client
            .get_entitlement(Snowflake::from("901"))
            .await
            .unwrap()
            .user_id
            .unwrap()
            .as_str(),
        "777"
    );
    client
        .consume_entitlement(Snowflake::from("901"))
        .await
        .unwrap();
    assert_eq!(
        client
            .create_test_entitlement(&CreateTestEntitlement {
                sku_id: Snowflake::from("900"),
                owner_id: Snowflake::from("200"),
                owner_type: 1,
            })
            .await
            .unwrap()
            .id
            .as_str(),
        "902"
    );
    client
        .delete_test_entitlement(Snowflake::from("902"))
        .await
        .unwrap();
    client
        .send_soundboard_sound(Snowflake::from("300"), &sound_body)
        .await
        .unwrap();
    assert_eq!(
        client.list_default_soundboard_sounds().await.unwrap().len(),
        1
    );
    assert_eq!(
        client
            .list_guild_soundboard_sounds(Snowflake::from("200"))
            .await
            .unwrap()
            .items
            .len(),
        1
    );
    assert_eq!(
        client
            .get_guild_soundboard_sound(Snowflake::from("200"), Snowflake::from("2"))
            .await
            .unwrap()
            .name,
        "quack"
    );
    assert_eq!(
        client
            .create_guild_soundboard_sound(Snowflake::from("200"), &sound_body)
            .await
            .unwrap()
            .sound_id
            .as_str(),
        "3"
    );
    assert_eq!(
        client
            .modify_guild_soundboard_sound(
                Snowflake::from("200"),
                Snowflake::from("3"),
                &json!({ "name": "quack" })
            )
            .await
            .unwrap()
            .sound_id
            .as_str(),
        "3"
    );
    client
        .delete_guild_soundboard_sound(Snowflake::from("200"), Snowflake::from("3"))
        .await
        .unwrap();

    server.await.expect("server finished");
    let requests = captured.lock().expect("captured requests");
    assert_eq!(requests.len(), 15);
    assert_request_basics(
        &requests[0],
        "GET",
        "/applications/555/skus",
        Some("Bot premium-token"),
    );
    assert_request_basics(
        &requests[1],
        "GET",
        "/skus/900/subscriptions?limit=10&user_id=777",
        Some("Bot premium-token"),
    );
    assert_request_basics(
        &requests[2],
        "GET",
        "/skus/900/subscriptions/950",
        Some("Bot premium-token"),
    );
    assert_request_basics(
            &requests[3],
            "GET",
            "/applications/555/entitlements?user_id=777&sku_ids=900,901&limit=25&guild_id=200&exclude_ended=true&exclude_deleted=false",
            Some("Bot premium-token"),
        );
    assert_request_basics(
        &requests[5],
        "POST",
        "/applications/555/entitlements/901/consume",
        Some("Bot premium-token"),
    );
    assert_request_basics(
        &requests[8],
        "POST",
        "/channels/300/send-soundboard-sound",
        Some("Bot premium-token"),
    );
    assert_request_basics(
        &requests[9],
        "GET",
        "/soundboard-default-sounds",
        Some("Bot premium-token"),
    );
    assert_request_basics(
        &requests[14],
        "DELETE",
        "/guilds/200/soundboard-sounds/3",
        Some("Bot premium-token"),
    );
}

#[tokio::test]
async fn poll_thread_invite_and_integration_wrappers_hit_expected_paths() {
    let thread_list = json!({
        "threads": [{ "id": "700", "type": 11, "name": "thread" }],
        "members": [{ "id": "700", "user_id": "777", "flags": 0 }],
        "has_more": false
    });
    let responses = vec![
        PlannedResponse::empty(StatusCode::NO_CONTENT),
        PlannedResponse::empty(StatusCode::NO_CONTENT),
        PlannedResponse::json(
            StatusCode::OK,
            json!({ "id": "700", "user_id": "777", "flags": 0 }),
        ),
        PlannedResponse::json(
            StatusCode::OK,
            json!([{ "id": "700", "user_id": "777", "flags": 0 }]),
        ),
        PlannedResponse::json(StatusCode::OK, thread_list.clone()),
        PlannedResponse::json(StatusCode::OK, thread_list.clone()),
        PlannedResponse::json(StatusCode::OK, thread_list.clone()),
        PlannedResponse::json(StatusCode::OK, thread_list),
        PlannedResponse::json(StatusCode::OK, json!({ "code": "abc", "uses": 2 })),
        PlannedResponse::json(
            StatusCode::OK,
            json!([{
                "id": "900",
                "name": "integration",
                "type": "discord",
                "enabled": true,
                "account": { "id": "acc", "name": "account" }
            }]),
        ),
        PlannedResponse::empty(StatusCode::NO_CONTENT),
        PlannedResponse::json(
            StatusCode::OK,
            json!({ "users": [{ "id": "777", "username": "voter" }] }),
        ),
        PlannedResponse::json(
            StatusCode::OK,
            json!({ "id": "800", "channel_id": "100", "content": "poll ended" }),
        ),
    ];
    let (base_url, captured, server) = spawn_test_server(responses).await;
    let client = RestClient::new_with_base_url("coverage-token", 555, base_url);

    client
        .add_thread_member(Snowflake::from("700"), Snowflake::from("777"))
        .await
        .unwrap();
    client
        .remove_thread_member(Snowflake::from("700"), Snowflake::from("777"))
        .await
        .unwrap();
    assert_eq!(
        client
            .get_thread_member(Snowflake::from("700"), Snowflake::from("777"), Some(true))
            .await
            .unwrap()
            .user_id
            .unwrap()
            .as_str(),
        "777"
    );
    assert_eq!(
        client
            .list_thread_members(
                Snowflake::from("700"),
                &ThreadMemberQuery {
                    with_member: Some(true),
                    after: Some(Snowflake::from("10")),
                    limit: Some(25),
                },
            )
            .await
            .unwrap()
            .len(),
        1
    );
    assert_eq!(
        client
            .list_public_archived_threads(
                Snowflake::from("100"),
                &ArchivedThreadsQuery {
                    before: Some("2026-04-29T00:00:00Z".to_string()),
                    limit: Some(50),
                },
            )
            .await
            .unwrap()
            .threads
            .len(),
        1
    );
    client
        .list_private_archived_threads(Snowflake::from("100"), &ArchivedThreadsQuery::default())
        .await
        .unwrap();
    client
        .list_joined_private_archived_threads(
            Snowflake::from("100"),
            &JoinedArchivedThreadsQuery {
                before: Some(Snowflake::from("700")),
                limit: Some(10),
            },
        )
        .await
        .unwrap();
    client
        .get_active_guild_threads(Snowflake::from("200"))
        .await
        .unwrap();
    assert_eq!(
        client
            .get_invite_with_options("abc", Some(true), Some(true), Some(Snowflake::from("300")))
            .await
            .unwrap()
            .uses,
        Some(2)
    );
    assert_eq!(
        client
            .get_guild_integrations(Snowflake::from("200"))
            .await
            .unwrap()[0]
            .id
            .as_str(),
        "900"
    );
    client
        .delete_guild_integration(Snowflake::from("200"), Snowflake::from("900"))
        .await
        .unwrap();
    assert_eq!(
        client
            .get_poll_answer_voters(
                Snowflake::from("100"),
                Snowflake::from("800"),
                1,
                Some(Snowflake::from("777")),
                Some(10),
            )
            .await
            .unwrap()
            .users[0]
            .username,
        "voter"
    );
    assert_eq!(
        client
            .end_poll(Snowflake::from("100"), Snowflake::from("800"))
            .await
            .unwrap()
            .content,
        "poll ended"
    );

    server.await.expect("server finished");
    let requests = captured.lock().expect("captured requests");
    assert_eq!(requests.len(), 13);
    assert_request_basics(
        &requests[0],
        "PUT",
        "/channels/700/thread-members/777",
        Some("Bot coverage-token"),
    );
    assert_request_basics(
        &requests[3],
        "GET",
        "/channels/700/thread-members?with_member=true&after=10&limit=25",
        Some("Bot coverage-token"),
    );
    assert_request_basics(
        &requests[4],
        "GET",
        "/channels/100/threads/archived/public?before=2026-04-29T00:00:00Z&limit=50",
        Some("Bot coverage-token"),
    );
    assert_request_basics(
        &requests[6],
        "GET",
        "/channels/100/users/@me/threads/archived/private?before=700&limit=10",
        Some("Bot coverage-token"),
    );
    assert_request_basics(
        &requests[7],
        "GET",
        "/guilds/200/threads/active",
        Some("Bot coverage-token"),
    );
    assert_request_basics(
        &requests[8],
        "GET",
        "/invites/abc?with_counts=true&with_expiration=true&guild_scheduled_event_id=300",
        Some("Bot coverage-token"),
    );
    assert_request_basics(
        &requests[10],
        "DELETE",
        "/guilds/200/integrations/900",
        Some("Bot coverage-token"),
    );
    assert_request_basics(
        &requests[11],
        "GET",
        "/channels/100/polls/800/answers/1?after=777&limit=10",
        Some("Bot coverage-token"),
    );
    assert_request_basics(
        &requests[12],
        "POST",
        "/channels/100/polls/800/expire",
        Some("Bot coverage-token"),
    );
}

#[tokio::test]
async fn client_methods_reject_missing_application_id_before_request() {
    let client = RestClient::new("token", 0);
    let command = sample_command();
    let commands = vec![command.clone()];
    let body = sample_message();

    assert_model_error_contains(
        client
            .bulk_overwrite_global_commands_typed(&commands)
            .await
            .unwrap_err(),
        "application_id must be set",
    );
    assert_model_error_contains(
        client.create_global_command(&command).await.unwrap_err(),
        "application_id must be set",
    );
    assert_model_error_contains(
        client.get_global_commands().await.unwrap_err(),
        "application_id must be set",
    );
    assert_model_error_contains(
        client
            .bulk_overwrite_guild_commands_typed(Snowflake::from("456"), &commands)
            .await
            .unwrap_err(),
        "application_id must be set",
    );
    assert_model_error_contains(
        client
            .create_followup_message_json("token", &json!({ "content": "hi" }))
            .await
            .unwrap_err(),
        "application_id must be set",
    );
    assert_model_error_contains(
        client
            .create_followup_message("token", &body)
            .await
            .unwrap_err(),
        "application_id must be set",
    );
    assert_model_error_contains(
        client
            .get_original_interaction_response("token")
            .await
            .unwrap_err(),
        "application_id must be set",
    );
    assert_model_error_contains(
        client
            .edit_original_interaction_response("token", &body)
            .await
            .unwrap_err(),
        "application_id must be set",
    );
    assert_model_error_contains(
        client
            .delete_original_interaction_response("token")
            .await
            .unwrap_err(),
        "application_id must be set",
    );
    assert_model_error_contains(
        client
            .edit_followup_message("token", "123", &body)
            .await
            .unwrap_err(),
        "application_id must be set",
    );
    assert_model_error_contains(
        client
            .delete_followup_message("token", "123")
            .await
            .unwrap_err(),
        "application_id must be set",
    );
}

#[tokio::test]
async fn client_methods_reject_unsafe_tokens_before_request() {
    let client = RestClient::new("token", 123);
    let body = sample_message();
    let response = sample_interaction_response();

    assert_model_error_contains(
        client
            .execute_webhook(Snowflake::from("456"), "bad/token", &json!({}))
            .await
            .unwrap_err(),
        "webhook_token",
    );
    assert_model_error_contains(
        client
            .create_interaction_response_typed(Snowflake::from("789"), "bad/token", &response)
            .await
            .unwrap_err(),
        "interaction_token",
    );
    assert_model_error_contains(
        client
            .create_interaction_response_json(Snowflake::from("789"), "bad/token", &json!({}))
            .await
            .unwrap_err(),
        "interaction_token",
    );
    assert_model_error_contains(
        client
            .create_followup_message_with_application_id("123", "bad/token", &body)
            .await
            .unwrap_err(),
        "interaction_token",
    );
    assert_model_error_contains(
        client
            .create_followup_message_json_with_application_id(
                "123",
                "bad/token",
                &json!({ "content": "hi" }),
            )
            .await
            .unwrap_err(),
        "interaction_token",
    );
    assert_model_error_contains(
        client
            .get_original_interaction_response_with_application_id("123", "bad/token")
            .await
            .unwrap_err(),
        "interaction_token",
    );
    assert_model_error_contains(
        client
            .edit_original_interaction_response_with_application_id("123", "bad/token", &body)
            .await
            .unwrap_err(),
        "interaction_token",
    );
    assert_model_error_contains(
        client
            .delete_original_interaction_response_with_application_id("123", "bad/token")
            .await
            .unwrap_err(),
        "interaction_token",
    );
}

#[tokio::test]
async fn client_followup_methods_validate_application_and_message_segments() {
    let client = RestClient::new("token", 123);
    let body = sample_message();

    assert_model_error_contains(
        client
            .create_followup_message_json_with_application_id(
                "12/3",
                "token",
                &json!({ "content": "hi" }),
            )
            .await
            .unwrap_err(),
        "application_id",
    );
    assert_model_error_contains(
        client
            .create_followup_message_with_application_id("12/3", "token", &body)
            .await
            .unwrap_err(),
        "application_id",
    );
    assert_model_error_contains(
        client
            .edit_followup_message_with_application_id("123", "token", "bad/id", &body)
            .await
            .unwrap_err(),
        "message_id",
    );
    assert_model_error_contains(
        client
            .delete_followup_message_with_application_id("123", "token", "bad/id")
            .await
            .unwrap_err(),
        "message_id",
    );
}

#[test]
fn header_string_returns_none_for_invalid_header_bytes() {
    let invalid = HeaderValue::from_bytes(&[0xFF]).expect("invalid but allowed header bytes");
    assert_eq!(header_string(Some(&invalid)), None);
}

#[test]
fn rate_limit_state_does_not_block_without_reset_after_header() {
    let state = RateLimitState::default();
    let mut headers = HeaderMap::new();
    headers.insert("x-ratelimit-remaining", HeaderValue::from_static("0"));

    state.observe("GET:channels/123/messages", &headers, StatusCode::OK, "");

    assert!(state.wait_duration("GET:channels/123/messages").is_none());
}

#[tokio::test]
async fn sleep_for_retry_after_clamps_negative_values() {
    let start = Instant::now();
    sleep_for_retry_after(-1.0).await;
    assert!(start.elapsed() < Duration::from_millis(50));
}

#[tokio::test]
async fn client_message_and_channel_wrappers_hit_local_server() {
    let responses = vec![
        PlannedResponse::json(StatusCode::OK, message_payload("1", "100", "hello")),
        PlannedResponse::json(StatusCode::OK, message_payload("1", "100", "updated")),
        PlannedResponse::json(StatusCode::OK, message_payload("1", "100", "updated")),
        PlannedResponse::json(StatusCode::OK, channel_payload("100", 0, Some("general"))),
        PlannedResponse::json(StatusCode::OK, channel_payload("100", 0, Some("general"))),
        PlannedResponse::json(StatusCode::OK, channel_payload("100", 0, Some("renamed"))),
        PlannedResponse::json(
            StatusCode::OK,
            json!([message_payload("1", "100", "updated")]),
        ),
        PlannedResponse::json(
            StatusCode::OK,
            json!([message_payload("2", "100", "latest")]),
        ),
        PlannedResponse::empty(StatusCode::NO_CONTENT),
        PlannedResponse::empty(StatusCode::NO_CONTENT),
        PlannedResponse::empty(StatusCode::NO_CONTENT),
        PlannedResponse::json(StatusCode::OK, message_payload("1", "100", "crosspost")),
        PlannedResponse::json(
            StatusCode::OK,
            json!([message_payload("3", "100", "paginated")]),
        ),
        PlannedResponse::json(
            StatusCode::OK,
            json!([{ "id": "42", "username": "reactor" }]),
        ),
        PlannedResponse::empty(StatusCode::NO_CONTENT),
        PlannedResponse::empty(StatusCode::NO_CONTENT),
        PlannedResponse::empty(StatusCode::NO_CONTENT),
        PlannedResponse::json(StatusCode::OK, json!({ "ok": true })),
        PlannedResponse::json(StatusCode::OK, json!({ "id": "903", "content": "raw" })),
        PlannedResponse::json(
            StatusCode::OK,
            json!({ "id": "903", "content": "edited raw" }),
        ),
        PlannedResponse::empty(StatusCode::NO_CONTENT),
    ];
    let (base_url, captured, server) = spawn_test_server(responses).await;
    let client = RestClient::new_with_base_url("token", 321, base_url);
    let body = sample_message();

    let created = client
        .create_message(Snowflake::from("100"), &body)
        .await
        .expect("create message");
    assert_eq!(created.content, "hello");

    let updated = client
        .update_message(Snowflake::from("100"), Snowflake::from("1"), &body)
        .await
        .expect("update message");
    assert_eq!(updated.content, "updated");

    let fetched = client
        .get_message(Snowflake::from("100"), Snowflake::from("1"))
        .await
        .expect("get message");
    assert_eq!(fetched.content, "updated");

    let channel = client
        .get_channel(Snowflake::from("100"))
        .await
        .expect("get channel");
    assert_eq!(channel.name.as_deref(), Some("general"));

    let deleted_channel = client
        .delete_channel(Snowflake::from("100"))
        .await
        .expect("delete channel");
    assert_eq!(deleted_channel.id.as_str(), "100");

    let renamed = client
        .update_channel_typed(Snowflake::from("100"), &json!({ "name": "renamed" }))
        .await
        .expect("update channel");
    assert_eq!(renamed.name.as_deref(), Some("renamed"));

    let limited_messages = client
        .get_channel_messages(Snowflake::from("100"), Some(2))
        .await
        .expect("channel messages with limit");
    assert_eq!(limited_messages.len(), 1);

    let all_messages = client
        .get_channel_messages(Snowflake::from("100"), None)
        .await
        .expect("channel messages without limit");
    assert_eq!(all_messages[0].content, "latest");

    client
        .bulk_delete_messages(
            Snowflake::from("100"),
            vec![Snowflake::from("1"), Snowflake::from("2")],
        )
        .await
        .expect("bulk delete");
    client
        .add_reaction(Snowflake::from("100"), Snowflake::from("1"), "spark")
        .await
        .expect("add reaction");
    client
        .remove_reaction(Snowflake::from("100"), Snowflake::from("1"), "spark")
        .await
        .expect("remove reaction");

    assert_eq!(
        client
            .crosspost_message(Snowflake::from("100"), Snowflake::from("1"))
            .await
            .expect("crosspost message")
            .content,
        "crosspost"
    );
    assert_eq!(
        client
            .get_channel_messages_paginated(
                Snowflake::from("100"),
                Some(50),
                Some(Snowflake::from("10")),
                Some(Snowflake::from("11")),
                Some(Snowflake::from("12")),
            )
            .await
            .expect("paginated messages")[0]
            .content,
        "paginated"
    );
    assert_eq!(
        client
            .get_reactions(
                Snowflake::from("100"),
                Snowflake::from("1"),
                "spark",
                Some(25),
                Some(Snowflake::from("42")),
            )
            .await
            .expect("get reactions")[0]
            .username,
        "reactor"
    );
    client
        .remove_user_reaction(
            Snowflake::from("100"),
            Snowflake::from("1"),
            "spark",
            Snowflake::from("42"),
        )
        .await
        .expect("remove user reaction");
    client
        .remove_all_reactions(Snowflake::from("100"), Snowflake::from("1"))
        .await
        .expect("remove all reactions");
    client
        .remove_all_reactions_for_emoji(Snowflake::from("100"), Snowflake::from("1"), "spark")
        .await
        .expect("remove all reactions for emoji");

    let raw = client
        .request(
            Method::GET,
            "channels/100/custom",
            Option::<&serde_json::Value>::None,
        )
        .await
        .expect("request with normalized path");
    assert_eq!(raw["ok"], json!(true));

    let sent = client
        .send_message_json(Snowflake::from("100"), &json!({ "content": "raw" }))
        .await
        .expect("send raw message");
    assert_eq!(sent["id"], json!("903"));

    let edited = client
        .edit_message_json(
            Snowflake::from("100"),
            Snowflake::from("903"),
            &json!({ "content": "edited raw" }),
        )
        .await
        .expect("edit raw message");
    assert_eq!(edited["content"], json!("edited raw"));

    client
        .delete_message(Snowflake::from("100"), Snowflake::from("903"))
        .await
        .expect("delete raw message");

    server.await.expect("server finished");
    let requests = captured.lock().expect("captured requests");
    let auth = Some("Bot token");

    assert_request_basics(&requests[0], "POST", "/channels/100/messages", auth);
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&requests[0].body).unwrap()["content"],
        json!("hello")
    );
    assert_request_basics(&requests[1], "PATCH", "/channels/100/messages/1", auth);
    assert_request_basics(&requests[2], "GET", "/channels/100/messages/1", auth);
    assert_request_basics(&requests[3], "GET", "/channels/100", auth);
    assert_request_basics(&requests[4], "DELETE", "/channels/100", auth);
    assert_request_basics(&requests[5], "PATCH", "/channels/100", auth);
    assert_request_basics(&requests[6], "GET", "/channels/100/messages?limit=2", auth);
    assert_request_basics(&requests[7], "GET", "/channels/100/messages", auth);
    assert_request_basics(
        &requests[8],
        "POST",
        "/channels/100/messages/bulk-delete",
        auth,
    );
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&requests[8].body).unwrap(),
        json!({ "messages": ["1", "2"] })
    );
    assert_request_basics(
        &requests[9],
        "PUT",
        "/channels/100/messages/1/reactions/spark/@me",
        auth,
    );
    assert_request_basics(
        &requests[10],
        "DELETE",
        "/channels/100/messages/1/reactions/spark/@me",
        auth,
    );
    assert_request_basics(
        &requests[11],
        "POST",
        "/channels/100/messages/1/crosspost",
        auth,
    );
    assert_request_basics(
        &requests[12],
        "GET",
        "/channels/100/messages?limit=50&before=10&after=11&around=12",
        auth,
    );
    assert_request_basics(
        &requests[13],
        "GET",
        "/channels/100/messages/1/reactions/spark?limit=25&after=42",
        auth,
    );
    assert_request_basics(
        &requests[14],
        "DELETE",
        "/channels/100/messages/1/reactions/spark/42",
        auth,
    );
    assert_request_basics(
        &requests[15],
        "DELETE",
        "/channels/100/messages/1/reactions",
        auth,
    );
    assert_request_basics(
        &requests[16],
        "DELETE",
        "/channels/100/messages/1/reactions/spark",
        auth,
    );
    assert_request_basics(&requests[17], "GET", "/channels/100/custom", auth);
    assert_request_basics(&requests[18], "POST", "/channels/100/messages", auth);
    assert_request_basics(&requests[19], "PATCH", "/channels/100/messages/903", auth);
    assert_request_basics(&requests[20], "DELETE", "/channels/100/messages/903", auth);
}

#[tokio::test]
async fn client_guild_and_command_wrappers_hit_local_server() {
    let responses = vec![
        PlannedResponse::json(StatusCode::OK, guild_payload("200", "guild")),
        PlannedResponse::json(StatusCode::OK, guild_payload("200", "guild-updated")),
        PlannedResponse::json(
            StatusCode::OK,
            json!([channel_payload("201", 0, Some("rules"))]),
        ),
        PlannedResponse::json(StatusCode::OK, channel_payload("202", 0, Some("new"))),
        PlannedResponse::json(StatusCode::OK, json!([{}])),
        PlannedResponse::json(StatusCode::OK, json!([{}])),
        PlannedResponse::empty(StatusCode::NO_CONTENT),
        PlannedResponse::empty(StatusCode::NO_CONTENT),
        PlannedResponse::empty(StatusCode::NO_CONTENT),
        PlannedResponse::json(StatusCode::OK, role_payload("300", "admin")),
        PlannedResponse::json(StatusCode::OK, role_payload("300", "mod")),
        PlannedResponse::empty(StatusCode::NO_CONTENT),
        PlannedResponse::json(StatusCode::OK, json!({})),
        PlannedResponse::json(StatusCode::OK, json!([role_payload("300", "mod")])),
        PlannedResponse::json(
            StatusCode::OK,
            json!([command_payload("401", "ping", "pong")]),
        ),
        PlannedResponse::json(StatusCode::OK, command_payload("402", "pong", "reply")),
        PlannedResponse::json(
            StatusCode::OK,
            json!([command_payload("401", "ping", "pong")]),
        ),
        PlannedResponse::json(StatusCode::OK, gateway_payload()),
        PlannedResponse::json(
            StatusCode::OK,
            json!([command_payload("403", "guild", "only")]),
        ),
    ];
    let (base_url, captured, server) = spawn_test_server(responses).await;
    let client = RestClient::new_with_base_url("guild-token", 0, base_url);
    client.set_application_id(555);
    let command = sample_command();

    let guild = client
        .get_guild(Snowflake::from("200"))
        .await
        .expect("get guild");
    assert_eq!(guild.name, "guild");

    let updated = client
        .update_guild(Snowflake::from("200"), &json!({ "name": "guild-updated" }))
        .await
        .expect("update guild");
    assert_eq!(updated.name, "guild-updated");

    let channels = client
        .get_guild_channels(Snowflake::from("200"))
        .await
        .expect("get guild channels");
    assert_eq!(channels.len(), 1);

    let created_channel = client
        .create_guild_channel(Snowflake::from("200"), &json!({ "name": "new" }))
        .await
        .expect("create guild channel");
    assert_eq!(created_channel.id.as_str(), "202");

    assert_eq!(
        client
            .get_guild_members(Snowflake::from("200"), Some(3))
            .await
            .expect("members with limit")
            .len(),
        1
    );
    assert_eq!(
        client
            .get_guild_members(Snowflake::from("200"), None)
            .await
            .expect("members without limit")
            .len(),
        1
    );
    client
        .remove_guild_member(Snowflake::from("200"), Snowflake::from("201"))
        .await
        .expect("remove guild member");
    client
        .add_guild_member_role(
            Snowflake::from("200"),
            Snowflake::from("201"),
            Snowflake::from("300"),
        )
        .await
        .expect("add guild member role");
    client
        .remove_guild_member_role(
            Snowflake::from("200"),
            Snowflake::from("201"),
            Snowflake::from("300"),
        )
        .await
        .expect("remove guild member role");

    let created_role = client
        .create_role(Snowflake::from("200"), &json!({ "name": "admin" }))
        .await
        .expect("create role");
    assert_eq!(created_role.name, "admin");

    let updated_role = client
        .update_role(
            Snowflake::from("200"),
            Snowflake::from("300"),
            &json!({ "name": "mod" }),
        )
        .await
        .expect("update role");
    assert_eq!(updated_role.name, "mod");

    client
        .delete_role(Snowflake::from("200"), Snowflake::from("300"))
        .await
        .expect("delete role");

    let member = client
        .get_member(Snowflake::from("200"), Snowflake::from("201"))
        .await
        .expect("get member");
    assert!(member.roles.is_empty());

    let roles = client
        .list_roles(Snowflake::from("200"))
        .await
        .expect("list roles");
    assert_eq!(roles[0].name, "mod");

    let overwritten = client
        .bulk_overwrite_global_commands_typed(std::slice::from_ref(&command))
        .await
        .expect("bulk overwrite global commands");
    assert_eq!(overwritten[0].name, "ping");

    let created_command = client
        .create_global_command(&CommandDefinition {
            name: "pong".to_string(),
            description: "reply".to_string(),
            ..command.clone()
        })
        .await
        .expect("create global command");
    assert_eq!(created_command.name, "pong");

    let global_commands = client
        .get_global_commands()
        .await
        .expect("get global commands");
    assert_eq!(global_commands.len(), 1);

    let gateway = client.get_gateway_bot().await.expect("get gateway bot");
    assert_eq!(gateway.shards, 1);

    let guild_commands = client
        .bulk_overwrite_guild_commands_typed(Snowflake::from("200"), &[command])
        .await
        .expect("bulk overwrite guild commands");
    assert_eq!(guild_commands[0].name, "guild");

    server.await.expect("server finished");
    let requests = captured.lock().expect("captured requests");
    let auth = Some("Bot guild-token");

    assert_request_basics(&requests[0], "GET", "/guilds/200", auth);
    assert_request_basics(&requests[1], "PATCH", "/guilds/200", auth);
    assert_request_basics(&requests[2], "GET", "/guilds/200/channels", auth);
    assert_request_basics(&requests[3], "POST", "/guilds/200/channels", auth);
    assert_request_basics(&requests[4], "GET", "/guilds/200/members?limit=3", auth);
    assert_request_basics(&requests[5], "GET", "/guilds/200/members", auth);
    assert_request_basics(&requests[6], "DELETE", "/guilds/200/members/201", auth);
    assert_request_basics(
        &requests[7],
        "PUT",
        "/guilds/200/members/201/roles/300",
        auth,
    );
    assert_request_basics(
        &requests[8],
        "DELETE",
        "/guilds/200/members/201/roles/300",
        auth,
    );
    assert_request_basics(&requests[9], "POST", "/guilds/200/roles", auth);
    assert_request_basics(&requests[10], "PATCH", "/guilds/200/roles/300", auth);
    assert_request_basics(&requests[11], "DELETE", "/guilds/200/roles/300", auth);
    assert_request_basics(&requests[12], "GET", "/guilds/200/members/201", auth);
    assert_request_basics(&requests[13], "GET", "/guilds/200/roles", auth);
    assert_request_basics(&requests[14], "PUT", "/applications/555/commands", auth);
    assert_request_basics(&requests[15], "POST", "/applications/555/commands", auth);
    assert_request_basics(&requests[16], "GET", "/applications/555/commands", auth);
    assert_request_basics(&requests[17], "GET", "/gateway/bot", auth);
    assert_request_basics(
        &requests[18],
        "PUT",
        "/applications/555/guilds/200/commands",
        auth,
    );
}

#[tokio::test]
async fn client_webhook_and_followup_wrappers_hit_local_server() {
    let responses = vec![
        PlannedResponse::json(StatusCode::OK, json!({ "id": "900" })),
        PlannedResponse::json(StatusCode::OK, json!([{ "id": "900" }])),
        PlannedResponse::json(StatusCode::OK, json!({ "unexpected": true })),
        PlannedResponse::json(
            StatusCode::TOO_MANY_REQUESTS,
            json!({ "retry_after": 0.0, "global": false }),
        ),
        PlannedResponse::json(StatusCode::OK, json!({ "id": "901" })),
        PlannedResponse::json(StatusCode::OK, channel_payload("500", 1, None)),
        PlannedResponse::empty(StatusCode::NO_CONTENT),
        PlannedResponse::empty(StatusCode::NO_CONTENT),
        PlannedResponse::json(StatusCode::OK, json!({ "id": "902" })),
        PlannedResponse::json(StatusCode::OK, json!({ "id": "903" })),
        PlannedResponse::json(StatusCode::OK, message_payload("904", "500", "followup")),
        PlannedResponse::json(StatusCode::OK, message_payload("905", "500", "followup")),
        PlannedResponse::json(StatusCode::OK, message_payload("906", "500", "original")),
        PlannedResponse::json(StatusCode::OK, message_payload("907", "500", "original")),
        PlannedResponse::json(StatusCode::OK, message_payload("908", "500", "edited")),
        PlannedResponse::json(StatusCode::OK, message_payload("909", "500", "edited")),
        PlannedResponse::empty(StatusCode::NO_CONTENT),
        PlannedResponse::empty(StatusCode::NO_CONTENT),
        PlannedResponse::json(
            StatusCode::OK,
            message_payload("910", "500", "followup-edit"),
        ),
        PlannedResponse::json(
            StatusCode::OK,
            message_payload("911", "500", "followup-edit"),
        ),
        PlannedResponse::empty(StatusCode::NO_CONTENT),
        PlannedResponse::empty(StatusCode::NO_CONTENT),
    ];
    let (base_url, captured, server) = spawn_test_server(responses).await;
    let client = RestClient::new_with_base_url("hook-token", 123, base_url);
    let body = sample_message();

    let webhook = client
        .create_webhook(Snowflake::from("500"), &json!({ "name": "hook" }))
        .await
        .expect("create webhook");
    assert_eq!(webhook["id"], json!("900"));

    let webhooks = client
        .get_channel_webhooks(Snowflake::from("500"))
        .await
        .expect("get channel webhooks array");
    assert_eq!(webhooks.len(), 1);

    let fallback = client
        .get_channel_webhooks(Snowflake::from("500"))
        .await
        .expect("get channel webhooks fallback");
    assert!(fallback.is_empty());

    let executed = client
        .execute_webhook(
            Snowflake::from("777"),
            "token",
            &json!({ "content": "hook" }),
        )
        .await
        .expect("execute webhook with retry");
    assert_eq!(executed["id"], json!("901"));

    let dm = client
        .create_dm_channel_typed(&crate::model::CreateDmChannel {
            recipient_id: Snowflake::from("42"),
        })
        .await
        .expect("create dm channel");
    assert_eq!(dm.id.as_str(), "500");

    client
        .create_interaction_response_typed(
            Snowflake::from("777"),
            "token",
            &sample_interaction_response(),
        )
        .await
        .expect("create interaction response typed");
    client
        .create_interaction_response_json(
            Snowflake::from("778"),
            "token",
            &json!({ "type": 4, "data": { "content": "json" } }),
        )
        .await
        .expect("create interaction response json");

    assert_eq!(
        client
            .create_followup_message_json_with_application_id(
                "123",
                "token",
                &json!({ "content": "json" }),
            )
            .await
            .expect("explicit followup json")["id"],
        json!("902")
    );
    assert_eq!(
        client
            .create_followup_message_json("token", &json!({ "content": "implicit" }))
            .await
            .expect("implicit followup json")["id"],
        json!("903")
    );
    assert_eq!(
        client
            .create_followup_message_with_application_id("123", "token", &body)
            .await
            .expect("explicit followup message")
            .content,
        "followup"
    );
    assert_eq!(
        client
            .create_followup_message("token", &body)
            .await
            .expect("implicit followup message")
            .content,
        "followup"
    );
    assert_eq!(
        client
            .get_original_interaction_response_with_application_id("123", "token")
            .await
            .expect("explicit original get")
            .content,
        "original"
    );
    assert_eq!(
        client
            .get_original_interaction_response("token")
            .await
            .expect("implicit original get")
            .content,
        "original"
    );
    assert_eq!(
        client
            .edit_original_interaction_response_with_application_id("123", "token", &body)
            .await
            .expect("explicit original edit")
            .content,
        "edited"
    );
    assert_eq!(
        client
            .edit_original_interaction_response("token", &body)
            .await
            .expect("implicit original edit")
            .content,
        "edited"
    );
    client
        .delete_original_interaction_response_with_application_id("123", "token")
        .await
        .expect("explicit original delete");
    client
        .delete_original_interaction_response("token")
        .await
        .expect("implicit original delete");

    assert_eq!(
        client
            .edit_followup_message_with_application_id("123", "token", "55", &body)
            .await
            .expect("explicit followup edit")
            .content,
        "followup-edit"
    );
    assert_eq!(
        client
            .edit_followup_message("token", "55", &body)
            .await
            .expect("implicit followup edit")
            .content,
        "followup-edit"
    );
    client
        .delete_followup_message_with_application_id("123", "token", "55")
        .await
        .expect("explicit followup delete");
    client
        .delete_followup_message("token", "55")
        .await
        .expect("implicit followup delete");

    server.await.expect("server finished");
    let requests = captured.lock().expect("captured requests");
    let bot_auth = Some("Bot hook-token");

    assert_request_basics(&requests[0], "POST", "/channels/500/webhooks", bot_auth);
    assert_request_basics(&requests[1], "GET", "/channels/500/webhooks", bot_auth);
    assert_request_basics(&requests[2], "GET", "/channels/500/webhooks", bot_auth);
    assert_request_basics(&requests[3], "POST", "/webhooks/777/token?wait=true", None);
    assert_request_basics(&requests[4], "POST", "/webhooks/777/token?wait=true", None);
    assert_request_basics(&requests[5], "POST", "/users/@me/channels", bot_auth);
    assert_request_basics(
        &requests[6],
        "POST",
        "/interactions/777/token/callback",
        None,
    );
    assert_request_basics(
        &requests[7],
        "POST",
        "/interactions/778/token/callback",
        None,
    );
    assert_request_basics(&requests[8], "POST", "/webhooks/123/token", None);
    assert_request_basics(&requests[9], "POST", "/webhooks/123/token", None);
    assert_request_basics(&requests[10], "POST", "/webhooks/123/token", None);
    assert_request_basics(&requests[11], "POST", "/webhooks/123/token", None);
    assert_request_basics(
        &requests[12],
        "GET",
        "/webhooks/123/token/messages/@original",
        None,
    );
    assert_request_basics(
        &requests[13],
        "GET",
        "/webhooks/123/token/messages/@original",
        None,
    );
    assert_request_basics(
        &requests[14],
        "PATCH",
        "/webhooks/123/token/messages/@original",
        None,
    );
    assert_request_basics(
        &requests[15],
        "PATCH",
        "/webhooks/123/token/messages/@original",
        None,
    );
    assert_request_basics(
        &requests[16],
        "DELETE",
        "/webhooks/123/token/messages/@original",
        None,
    );
    assert_request_basics(
        &requests[17],
        "DELETE",
        "/webhooks/123/token/messages/@original",
        None,
    );
    assert_request_basics(
        &requests[18],
        "PATCH",
        "/webhooks/123/token/messages/55",
        None,
    );
    assert_request_basics(
        &requests[19],
        "PATCH",
        "/webhooks/123/token/messages/55",
        None,
    );
    assert_request_basics(
        &requests[20],
        "DELETE",
        "/webhooks/123/token/messages/55",
        None,
    );
    assert_request_basics(
        &requests[21],
        "DELETE",
        "/webhooks/123/token/messages/55",
        None,
    );
}

#[tokio::test]
async fn request_surfaces_api_and_rate_limit_errors_from_local_server() {
    let responses = vec![
        PlannedResponse::text(
            StatusCode::BAD_REQUEST,
            r#"{"code":50035,"message":"bad payload"}"#,
        ),
        PlannedResponse::json(
            StatusCode::TOO_MANY_REQUESTS,
            json!({ "retry_after": 0.0, "global": false }),
        ),
        PlannedResponse::json(
            StatusCode::TOO_MANY_REQUESTS,
            json!({ "retry_after": 0.0, "global": false }),
        ),
    ];
    let (base_url, captured, server) = spawn_test_server(responses).await;
    let client = RestClient::new_with_base_url("err-token", 123, base_url);

    match client
        .request(
            Method::POST,
            "/channels/9/messages",
            Some(&json!({ "content": "boom" })),
        )
        .await
        .unwrap_err()
    {
        DiscordError::Api {
            status,
            code,
            message,
        } => {
            assert_eq!(status, 400);
            assert_eq!(code, Some(50035));
            assert_eq!(message, "bad payload");
        }
        other => panic!("unexpected api error: {other:?}"),
    }

    match client
        .execute_webhook(Snowflake::from("9"), "token", &json!({ "content": "boom" }))
        .await
        .unwrap_err()
    {
        DiscordError::RateLimit { route, retry_after } => {
            assert_eq!(route, "POST:webhooks/9/token");
            assert_eq!(retry_after, 0.0);
        }
        other => panic!("unexpected rate limit error: {other:?}"),
    }

    server.await.expect("server finished");
    let requests = captured.lock().expect("captured requests");
    assert_eq!(requests.len(), 3);
    assert_request_basics(
        &requests[0],
        "POST",
        "/channels/9/messages",
        Some("Bot err-token"),
    );
    assert_request_basics(&requests[1], "POST", "/webhooks/9/token?wait=true", None);
    assert_request_basics(&requests[2], "POST", "/webhooks/9/token?wait=true", None);
}
