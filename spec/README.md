# spec — CHAT Specification

## Overview

Markdown specification files define valid constructs and error cases for CHAT.
Generators in `spec/tools/` turn these specs into tree-sitter corpus tests, Rust
tests, and documentation.

**Specs are the source of truth.** Generated artifacts should never be edited
by hand.

## Structure

```
spec/
├── constructs/           Valid CHAT examples (164 specs)
│   ├── header/           Header constructs
│   ├── main_tier/        Main tier constructs
│   ├── tiers/            Dependent tier constructs
│   ├── utterance/        Utterance-level constructs
│   └── word/             Word-level constructs
├── errors/               Error specs (197 files, 181 error codes)
├── symbols/              Shared symbol registry (JSON + generators)
├── tools/                Generator binaries (separate Cargo workspace)
│   ├── src/bin/          Entry points
│   └── templates/        Tera templates for wrapping test fragments
└── docs/                 Format reference and guides
    ├── ERROR_SPEC_FORMAT.md   ← Comprehensive spec format reference
    └── WRITING_ERROR_SPECS.md ← Quick workflow guide
```

## Key Commands

```bash
# Regenerate ALL tests from specs (preferred)
make test-gen

# Manual: tree-sitter tests
cargo run --bin gen_tree_sitter_tests --manifest-path spec/tools/Cargo.toml \
  -- -o ../tree-sitter-talkbank/test/corpus -t spec/tools/templates

# Manual: Rust tests
cargo run --bin gen_rust_tests --manifest-path spec/tools/Cargo.toml \
  -- -o crates/talkbank-parser-tests/tests/generated

# Validate spec format
cargo run --bin validate_error_specs --manifest-path spec/tools/Cargo.toml

# Check error coverage
cargo run --bin coverage --manifest-path spec/tools/Cargo.toml \
  -- --spec-dir spec --errors
```

## Current Coverage

| Metric | Count |
|--------|-------|
| Construct specs | 164 |
| Error specs (total) | 197 files |
| Error codes covered | 181/181 (100%) |
| Error specs with CHAT examples | 169 |
| Documented stubs (untriggerable) | 12 |

## Workflows

See `docs/ERROR_SPEC_FORMAT.md` for the complete format reference, including
metadata fields, layer semantics, code block info strings, and template usage.

See `docs/WRITING_ERROR_SPECS.md` for the practical step-by-step workflow.
See `docs/CURATION_WORKFLOW.md` for the mine -> curate -> generate workflow for construct specs.

## See Also

- `tools/CLAUDE.md` — Generator workspace details
- `CLAUDE.md` (spec directory) — AI assistant guidance
- `../crates/talkbank-parser-tests/CLAUDE.md` — Parser test crate

---
Last Updated: 2026-02-27
