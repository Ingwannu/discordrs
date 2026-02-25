# discordrs 사용법

`discordrs`는 serenity 기반 봇에서 **Discord Components V2**를 쉽게 쓰기 위한 라이브러리입니다.

## 1) 설치

`Cargo.toml`:

```toml
[dependencies]
discordrs = "0.1.3"
serenity = { version = "0.12.5", features = ["client", "gateway", "model", "http", "rustls_backend"] }
```

## 2) 채널에 컨테이너 메시지 보내기

```rust
use discordrs::{button_style, create_container, send_container_message, ButtonConfig};
use serenity::all::ChannelId;
use serenity::http::Http;

async fn send_panel(http: &Http, channel_id: ChannelId) -> Result<(), discordrs::Error> {
    let buttons = vec![
        ButtonConfig::new("ticket_open", "티켓 열기")
            .style(button_style::PRIMARY)
            .emoji("🎫"),
    ];

    let container = create_container(
        "고객지원 패널",
        "아래 버튼으로 문의 티켓을 생성하세요.",
        buttons,
        None,
    );

    send_container_message(http, channel_id, container).await?;
    Ok(())
}
```

## 3) Slash Command 응답 (ephemeral)

```rust
use discordrs::{respond_with_container, create_container};
use serenity::all::CommandInteraction;
use serenity::http::Http;

async fn respond_cmd(http: &Http, interaction: &CommandInteraction) -> Result<(), discordrs::Error> {
    let container = create_container("알림", "설정이 완료되었습니다.", vec![], None);
    respond_with_container(http, interaction, container, true).await
}
```

## 4) 버튼/셀렉트(Component) 응답

```rust
use discordrs::{respond_component_with_container, create_container};
use serenity::all::ComponentInteraction;
use serenity::http::Http;

async fn respond_component(http: &Http, interaction: &ComponentInteraction) -> Result<(), discordrs::Error> {
    let container = create_container("처리 결과", "선택값이 저장되었습니다.", vec![], None);
    respond_component_with_container(http, interaction, container, true).await
}
```

## 5) 모달 제출 응답

```rust
use discordrs::{respond_modal_with_container, create_container};
use serenity::all::ModalInteraction;
use serenity::http::Http;

async fn respond_modal(http: &Http, interaction: &ModalInteraction) -> Result<(), discordrs::Error> {
    let container = create_container("완료", "모달 입력이 반영되었습니다.", vec![], None);
    respond_modal_with_container(http, interaction, container, true).await
}
```

## 6) 자주 쓰는 API

- `create_container(...)`: 제목/설명/버튼/이미지로 표준 컨테이너 생성
- `send_container_message(...)`: 채널 전송
- `respond_with_container(...)`: 슬래시 커맨드 응답
- `respond_component_with_container(...)`: 버튼/셀렉트 응답
- `respond_modal_with_container(...)`: 모달 제출 응답
- `followup_with_container(...)`: defer 이후 후속 응답
- `respond_with_modal(...)`: raw 모달 응답

## 7) 모달 Radio/Checkbox 컴포넌트

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

## 8) Slash Command 등록

```rust
use discordrs::{
    slash_commands, CommandOptionBuilder, CommandOptionChoice, SlashCommandBuilder,
    SlashCommandScope, SlashCommandSet,
};
use serenity::all::GuildId;
use serenity::http::Http;

async fn register(http: &Http, guild_id: GuildId) -> Result<(), discordrs::Error> {
    let mut commands = slash_commands![
        SlashCommandBuilder::new("ping", "지연 시간 확인")
            .dm_permission(false)
            .add_option(
                CommandOptionBuilder::string("target", "대상")
                    .required(true)
                    .add_choice(CommandOptionChoice::string("전체", "all")),
            ),
        SlashCommandBuilder::new("about", "봇 정보"),
    ];

    // 이름 기반 upsert/remove
    commands.set_command(SlashCommandBuilder::new("ping", "업데이트된 지연 시간 확인"));
    let _ = commands.remove("about");
    assert!(commands.contains("ping"));

    // payload 확인 (set을 소모하지 않음)
    let payload = commands.payload();
    assert_eq!(payload.len(), 1);

    // 통합 scope API
    let _ = commands.register_ref(http, SlashCommandScope::Global).await?;
    let _ = commands
        .register_ref(http, SlashCommandScope::Guild(guild_id))
        .await?;

    // 여러 scope를 한 번에 등록
    let registered = commands
        .register_many_ref(
            http,
            [SlashCommandScope::Global, SlashCommandScope::Guild(guild_id)],
        )
        .await?;
    assert_eq!(registered.len(), 2);
    Ok(())
}
```

