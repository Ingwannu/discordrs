# Deployment (GitHub Pages)

This docs site is static and buildless (Docsify), so deployment is simple.

## Option A: GitHub Pages from Actions (Recommended)

1. Push the `docs-pages` workflow.
2. In GitHub: `Settings > Pages`.
3. Set `Build and deployment` to `GitHub Actions`.
4. The workflow will publish `discordrsdocs/` on pushes to `main`.

## Option B: Branch Folder Source

1. In `Settings > Pages`, set source to `Deploy from a branch`.
2. Branch: `main`, folder: `/discordrsdocs`.
3. Save settings.

## Local Preview

```bash
python3 -m http.server 8080 --directory discordrsdocs
```

Open <http://localhost:8080>.
