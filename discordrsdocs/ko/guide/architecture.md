# 아키텍처

`discordrs`는 역할별 모듈 분리 구조를 가집니다.

```mermaid
flowchart LR
  subgraph App[애플리케이션]
    H[EventHandler]
    IH[InteractionHandler]
  end

  subgraph Runtime[Gateway 런타임]
    BC[BotClient]
    GC[GatewayClient]
  end

  subgraph Transport[HTTP 레이어]
    HC[DiscordHttpClient]
    HP[Helpers]
  end

  subgraph Build[빌더]
    BV2[Components V2 Builders]
    MB[Modal Builders]
  end

  subgraph Parse[파서]
    IP[Interaction Parser]
    MP[V2 Modal Parser]
  end

  subgraph Endpoint[Interactions Endpoint]
    AX[Axum Router + Ed25519 Verify]
  end

  H --> BC --> GC
  BC --> HC
  IH --> AX --> HP
  HP --> HC
  HP --> BV2
  IP --> MP
  AX --> IP
```

## 모듈 구성

- `src/builders/`: Components V2, Modal 페이로드 빌더
- `src/gateway/`: WebSocket 런타임, heartbeat/resume/reconnect
- `src/http.rs`: REST 클라이언트 및 429 재시도
- `src/parsers/`: 인터랙션/모달 파싱
- `src/helpers.rs`: 응답 헬퍼
- `src/interactions.rs`: HTTP 엔드포인트 모드
