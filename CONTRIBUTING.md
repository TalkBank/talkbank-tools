# Contributing to talkbank-tools

Thank you for contributing.

This repository contains the CHAT specification and core Rust library crates:
- `spec/` is the specification source of truth.
- `crates/` contains the core Rust library crates (parsing, model, validation, etc.).

The grammar lives in `grammar/`.

### External Dependency Note

The file `crates/talkbank-parser-tests/src/generated_traversal.rs` is generated
by [`tree-sitter-grammar-utils`](https://github.com/TalkBank/tree-sitter-grammar-utils),
which is not yet published. If your changes require regenerating this file
(i.e., you modified `grammar/grammar.js` in a way that changes the CST node
types), note this in your PR and a maintainer will regenerate it.

Most contributions (spec changes, validation logic, CLAN commands, CLI features)
do not require this step.

The CLI and LSP live in `crates/talkbank-cli/` and `crates/talkbank-lsp/`.

## Development Setup
1. Install Rust (stable).
2. Install Node.js (for spec tooling).

Core commands:
```bash
make build
make test
make check
make test-gen
make chat-anchors-check
```

`make chat-anchors-check` validates all `CHAT.html#...` links in `crates/`, `schema/`, and `docs/` against the published CHAT manual at `https://talkbank.org/0info/manuals/CHAT.html` by default.
To validate against a local mirror instead, pass:

```bash
CHAT_HTML_PATH=/abs/path/to/CHAT.html make chat-anchors-check
```

Without `CHAT_HTML_PATH`, the script fetches from:

```bash
CHAT_HTML_URL=https://talkbank.org/0info/manuals/CHAT.html make chat-anchors-check
```

This check is now part of required CI gates.

## Required Workflow
If you change specs, symbols, or other inputs that feed generated artifacts,
regenerate the affected outputs:
```bash
make test-gen
```

## Before Opening a PR
Run at minimum:
```bash
make verify
```

If you need additional confidence for broad changes, also run:
```bash
cargo test --workspace
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
```

## Generated Files
Do not hand-edit generated artifacts.
Regenerate them from their source inputs and include the generated updates in the same PR.

## Pull Request Expectations
Include:
- what changed and why,
- which subsystem(s) were touched,
- tests run,
- whether generated files changed.
- whether docs were updated (or why not),
- whether integrator/API behavior changed (or why not).

## Documentation Expectations
Update docs in the same PR when behavior, workflows, or contracts change.

## Reporting Bugs
Open an issue with:
- minimal reproduction,
- expected behavior,
- actual behavior,
- relevant files and commands.
