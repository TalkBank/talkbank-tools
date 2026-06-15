# Introduction

**Status:** Current
**Last updated:** 2026-04-29 10:26 EDT

[TalkBank](https://talkbank.org/) is the world's largest open repository of spoken language data. This repository (`talkbank-tools`) is the home of several related surfaces: the `chatter` CLI, source-first preview Rust crates for CHAT parsing/validation, the `tree-sitter-talkbank` grammar, and the preview `batchalign3` product line.

Platform and support status now depend on the surface. `chatter` ships release binaries for Windows, macOS, and Linux; the public Rust core is a source-level cross-platform surface; `batchalign3` is public preview on the same three operating systems; and the desktop app in this repo remains experimental. See the repo-root `docs/PLATFORM-SUPPORT.md` and `docs/RELEASE-CONTRACT.md`.

## Choose the right surface

| Task | Recommended Surface | Support Status |
|---|---|---|
| **CHAT validation, normalization, conversion, or CLAN analysis** | `chatter` CLI | ✅ Stable; binaries for Windows, macOS, Linux |
| **Transcribe, align, or morphotag CHAT with audio/ML** | `batchalign3` CLI/server | 🔷 Preview; wheels for Windows, macOS, Linux |
| **Build CHAT tooling in Rust** | Public Rust crates (`talkbank-model`, `talkbank-parser`, etc.) | 🔷 Preview; source-first via path dependencies |
| **Reuse grammar in other tools** | `tree-sitter-talkbank` npm package | 🔷 Preview; API not yet frozen |
| **Standalone desktop GUI for Batchalign** | Batchalign Desktop (`apps/dashboard-desktop/`) | ⚠️ Experimental only; build from source |

**Legend:** ✅ = Stable public release | 🔷 = Public preview | ⚠️ = Experimental (not supported for end-users)

## What's In This Repo

- **`chatter` CLI** — validate, convert, normalize, and analyze CHAT files from the command line, with an interactive TUI for corpus-scale workflows
- **CLAN command surface** — 1 validation + 34 analysis + 23 transform commands plus a handful of format converters, covering the full CLAN binary set in `OSX-CLAN/src/unix/bin/`. Six legacy NLP commands (MOR, POST, MEGRASP, etc.) are deliberately not implemented and direct callers to Batchalign's neural pipeline instead. See the [CLAN command status matrix](clan-reference/appendices/status-matrix.md).
- **JSON data model** — every CHAT structure as typed JSON with lossless roundtrip fidelity, backed by a published JSON Schema
- **Rust API** — parse, validate, inspect, and transform CHAT files programmatically via library crates

## Who This Book Is For

- **Researchers** who validate or analyze CHAT files → [Installation](chatter/user-guide/installation.md), [CLI Reference](chatter/user-guide/cli-reference.md)
- **CLAN users** migrating to the new toolchain → [Migration Guide](chatter/user-guide/migrating-from-clan.md)
- **Developers** building on the CHAT format → [CHAT Format](chat-format/overview.md)
- **Integrators** consuming CHAT data via JSON or Rust → [Integration Guide](chatter/integrating/library-usage.md)
- **Contributors** to the toolchain itself → [Contributing](contributing/setup.md)

## Repository Layout

```text
grammar/        Tree-sitter grammar (~380 rules, ~410 node types)
spec/           Source of truth: CHAT specification + error specs
crates/         talkbank-* (parsers, model, validation, CLAN, CLI) + batchalign-* (runtime, types, PyO3 bridge) + send2clan-sys
batchalign/     Python worker code (ML inference hosting, internal)
apps/           Tauri v2 desktop app (dashboard-desktop, experimental)
frontend/       React dashboard for the Batchalign server
corpus/         Reference corpus (100 .cha files, 20+ languages, 100% pass required)
schema/         JSON Schema for the CHAT AST
tests/          Integration tests and fixtures
fuzz/           Fuzz testing targets (separate Cargo workspace)
book/           This documentation (mdBook)
```

Data flows: **spec** (source of truth) → **grammar** (tree-sitter) → **Rust crates** (parsers, model, validation, CLAN, CLI) → **applications** (chatter, batchalign3, desktop app).
