# 인터랙션 흐름

## 1. 컨텍스트/타입 파싱

```rust
let ctx = parse_interaction_context(payload)?;
let raw = parse_raw_interaction(payload)?;
```

## 2. 타입별 라우팅

```rust
match raw {
    RawInteraction::Command { name, .. } => { /* slash */ }
    RawInteraction::Component { custom_id, .. } => { /* button/select */ }
    RawInteraction::ModalSubmit(submission) => { /* modal */ }
    RawInteraction::Ping => { /* endpoint pong */ }
}
```

## 3. 헬퍼로 응답

- `respond_with_container(...)`: Slash Command
- `respond_component_with_container(...)`: 버튼/셀렉트
- `respond_modal_with_container(...)`: 모달 제출
- `respond_with_modal(...)`: 모달 열기

## 4. Ephemeral 처리

응답 헬퍼의 `ephemeral` 플래그로 공개/비공개 응답을 분기합니다.
