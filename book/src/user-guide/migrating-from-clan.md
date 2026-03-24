# Migrating from CLAN to `talkbank-tools`

**Status:** Current
**Last updated:** 2026-03-24 00:01 EDT

This page is for two audiences:

- users moving day-to-day workflows from legacy CLAN binaries to `chatter`
- developers moving implementation work from older CLAN- or Java/Python-based tooling to the Rust rewrite

It is not a promise of character-for-character equivalence with every historical CLAN binary. The goal of the rewrite is a stable, documented Rust toolchain for CHAT parsing, validation, normalization, JSON interchange, and CLAN-style analyses.

## User Workflow Crosswalk

| Legacy workflow | `talkbank-tools` workflow | Notes |
| --- | --- | --- |
| Run `CHECK` on one file | `chatter validate file.cha` | Validation uses stable Rust error codes and richer diagnostics |
| Run `CHECK` on a corpus | `chatter validate corpus/` | Recursive directory validation with cache support |
| Normalize with `fixit` / `indent` / `longtier` | `chatter normalize file.cha` | Normalization writes to stdout unless `-o/--output` is used |
| Convert to an interchange format | `chatter to-json file.cha` | JSON output is defined by the published schema |
| Convert back from interchange format | `chatter from-json file.json` | Uses the same AST/serializer as the Rust toolchain |
| Run CLAN-style analysis commands | `chatter clan ...` | One entry point, with text/json/csv/clan output modes where supported |

## Important Behavioral Differences

- `chatter clan` is one command family, not a directory of separate CLAN executables.
- JSON conversion is first-class and schema-backed.
- Validation diagnostics use Rust error codes like `E301` or `W603`, not CLAN’s historical numbering.
- Cache behavior is explicit. `validate` reuses cached clean results unless `--force` is passed.
- Normalization does not overwrite files unless you give an output path yourself.

## Recommended User Migration Steps

1. Start by replacing corpus-wide `CHECK` runs with `chatter validate`.
2. Switch any JSON/export glue to `chatter to-json` and `chatter from-json`.
3. Migrate CLAN analysis invocations to `chatter clan ...` and verify any output-format assumptions.
4. Rebaseline automation on Rust error codes and structured JSON output rather than parsing legacy text output.

## Developer Crosswalk

| Legacy implementation style | Rust rewrite replacement |
| --- | --- |
| String-oriented parsing and repair | Typed AST/model in `talkbank-model` |
| Tool-specific parser logic | Shared parser/transform crates |
| Ad-hoc validation checks | Stable validation rules and error codes |
| CLAN-only output assumptions | Shared `talkbank-clan` command implementations plus JSON/CSV support |
| Loosely coupled scripts | Workspace crates plus integration tests and corpus gates |

## What to Change in Developer Workflows

- Build on the Rust crates instead of editing CLAN-era text manipulation logic.
- Treat the AST, serializer, and validator as the source of truth for current behavior.
- Add or update integration tests when changing CLI-visible behavior or corpus-wide semantics.
- Document behavior changes in this book when they affect users or downstream integrations.

## Scope of Compatibility

`talkbank-tools` aims for practical migration, not blind historical reproduction:

- legacy data should continue to parse where feasible
- CLAN-style analyses should document any deliberate divergence
- new public contracts are the Rust CLI, Rust crates, JSON Schema, and documented diagnostics
