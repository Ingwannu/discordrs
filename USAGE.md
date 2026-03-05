# discordrs 사용법

`discordrs`는 Components V2, Gateway WebSocket, HTTP 클라이언트를 함께 제공하는 독립형 Discord 봇 프레임워크입니다.

## 1. 설치

`gateway` 기능으로 봇 런타임을 쓰는 기본 설정은 아래와 같습니다.

```toml
[dependencies]
discordrs = { version = "0.3.0", features = ["gateway"] }
```

필요에 따라 기능 플래그를 선택할 수 있습니다.

```toml
[dependencies]
# 기본 기능만 사용 (빌더, 파서, HTTP 클라이언트, 헬퍼)
discordrs = "0.3.0"

# Gateway + 봇 클라이언트
discordrs = { version = "0.3.0", features = ["gateway"] }

# Interactions Endpoint
discordrs = { version = "0.3.0", features = ["interactions"] }

# 둘 다 사용
discordrs = { version = "0.3.0", features = ["gateway", "interactions"] }
```

## 2. 봇 시작하기

```rust
use async_trait::async_trait;
use discordrs::{gateway_intents, BotClient, Context, EventHandler};
use serde_json::Value;

struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn ready(&self, _ctx: Context, ready: Value) {
        println!("READY: {}", ready["user"]["username"]);
    }

    async fn message_create(&self, _ctx: Context, message: Value) {
        println!("MESSAGE_CREATE: {}", message["id"]);
    }
}

#[tokio::main]
async fn main() -> Result<(), discordrs::Error> {
    let token = std::env::var("DISCORD_TOKEN")?;

    BotClient::builder(
        &token,
        gateway_intents::GUILDS | gateway_intents::GUILD_MESSAGES,
    )
    .event_handler(Handler)
    .start()
    .await?;

    Ok(())
}
```

## 3. 채널에 컨테이너 메시지 보내기

```rust
use discordrs::{button_style, create_container, send_container_message, ButtonConfig, DiscordHttpClient};

async fn send_panel(http: &DiscordHttpClient, channel_id: u64) -> Result<(), discordrs::Error> {
    let buttons = vec![
        ButtonConfig::new("ticket_open", "티켓 열기").style(button_style::PRIMARY),
        ButtonConfig::new("ticket_status", "진행 상태").style(button_style::SECONDARY),
    ];

    let container = create_container(
        "고객지원 패널",
        "아래 버튼으로 문의를 접수하거나 상태를 확인하세요.",
        buttons,
        None,
    );

    send_container_message(http, channel_id, container).await?;
    Ok(())
}
```

## 4. Slash Command 응답

`InteractionContext`를 사용하면 `interaction_id`, `token`, `application_id`를 바로 꺼내서 응답할 수 있습니다.

```rust
use discordrs::{
    create_container, parse_interaction_context, parse_raw_interaction,
    respond_with_container, DiscordHttpClient, RawInteraction,
};
use serde_json::Value;

async fn handle_slash(http: &DiscordHttpClient, payload: &Value) -> Result<(), discordrs::Error> {
    let ctx = parse_interaction_context(payload)?;

    if let RawInteraction::Command { name, .. } = parse_raw_interaction(payload)? {
        if name.as_deref() == Some("hello") {
            let container = create_container("알림", "명령이 처리되었습니다.", vec![], None);
            respond_with_container(http, &ctx.id, &ctx.token, container, true).await?;
        }
    }

    Ok(())
}
```

## 5. 버튼/셀렉트 응답

```rust
use discordrs::{
    create_container, parse_interaction_context, respond_component_with_container,
    DiscordHttpClient,
};
use serde_json::Value;

async fn handle_component(http: &DiscordHttpClient, payload: &Value) -> Result<(), discordrs::Error> {
    let ctx = parse_interaction_context(payload)?;
    let container = create_container("처리 결과", "선택한 값을 저장했습니다.", vec![], None);

    respond_component_with_container(http, &ctx.id, &ctx.token, container, true).await?;
    Ok(())
}
```

## 6. 모달 제출 응답

`RawInteraction::ModalSubmit`에서 `V2ModalSubmission`을 받아 Radio/Checkbox 값을 그대로 읽을 수 있습니다.

```rust
use discordrs::{
    create_container, parse_interaction_context, parse_raw_interaction,
    respond_modal_with_container, DiscordHttpClient, RawInteraction, V2ModalSubmission,
};
use serde_json::Value;

fn summarize(submission: &V2ModalSubmission) -> String {
    let theme = submission.get_radio_value("theme").unwrap_or("미선택");
    let channels = submission
        .get_select_values("notify_channels")
        .map(|v| v.join(", "))
        .unwrap_or_else(|| "없음".to_string());

    format!("테마: {theme}, 알림: {channels}")
}

async fn handle_modal(http: &DiscordHttpClient, payload: &Value) -> Result<(), discordrs::Error> {
    let ctx = parse_interaction_context(payload)?;

    if let RawInteraction::ModalSubmit(submission) = parse_raw_interaction(payload)? {
        let result = summarize(&submission);
        let container = create_container("모달 처리 완료", &result, vec![], None);
        respond_modal_with_container(http, &ctx.id, &ctx.token, container, true).await?;
    }

    Ok(())
}
```

## 7. 자주 쓰는 API

- `DiscordHttpClient::new(token, application_id)`: REST 클라이언트 생성
- `create_container(...)`: 기본 컨테이너 메시지 구성
- `send_container_message(...)`: 채널에 Components V2 메시지 전송
- `respond_with_container(...)`: Slash Command 응답
- `respond_component_with_container(...)`: 버튼, 셀렉트 응답
- `respond_modal_with_container(...)`: 모달 제출 응답
- `respond_with_modal(...)`: 모달 열기 응답
- `parse_raw_interaction(...)`: 인터랙션 타입 라우팅
- `parse_interaction_context(...)`: 응답에 필요한 공통 컨텍스트 추출
- `parse_modal_submission(...)`: V2 모달 데이터 파싱

## 8. 모달 Radio/Checkbox

```rust
use discordrs::{
    CheckboxBuilder, CheckboxGroupBuilder, ModalBuilder, RadioGroupBuilder, SelectOption,
};

let modal = ModalBuilder::new("preferences_modal", "Preferences")
    .add_radio_group(
        "테마",
        Some("하나만 선택"),
        RadioGroupBuilder::new("theme")
            .add_option(SelectOption::new("라이트", "light"))
            .add_option(SelectOption::new("다크", "dark"))
            .required(true),
    )
    .add_checkbox_group(
        "알림 채널",
        Some("여러 개 선택 가능"),
        CheckboxGroupBuilder::new("notify_channels")
            .add_option(SelectOption::new("이메일", "email"))
            .add_option(SelectOption::new("푸시", "push"))
            .min_values(0)
            .max_values(2),
    )
    .add_checkbox(
        "약관 동의",
        None,
        CheckboxBuilder::new("agree_terms").required(true),
    );
```

## 9. 참고

- v0.3.0부터 `discordrs`는 독립형 프레임워크로 동작하며, Gateway와 HTTP를 함께 제공합니다.
- V2 모달 파서는 `Label`, `RadioGroup`, `CheckboxGroup`, `Checkbox` 같은 컴포넌트를 유지해서 후처리 로직 작성이 쉽습니다.
- 인터랙션 응답 함수는 `InteractionContext`의 `id`와 `token`을 그대로 사용하면 됩니다.
