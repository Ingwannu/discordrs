# Core Types and Constants

## `src/types.rs`

Core utility and data types include:

- `Error` alias + `invalid_data_error`
- `ButtonConfig`
- `Emoji`
- `SelectOption`
- `MediaGalleryItem`
- `MediaInfo`

These types are used by builders, helpers, and parsers.

## `TypeMap`

`TypeMap` allows shared state storage in runtime context for gateway handlers.

Use this for cross-handler app state that does not fit static globals.

## `src/constants.rs`

Constant groups include:

- component type codes
- button styles
- text input styles
- separator spacing values
- gateway opcodes

## Advice

- Reference constants instead of hardcoded numeric magic values.
- Keep custom IDs + style values centralized in your app code.
