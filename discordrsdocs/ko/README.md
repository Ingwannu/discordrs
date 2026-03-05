# discordrs 문서

`discordrs`로 Discord 봇을 만들기 위한 탐색형 문서 사이트입니다.

> `discordrs`는 Gateway 런타임, Interaction Endpoint, HTTP 클라이언트, Components V2 빌더, 파서를 제공하는 독립형 Rust 프레임워크입니다.

## 언어 전환

- [English](#/README)
- [한국어](#/ko/README)

## 먼저 보기

- [빠른 시작](#/ko/guide/getting-started)
- [사용 가이드](#/ko/guide/usage-guide)
- [아키텍처](#/ko/guide/architecture)
- [인터랙션 흐름](#/ko/guide/interaction-flow)
- [전체 매뉴얼 (Markdown)](#/ko/guide/full-manual)
- [전체 매뉴얼 (PDF)](#/ko/guide/pdf-manual)

## 설치

```toml
[dependencies]
# 코어만 사용
discordrs = "0.3.0"

# Gateway 런타임
discordrs = { version = "0.3.0", features = ["gateway"] }

# Interactions Endpoint
discordrs = { version = "0.3.0", features = ["interactions"] }

# 둘 다 사용
discordrs = { version = "0.3.0", features = ["gateway", "interactions"] }
```

## 로컬 미리보기

```bash
python3 -m http.server 8080 --directory discordrsdocs
```

브라우저에서 <http://localhost:8080> 을 열면 됩니다.
