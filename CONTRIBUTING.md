# Contributing to talkbank-tools

**Status:** Current
**Last updated:** 2026-04-29 13:15 EDT

Thank you for contributing.

This repository is the unified home for:

- the CHAT specification and grammar pipeline
- the core Rust crates (`talkbank-*`)
- the `chatter` CLI and `talkbank-lsp`
- the VS Code extension
- the imported Batchalign stack (`batchalign3`, Python package, `batchalign-*` crates, dashboard, PyO3 bridge)

Start with the root [README.md](README.md) for the documentation map by surface.

### External Dependency Note

The file `crates/talkbank-parser-tests/src/generated_traversal.rs` is generated
by [`tree-sitter-grammar-utils`](https://github.com/TalkBank/tree-sitter-grammar-utils),
which is not yet published. If your changes require regenerating this file
(i.e., you modified `grammar/grammar.js` in a way that changes the CST node
types), note this in your PR and a maintainer will regenerate it.

Most contributions (spec changes, validation logic, CLAN commands, CLI features)
do not require this step.

The main user-facing binaries live in:

- `crates/talkbank-cli/` -> `chatter`
- `crates/talkbank-lsp/` -> `talkbank-lsp`
- `crates/batchalign/` -> `batchalign3`

## Development Setup
1. Install Rust (stable).
2. Install Node.js (for grammar/frontend tooling).
3. Install `uv` for the Python/Batchalign surfaces.

Core commands:
```bash
make check
make test
make verify
make batchalign-check
make batchalign-test-rust
make batchalign-test-integration
make batchalign-test-python
make batchalign-typecheck-python
make batchalign-ci-python
make ci-local
make ci-full
make chat-anchors-check
```

For the full target list:

```bash
make help
```

For xtask helpers:

```bash
cargo run -q -p xtask -- help
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

If you changed imported Batchalign code or packaging/runtime surfaces, also run:

```bash
make batchalign-check
make batchalign-test-rust
make batchalign-test-integration
make batchalign-ci-python
```

If you changed the dashboard frontend, also run:

```bash
make batchalign-dashboard-api-check  # Verify API types are in sync
make batchalign-dashboard-e2e        # Quick mock-server e2e tests
```

For a comprehensive confidence check before a dashboard PR, also run:

```bash
make batchalign-dashboard-build      # Verify build completes
make batchalign-dashboard-e2e-real   # Integration tests with real server
```

If you need broader confidence for cross-cutting changes, also run:
```bash
make ci-local
make ci-full
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

Key doc surfaces:

- `book/` — the unified TalkBank Toolchain mdBook. All four product
  surfaces (chatter, Batchalign3, VS Code extension, CLAN command
  reference) live as sections under `book/src/`.
- `vscode/README.md` for the VS Code extension entrypoint
- crate READMEs for component-specific entrypoints

## Reporting Bugs
Open an issue with:
- minimal reproduction,
- expected behavior,
- actual behavior,
- relevant files and commands.
