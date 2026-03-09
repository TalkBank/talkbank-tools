# Setup

Development is supported on **Windows, macOS, and Linux**. The instructions below use Unix shell syntax; on Windows, use PowerShell or Git Bash equivalently.

## Prerequisites

- **Rust (stable)** via [rustup](https://rustup.rs/) (all platforms)
- **Node.js** for tree-sitter grammar generation and symbol validation
- **tree-sitter CLI**: `cargo install tree-sitter-cli`

## Clone Repository

```bash
mkdir -p ~/talkbank && cd ~/talkbank
git clone <talkbank-tools-url> talkbank-tools
```

## Build

```bash
cd ~/talkbank/talkbank-tools
cargo build               # Build all crates
cargo build --all-targets # Including tests and benchmarks
```

## Two Cargo Workspaces

The repository has two independent Cargo workspaces:

### 1. Root workspace (`Cargo.toml`)

Contains all Rust crates for parsing, model, validation, and transform:

```bash
cd ~/talkbank/talkbank-tools
cargo build
cargo test
```

### 2. Spec tools (`spec/tools/Cargo.toml`)

Contains generators that produce tests and docs from specifications:

```bash
cd ~/talkbank/talkbank-tools/spec/tools
cargo build
cargo run --bin gen_tree_sitter_tests -- --help
```

## Makefile Targets

```bash
make build           # Build everything
make test            # Run all tests (nextest + parser-tests + doctests)
make verify          # Pre-merge verification gates (G0-G7)
make test-gen        # Regenerate tests from specs
make symbols-gen     # Regenerate shared symbol sets
make generated-check # Verify generated artifacts are committed
make check           # Fast compile check
make clean           # Clean build artifacts
make book            # Build documentation
make book-serve      # Serve documentation locally
```

## Verification

Before submitting changes, run the full verification suite:

```bash
make verify
```

This runs gates G0 through G7, checking compilation, formatting, clippy, tests, parser equivalence, and generated artifact consistency.

## Editor Setup

### VS Code

Install the TalkBank extension from `vscode/` for CHAT syntax highlighting and diagnostics.

### rust-analyzer

The workspace should work out of the box with rust-analyzer. The root `Cargo.toml` workspace configuration is standard.
