# 파서 API

원본 JSON payload를 타입 구조체로 안전하게 변환합니다.

## 인터랙션 파서

- `parse_raw_interaction(&Value)`
- `parse_interaction_context(&Value)`

## 모달 파서

- `parse_modal_submission(&Value)`

`V2ModalSubmission`은 `Label`, `RadioGroup`, `CheckboxGroup`, `Checkbox` 타입을 보존합니다.
