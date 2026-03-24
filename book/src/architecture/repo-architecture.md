# Repository Architecture and Boundaries

**Status:** Current
**Last updated:** 2026-03-24 00:01 EDT

## Current Structural Inventory
- `spec/`: construct and error specification corpus plus generator workspace (`spec/tools`).
- `grammar/`: tree-sitter grammar source and generated parser artifacts.
- `crates/`: model, parsers, CLI/LSP, CLAN, transform and support crates.
- `docs/`: audits and supplementary documentation.

## Architectural Principles
1. Clear boundaries between generation, runtime logic, and documentation.
2. Generated artifacts and hand-authored code are kept separate with hard guardrails.
3. Each crate has a single clear responsibility.
4. Entrypoint docs guide new contributors to authoritative references quickly.

## Top-Level Architecture

```text
/spec/                  canonical syntax and error spec source
/spec/tools/            deterministic generators + validators only
/grammar/               CST grammar source + generated parser artifacts
/crates/
  talkbank-model/       data model, validation, alignment, errors, parser API trait
  talkbank-derive/      proc macros (SemanticEq, SpanShift, ValidationTagged)
  talkbank-parser/      parser (tree-sitter)
  talkbank-transform/   pipelines, CHAT↔JSON, caching, parallel validation
  talkbank-clan/        CLAN analysis commands and format converters
  talkbank-cli/         chatter CLI
  talkbank-lsp/         LSP server
  send2clan-sys/        FFI to CLAN app
  talkbank-parser-tests/ parser equivalence and roundtrip tests
/corpus/                reference corpus (78 files, 100% required)
/schema/                JSON Schema for ChatFile AST
/vscode/                VS Code extension (TypeScript)
/book/                  mdBook documentation
/fuzz/                  fuzz testing targets (separate Cargo workspace)
```

## Canonical Ownership Rules
- `spec/` owns language intent and accepted examples.
- `grammar/` owns tokenization and CST shape only, not semantic validation policy.
- `talkbank-model` owns semantic validity, serialization invariants, error types, and parser API contracts.
- `talkbank-transform` owns pipelines and JSON schema validation.

## Dependency Direction Rules
1. `spec` does not depend on runtime crates.
2. `grammar` is consumed by parser crates, not vice versa.
3. `talkbank-model` is dependency-minimal and stable; all other crates depend on it.
4. CLI/LSP depend on stable internal APIs, never directly on unstable internals.
5. Generator tools may read specs and grammar metadata but do not become runtime dependencies.

## Acceptance Criteria
- Every top-level directory has a clear purpose statement.
- No crate depends on internal modules outside declared boundaries.
- No generated artifact is edited manually.
- New contributors can identify authoritative docs in less than five minutes.
