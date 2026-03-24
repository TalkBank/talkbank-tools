# Architecture Overview

**Status:** Current
**Last updated:** 2026-03-23 23:49 EDT

## Data Flow

Data flows through the system in a single direction, from specification to applications:

```
spec/           Source of truth (CHAT specification)
    ↓
grammar.js      Tree-sitter grammar (in grammar/)
    ↓
parser.c        Generated C parser (never hand-edited)
    ↓
Rust crates     Parser → Model → Validation → Transform
    ↓
Applications    CLI (chatter), LSP, VS Code, CLAN, batchalign
```

## Crate Dependency Graph

```mermaid
flowchart TD
    model["talkbank-model\nData model, validation, alignment, errors"]
    derive["talkbank-derive\nProc macros"]
    parser["talkbank-parser\nCanonical parser (tree-sitter)"]
    transform["talkbank-transform\nPipelines, CHAT↔JSON, caching"]
    clan["talkbank-clan\nCLAN analysis commands"]
    cli["talkbank-cli (chatter)\nCLI: validate, normalize, convert"]
    lsp["talkbank-lsp\nLanguage Server Protocol"]
    s2c["send2clan-sys\nFFI to CLAN app"]
    desktop["chatter-desktop\nDesktop validation app (Tauri)"]
    tests["talkbank-parser-tests\nEquivalence tests"]

    derive --> model
    model --> parser
    parser --> transform
    transform --> clan & cli & lsp & desktop
    clan --> cli & lsp
    s2c --> cli & desktop
    parser --> tests
```

Supporting crate: `talkbank-derive` (proc macros). Downstream consumer: `batchalign3` (path deps to this workspace's crates).

## Repository Layout

Everything lives in a single repository (`talkbank-tools`):

```
talkbank-tools/
├── grammar/                Tree-sitter grammar
├── spec/                   CHAT specification (source of truth)
│   ├── constructs/         Valid CHAT examples + expected parse trees
│   ├── errors/             Invalid CHAT examples + expected error codes
│   ├── symbols/            Shared symbol registry (JSON)
│   ├── tools/              Core spec generators
│   └── runtime-tools/      Runtime-aware spec bootstrap/validation tools
├── crates/                 All Rust crates (parsing, model, CLI, LSP, CLAN, etc.)
├── corpus/                 Reference corpus (78 files)
├── schema/                 JSON Schema (auto-generated)
├── vscode/                 VS Code extension
├── desktop/                Desktop validation app (Tauri v2, React)
├── book/                   This documentation
└── fuzz/                   Fuzz testing targets (separate Cargo workspace)
```

## Two Cargo Workspaces

The `talkbank-tools` repository contains two separate Cargo workspaces:

1. **Root workspace** (`Cargo.toml`) — all Rust crates for parsing, model, and transform
2. **Spec workspace** (`spec/Cargo.toml`) — `spec/tools` for core generation and `spec/runtime-tools` for runtime-aware spec tooling

Run spec-workspace commands with the relevant manifest path:
- `spec/tools/Cargo.toml` for core generators
- `spec/runtime-tools/Cargo.toml` for bootstrap/mining/runtime validation
