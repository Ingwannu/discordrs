# 사용 가이드

이 문서는 `discordrs`의 실전 사용 흐름을 요약합니다.

## 1. 기능 플래그 선택

- `gateway`: WebSocket Gateway 런타임
- `interactions`: HTTP Interactions Endpoint
- 둘 다 활성화해서 하이브리드 운영 가능

## 2. 기본 패턴

1. Gateway 또는 Endpoint로 이벤트 수신
2. `parse_raw_interaction`, `parse_interaction_context`로 파싱
3. `create_container` 등 빌더로 응답 페이로드 생성
4. `respond_with_container` 계열 헬퍼로 응답

## 3. Slash Command 응답 예시

```rust
let ctx = parse_interaction_context(payload)?;
if let RawInteraction::Command { name, .. } = parse_raw_interaction(payload)? {
    if name.as_deref() == Some("hello") {
        let container = create_container("알림", "명령이 처리되었습니다.", vec![], None);
        respond_with_container(http, &ctx.id, &ctx.token, container, true).await?;
    }
}
```

## 4. Modal 제출 처리

`RawInteraction::ModalSubmit`에서 `V2ModalSubmission`을 받아 Radio/Checkbox 값을 그대로 읽을 수 있습니다.

## 5. 참고

영문 원문 예제 전체는 [English Usage Guide](#/docs/guide/usage-guide)에서 확인할 수 있습니다.
