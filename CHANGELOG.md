# Changelog

## Unreleased

- Added `SlashCommandScope` and `register_slash_commands(...)` for unified global/guild slash registration.
- Extended `SlashCommandSet` ergonomics with scope-based `register[_ref]`, name-based upsert/remove (`set_command`, `with_set_command`, `remove`, `contains`), plus standard `Extend`/`IntoIterator` support.
- Improved `InteractionRouter` robustness with per-kind fallback handlers (`*_fallback`).
- Expanded tests and docs for the updated slash registration and interaction routing APIs.

## 0.1.3

- Added modal interaction components:
  - `RadioGroupBuilder` for single-choice selection.
  - `CheckboxGroupBuilder` for multi-choice selection.
  - `CheckboxBuilder` for yes/no style toggles.
- Updated package version to `0.1.3` in `Cargo.toml`.
