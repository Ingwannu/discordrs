# 게이트웨이 API

`gateway` 기능 플래그 활성화 시 WebSocket 런타임을 사용합니다.

## 핵심 타입

- `GatewayClient`: identify/heartbeat/resume/reconnect 처리
- `BotClient`: 고수준 런타임 래퍼
- `Context`: 핸들러 공유 컨텍스트(`http`, typemap)
- `EventHandler`: 이벤트 콜백 트레잇

## 기본 실행

```rust
BotClient::builder(&token, gateway_intents::GUILDS | gateway_intents::GUILD_MESSAGES)
    .event_handler(handler)
    .start()
    .await?;
```
