# HTTP and Helpers API

## `RestClient`

`RestClient` is the primary Discord REST v10 surface. It keeps shared route/global rate-limit state and also keeps `DiscordHttpClient` as a compatibility alias.

Common operations include:

- typed message create/update/get
- typed guild/channel/member/role lookups
- typed application command overwrite
- typed Auto Moderation rule reads and writes through `get_auto_moderation_rules_typed`, `create_auto_moderation_rule_typed`, and `modify_auto_moderation_rule_typed`
- typed guild administration helpers for bulk bans, single-role lookup, guild preview, prune count/result, vanity URL, and voice regions
- typed emoji helpers for guild and application emoji reads/writes
- typed scheduled-event recurrence/entity metadata, plus typed create/modify request helpers
- typed current-application and application role-connection metadata helpers
- typed current-user guild list, current-member modify, webhook-by-token lookup, and guild-specific voice region helpers
- interaction responses and follow-up webhook helpers

Raw `serde_json::Value` methods remain available for routes where Discord adds fields before discord.rs has a typed model. Prefer the typed methods first, then drop to raw JSON only for newly released or experimental API fields.

## Helper Functions

For Components V2 and interaction response workflows, use:

- `send_container_message(...)`
- `respond_with_container(...)`
- `respond_component_with_container(...)`
- `respond_modal_with_container(...)`
- `respond_with_modal(...)`

## Example

```rust
let c = create_container("Notice", "Done", vec![], None);
respond_with_container(http, &ctx.id, &ctx.token, c, true).await?;
```

## OAuth2 Backend Helpers

`OAuth2Client` is the application-backend OAuth2 surface. It builds authorization URLs and exchanges authorization codes or refresh tokens with form-encoded OAuth2 requests. It is intentionally separate from bot-token `RestClient` calls.

```rust
use discordrs::{OAuth2AuthorizationRequest, OAuth2Client, OAuth2CodeExchange, OAuth2Scope};

let oauth = OAuth2Client::new("client_id", "client_secret");
let url = oauth.authorization_url(
    OAuth2AuthorizationRequest::code(
        "https://app.example/callback",
        [OAuth2Scope::identify(), OAuth2Scope::guilds()],
    )
    .state("csrf-token"),
)?;

let token = oauth
    .exchange_code(OAuth2CodeExchange::new("code", "https://app.example/callback"))
    .await?;
```

## Recommended Pattern

- Use `RestClient` or context managers for typed fetch/send flows.
- Use `OAuth2Client` for OAuth2 application backend flows.
- Use helper functions for common interaction acknowledgment paths.
- Keep Components V2 payload generation inside the builders.
- Fall back to low-level raw request helpers only when the typed surface does not yet cover the route.