## 9) Interaction 디스패치 헬퍼

```rust
use discordrs::{dispatch_interaction, dispatch_interaction_match, InteractionRouter};

let mut router = InteractionRouter::new();
router.insert_command("ping", "handle_ping");
router.insert_component_prefix("ticket:", "handle_ticket_component");
router.insert_modal_prefix("ticket_modal:", "handle_ticket_modal");
router.set_component_fallback("handle_component_fallback");

// event loop 내부
// if let Some(route) = router.resolve_interaction(&interaction) {
//     match *route {
//         "handle_ping" => { /* ... */ }
//         "handle_ticket_component" => { /* ... */ }
//         "handle_ticket_modal" => { /* ... */ }
//         _ => {}
//     }
// }
// if let Some(m) = router.resolve_interaction_match(&interaction) {
//     println!("matched {:?} by key {}", m.kind, m.key);
// }
// assert!(router.contains_command("ping"));
// router.set_component_prefix("ticket:", "new_ticket_component_handler");
// router.remove_modal("ticket_modal:legacy");
// dispatch_interaction(&router, &interaction) / dispatch_interaction_match(...)도 계속 사용 가능
```

라우팅 규칙:
- exact(custom_id/command name) 우선
- exact 미스 시 prefix 매칭
- prefix가 여러 개면 **가장 긴 prefix** 우선
- 동일 우선순위(같은 exact 키 / 같은 prefix 길이)면 **나중에 insert된 라우트** 우선
- 매칭 실패 시 타입별 fallback(`set_*_fallback`)이 있으면 fallback 사용
- `contains_command` / `contains_component` / `contains_modal`은 **등록된 라우트만** 검사 (fallback은 포함하지 않음)
- `set_*`은 upsert, `insert_*`는 추가, `remove_*`는 삭제
- 타입별 헬퍼: `resolve_command`, `resolve_component`, `resolve_modal`
- 공통 fallback 헬퍼: `set_fallback(kind, ...)`, `remove_fallback(kind)`, `has_fallback(kind)`
- 공통 exact 라우트 헬퍼: `insert(kind, key, ...)`, `set(kind, key, ...)`, `remove(kind, key)`, `contains(kind, key)`
- 공통 prefix 라우트 헬퍼: `insert_prefix(kind, ...)`, `set_prefix(kind, ...)`, `remove_prefix(kind, ...)`
- 타입별 라우트 정리 헬퍼: `len_for(kind)`, `has_routes_for(kind)`, `clear_kind(kind)`

`SlashCommandSet` 추가 유틸:
- `slash_commands![ ... ]`: 간결한 명령어 세트 구성
- `names()` / `iter()` / `iter_mut()`: 삽입 순서 순회/수정
- `get("name")` / `get_mut("name")`: 이름으로 조회/수정
- `retain(...)`: 등록 전 조건에 맞는 명령어만 유지
- `with_set_commands(...)` / `set_commands(...)`: 이름 기반 bulk upsert
- `merge(...)` / `with_merged(...)`: 다른 Set을 이름 기반으로 병합
- `dedup_by_name()`: 중복 이름 제거(가장 마지막에 정의된 명령어 유지)
- `without("name")`, `remove_where(...)`: 등록 전 정리 작업을 간결하게 처리
- `register_many_ref(...)` / `register_many(...)`: 동일 명령어 세트를 여러 scope로 일괄 등록

## 10) 참고

- `discordrs`는 serenity가 아직 완전 지원하지 않는 V2 컴포넌트를 **raw HTTP payload**로 전송합니다.
- 버튼/셀렉트의 `custom_id`는 핸들러 라우팅 규칙과 반드시 일치시켜야 합니다.
