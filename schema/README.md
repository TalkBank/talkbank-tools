# JSON Schemas

**Status:** Current
**Last updated:** 2026-05-21 14:45 EDT

This directory contains generated JSON Schema artifacts for the TalkBank
transcript model.

## Files

- **`chat-file.schema.generated.json`** -- generated transcript-model schema artifact
- **`chat-file.schema.json`** -- checked-in transcript-model schema

The `.generated.json` and canonical `.json` files currently contain the same
content. Keeping both matches the shared schema-generation harness and makes it
clear which files are machine-written outputs.

## Canonical URLs

```
https://talkbank.org/schemas/v0.1/chat-file.json
```

The `$id` in the schema resolves to its canonical URL. The transcript-model
schema also has a `/latest/` alias that always points to the current version.

## How it's generated

The schemas are auto-generated from Rust type definitions using
[schemars](https://docs.rs/schemars). A shared test harness adds metadata
(`$schema`, `$id`, `$comment`, `description`) and writes the files.

**Never edit these schema files by hand.** To regenerate:

```bash
cargo test --test generate_schema
```

The transcript schema comes from `talkbank-model`.

## Versioning

These schema URLs currently follow the workspace contract version under
`/v0.1/`. Patch releases that don't change a schema shape can reuse the same
versioned URL.

## Deployment

The checked-in `.json` files in this directory are what gets published
at the canonical URLs above. There is no in-tree deploy script today;
publication is handled out-of-band as part of the release process.

## Further reading

- [JSON Schema book chapter](../book/src/integrating/json-schema.md) -- usage
  guide, external validation examples, and transcript roundtrip guarantee
- `crates/talkbank-transform/src/json/mod.rs` -- Rust API for schema-validated
  serialization
- `tests/generate_schema/` -- shared generation helpers, metadata injection, schema transforms
