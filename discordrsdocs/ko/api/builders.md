# 빌더 API

`builders` 모듈은 Components V2/Modal 페이로드를 플루언트 스타일로 구성합니다.

## 하위 모듈

- `components.rs`: `ButtonBuilder`, `SelectMenuBuilder`, `ActionRowBuilder`
- `container.rs`: 컨테이너 레이아웃 + 편의 헬퍼
- `media.rs`: 섹션/썸네일/미디어 갤러리
- `modal.rs`: 텍스트 입력, 라디오, 체크박스, 파일 업로드

## 예시

```rust
let button = ButtonBuilder::new()
    .label("Open")
    .style(button_style::PRIMARY)
    .custom_id("open_ticket");
```
