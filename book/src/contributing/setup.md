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

### 2. Spec workspace (`spec/Cargo.toml`)

Contains two sibling crates for spec-driven artifacts:

```bash
cargo build --manifest-path ~/talkbank/talkbank-tools/spec/tools/Cargo.toml
cargo build --manifest-path ~/talkbank/talkbank-tools/spec/runtime-tools/Cargo.toml
cargo run --manifest-path ~/talkbank/talkbank-tools/spec/tools/Cargo.toml --bin gen_tree_sitter_tests -- --help
cargo run --manifest-path ~/talkbank/talkbank-tools/spec/runtime-tools/Cargo.toml --bin validate_error_specs -- --help
```

## Makefile Targets

```bash
make build           # Build everything
make test            # Run all tests (nextest + parser-tests + doctests)
make verify          # Pre-merge verification gates
make test-gen        # Regenerate spec-driven artifacts when they actually changed
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

See [Testing](testing.md) for the current gate breakdown. The important point is
that `make verify` remains the pre-merge gate, while `make test-gen` is a
targeted regeneration step rather than a universal parser-testing ritual.

## Editor Setup

### VS Code

Install the TalkBank extension from `vscode/` for CHAT syntax highlighting and diagnostics.

### rust-analyzer

The workspace should work out of the box with rust-analyzer. The root `Cargo.toml` workspace configuration is standard.
