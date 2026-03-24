# Introduction

**Status:** Current
**Last updated:** 2026-03-24 00:01 EDT

[TalkBank](https://talkbank.org/) is the world's largest open repository of spoken language data. This repository (`talkbank-tools`) is the complete CHAT toolchain: a tree-sitter grammar, 10 Rust crates, a CLI with interactive TUI, a Language Server, a VS Code extension, and 77 reimplemented CLAN analysis commands. All tools run on **Windows, macOS, and Linux**.

## What's In This Repo

- **`chatter` CLI** — validate, convert, normalize, and analyze CHAT files from the command line, with an interactive TUI for corpus-scale workflows
- **VS Code extension** — live validation, cross-tier alignment visualization, media playback, transcription mode, dependency graphs, and 33 CLAN analysis commands
- **Language Server (LSP)** — powers VS Code but works with any LSP-compatible editor (Neovim, Emacs, Helix, Zed, etc.)
- **77 CLAN commands** — 41 analysis, 23 transforms, 13 format converters, all reimplemented in Rust
- **JSON data model** — every CHAT structure as typed JSON with lossless roundtrip fidelity, backed by a published JSON Schema
- **Rust API** — parse, validate, inspect, and transform CHAT files programmatically via library crates

## Who This Book Is For

- **Researchers** who validate or analyze CHAT files → [Installation](user-guide/installation.md), [CLI Reference](user-guide/cli-reference.md)
- **CLAN users** migrating to the new toolchain → [Migration Guide](user-guide/migrating-from-clan.md)
- **Editors** using VS Code for transcription → [VS Code Extension](user-guide/vscode-extension.md)
- **Developers** building on the CHAT format → [CHAT Format](chat-format/overview.md)
- **Integrators** consuming CHAT data via JSON or Rust → [Integration Guide](integrating/library-usage.md)
- **Contributors** to the toolchain itself → [Contributing](contributing/setup.md)

## Repository Layout

```
grammar/        Tree-sitter grammar (372 rules, 380 node types)
spec/           Source of truth: CHAT specification + error specs
crates/         10 Rust crates (parsers, model, validation, CLAN, CLI, LSP)
vscode/         VS Code extension (TypeScript)
corpus/         Reference corpus (78 .cha files, 20 languages)
schema/         JSON Schema for the CHAT AST
tests/          Integration tests and fixtures
fuzz/           Fuzz testing targets
book/           This documentation (mdBook)
```

Data flows: **spec** (source of truth) → **grammar** (tree-sitter) → **Rust crates** (parsers, model, validation, CLAN, CLI, LSP).
