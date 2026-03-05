# discordrs Documentation

A practical docs site for building Discord bots with `discordrs`.

> `discordrs` is a standalone Rust framework with Gateway runtime, Interaction Endpoint support, HTTP client, Components V2 builders, and robust payload parsers.

## Start Here

- [Getting Started](docs/guide/getting-started.md)
- [Usage Guide](docs/guide/usage-guide.md)
- [Architecture](docs/guide/architecture.md)
- [Interaction Flow](docs/guide/interaction-flow.md)
- [Full Technical Manual (Markdown)](docs/guide/full-manual.md)
- [Full Technical Manual (PDF)](docs/guide/pdf-manual.md)

## Install

```toml
[dependencies]
# core only
discordrs = "0.3.0"

# gateway runtime
discordrs = { version = "0.3.0", features = ["gateway"] }

# interactions endpoint
discordrs = { version = "0.3.0", features = ["interactions"] }

# both
discordrs = { version = "0.3.0", features = ["gateway", "interactions"] }
```

## What You Can Build

- Event-driven Gateway bots (`READY`, `MESSAGE_CREATE`, `INTERACTION_CREATE`)
- Slash command + component + modal workflows
- Components V2 rich layouts with sections, media galleries, buttons, and selects
- Verified `/interactions` HTTP endpoint (Ed25519)

## Local Preview

```bash
python3 -m http.server 8080 --directory discordrsdocs
```

Then open <http://localhost:8080>.
