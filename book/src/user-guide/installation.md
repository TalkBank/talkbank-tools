# Installation

**Status:** Current
**Last updated:** 2026-03-24 00:01 EDT

`chatter` runs on **Windows, macOS, and Linux**. Pre-built binaries are available from [GitHub Releases](https://github.com/TalkBank/talkbank-tools/releases). To build from source, follow the instructions below.

## Prerequisites

- **Rust (stable)** — install via [rustup](https://rustup.rs/) (supports Windows, macOS, Linux)

If you are only building the `chatter` CLI from source, Rust is sufficient.
The extras below are only needed when you work on the grammar or generated
artifacts:

- **Node.js** — required for tree-sitter grammar generation
- **tree-sitter CLI** — `cargo install tree-sitter-cli`

## Installing chatter (CLI)

The `chatter` CLI is the primary tool for working with CHAT files. It provides validation, format conversion, and batch processing.

### From Source

Clone the repository:

```bash
mkdir -p ~/talkbank && cd ~/talkbank
git clone https://github.com/TalkBank/talkbank-tools.git talkbank-tools
```

Build and install:

```bash
cd ~/talkbank/talkbank-tools
cargo install --path crates/talkbank-cli
```

This installs the `chatter` binary to `~/.cargo/bin/` (macOS/Linux) or `%USERPROFILE%\.cargo\bin\` (Windows).

### Verify Installation

```bash
chatter --version
chatter --help
```

## Building the Libraries

If you're developing with the Rust crates directly:

```bash
cd ~/talkbank/talkbank-tools
cargo build           # Build all crates
cargo test            # Run all tests
cargo clippy          # Lint check
```

See the [Makefile targets](../contributing/setup.md) for additional commands.

## Directory Layout

Everything lives in a single repository:

```
~/talkbank/
├── talkbank-tools/         # This repo (grammar, crates, CLI, LSP, VS Code, CLAN)
│   ├── grammar/            # Tree-sitter grammar
│   ├── crates/             # All Rust crates
│   ├── spec/               # CHAT specification
│   └── vscode/             # VS Code extension
└── batchalign3/            # ASR/NLP pipeline (optional, separate repo)
```

The CLI, grammar, and all crates are in this single repository.
