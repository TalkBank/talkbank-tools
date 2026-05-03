# Testing and Quality Gates

**Status:** Current
**Last updated:** 2026-04-29 10:39 EDT

This page summarizes the **current** relationship between the local pre-merge
gate (`make verify`) and the root CI workflow (`.github/workflows/ci.yml`).
See [Testing](testing.md) for the canonical local gate definitions.

## Local pre-merge contract

`make verify` is the maintainer-facing local contract. It runs gates G0–G14 in
sequence. `hooks-check` runs first as a warning, but it is not a numbered gate.

## Root CI contract

Root CI is broader than `make verify`, but it is **not** a byte-for-byte mirror
of the local gate sequence. The workflow includes local-contract coverage where
practical, plus CI-only jobs such as grammar generation, reference-corpus
roundtrip, VS Code jobs, cross-platform CLI smoke, dependency audit, and the
aggregate `ci-report`.

### Local gate coverage in CI

| Local gate | Local command | CI coverage today |
|---|---|---|
| G0 | `make parser-guard` | `rust-check-and-test` |
| G1 | `cargo check --workspace --all-targets` | `rust-check-and-test` |
| G2 | `cd spec/tools && cargo check --all-targets` | `spec-tools` |
| G3 | `cargo check --manifest-path spec/runtime-tools/Cargo.toml --all-targets` | **Not mirrored in root CI** |
| G4 | `make chat-anchors-check` | `chat-manual-anchor-check` |
| G5 | `cargo nextest run -p talkbank-parser-tests --test generated` | `rust-check-and-test` |
| G6 | `make test-fragment-semantics` | `rust-check-and-test` |
| G7 | `cargo nextest run --test bare_timestamp_regression` | `rust-check-and-test` |
| G8 | `cargo nextest run -p talkbank-parser-tests --test parser_equivalence_files` | `rust-check-and-test` |
| G9 | `cargo nextest run -p talkbank-parser-tests --test wor_terminator_alignment` | `rust-check-and-test` |
| G10 | `cargo nextest run -p talkbank-parser-tests --test parser_suite` | `rust-check-and-test` |
| G11 | `make coverage` | **Not mirrored in root CI** |
| G12 | `make generated-check` | `generated-artifacts` |
| G13 | `make fuzz-check` | Covered more broadly by `fuzz-smoke`, not by the same command |
| G14 | `make batchalign-ci-rust` | **Not mirrored in root CI** |

### Additional CI-only checks

These are required CI signals but are not part of `make verify`:

- `grammar`
- `reference-corpus-roundtrip`
- `vscode` and `vscode-vsix-smoke`
- `cross-platform-smoke`
- `dependency-audit`
- `semver-checks` (pull requests)
- `ci-report`
