# 배포 (GitHub Pages)

이 문서 사이트는 Docsify 기반 정적 사이트라서 빌드 없이 배포됩니다.

## 방법 A: GitHub Actions (권장)

1. `docs-pages` 워크플로우를 main에 푸시
2. GitHub `Settings > Pages`
3. `Build and deployment`를 `GitHub Actions`로 설정
4. main 푸시 시 `discordrsdocs/`가 자동 배포

## 방법 B: Branch 폴더 직접 배포

1. `Settings > Pages`
2. `Deploy from a branch`
3. Branch: `main`, Folder: `/discordrsdocs`

## 서비스 URL

- <http://discordrs.teamwicked.me/discordrsdocs/#/>
- 한국어 홈: <http://discordrs.teamwicked.me/discordrsdocs/#/ko/README>
