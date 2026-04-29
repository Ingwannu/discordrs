# 빠른 시작

## 준비물

- Rust stable toolchain
- Discord 애플리케이션과 봇 토큰
- 선택 사항: Interaction Endpoint 모드에서 사용할 공개 HTTP 엔드포인트

## 의존성 추가

```toml
[dependencies]
discordrs = { version = "1.1.0", features = ["gateway"] }
```

필요한 런타임에 따라 기능을 추가합니다.

```toml
# REST/빌더/타입 모델만 사용할 때
discordrs = "1.1.0"

# Voice receive와 Opus PCM decode
discordrs = { version = "1.1.0", features = ["voice"] }

# 실험적 DAVE/MLS hook
discordrs = { version = "1.1.0", features = ["voice", "dave"] }
```

## 최소 Typed Gateway Bot

```rust
use async_trait::async_trait;
use discordrs::{gateway_intents, Client, Context, Event, EventHandler};

struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn handle_event(&self, _ctx: Context, event: Event) {
        if let Event::Ready(ready) = event {
            println!("READY as {}", ready.data.user.username);
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), discordrs::DiscordError> {
    let token = std::env::var("DISCORD_TOKEN")?;

    Client::builder(&token, gateway_intents::GUILDS | gateway_intents::GUILD_MESSAGES)
        .event_handler(Handler)
        .start()
        .await?;

    Ok(())
}
```

## 환경 변수

```bash
export DISCORD_TOKEN="your-bot-token"
```

## 실행

```bash
cargo run
```

## 다음 단계

- [사용 가이드](usage-guide.md)로 이동
- [아키텍처](architecture.md) 읽기
- [커맨드 API](../api/commands.md) 살펴보기
- Poll, Subscription, Soundboard, Thread, Forum, Integration, Voice receive 같은 확장 표면은 먼저 타입 API를 확인한 뒤 raw JSON은 마지막 수단으로 사용하기
