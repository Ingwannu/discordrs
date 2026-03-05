# 빠른 시작

## 준비물

- Rust stable toolchain
- Discord 애플리케이션 및 봇 토큰
- (선택) Interactions Endpoint 모드용 공개 HTTP 엔드포인트

## 의존성 추가

```toml
[dependencies]
discordrs = { version = "0.3.0", features = ["gateway"] }
```

## 최소 Gateway 봇 예제

```rust
use async_trait::async_trait;
use discordrs::{gateway_intents, BotClient, Context, EventHandler};
use serde_json::Value;

struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn ready(&self, _ctx: Context, ready: Value) {
        println!("READY as {}", ready["user"]["username"]);
    }
}

#[tokio::main]
async fn main() -> Result<(), discordrs::Error> {
    let token = std::env::var("DISCORD_TOKEN")?;

    BotClient::builder(&token, gateway_intents::GUILDS | gateway_intents::GUILD_MESSAGES)
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

- [사용 가이드](#/ko/guide/usage-guide)
- [API 레퍼런스](#/ko/api/builders)
