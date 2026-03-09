# talkbank-tools — Copilot Instructions

## Build, Test, and Lint

```bash
# Full build and test
make build          # Build grammar + Rust workspace
make test           # Run all tests (grammar + Rust)
make check          # Fast compile check

# Rust workspace
cargo nextest run --workspace           # All Rust tests (parallel per-test)
cargo nextest run -p talkbank-model     # Single crate
cargo nextest run -p talkbank-parser-tests  # Parser test suite
cargo clippy --all-targets -- -D warnings
cargo fmt

# Single test or file
cargo test test_name                    # Test by name
cargo test --test roundtrip_corpus --release
cargo test --test single_file_roundtrip -- --file path/to/file.cha

# Tree-sitter grammar
cd grammar && tree-sitter generate      # Regenerate parser
cd grammar && tree-sitter test          # Run corpus tests
cd grammar && tree-sitter parse file.cha

# Spec tools (separate workspace)
cd spec/tools && cargo test
```

## Architecture

**Data flow**: `spec/ → grammar/ → crates/`

- **spec/**: CHAT specification files (source of truth)
  - `constructs/` - valid CHAT examples
  - `errors/` - invalid CHAT examples
  - `tools/` - generators that produce tests and docs
- **grammar/**: Tree-sitter grammar for CHAT
  - `grammar.js` - grammar definition (edit this)
  - `src/` - generated C parser (do not edit)
  - `test/corpus/` - generated tests (do not edit)
- **crates/**: Parsers, model, tooling
  - `talkbank-parser` - canonical parser
  - `talkbank-direct-parser` - experimental alternative
  - `talkbank-model` - CHAT data model
  - `talkbank-transform` - pipelines (parse+validate, CHAT↔JSON)

**Parser hierarchy**: Tree-sitter parser is canonical. Direct parser is experimental.

## Key Conventions

- **Specs are the source of truth** - After spec changes, run `make test-gen` to regenerate grammar tests, Rust tests, and error docs
- **Generated files are read-only** - `grammar/src/`, `grammar/test/corpus/`, generated Rust tests are generated; edit specs instead
- **Spec tools is a separate workspace** - Located at `spec/tools/` with its own `Cargo.toml`; use `cd spec/tools && cargo ...`
- **Roundtrip tests use a shared cache** - SQLite cache in `~/.cache/talkbank-tools/` or `~/Library/Caches/`; do not delete without explicit request
