# spec/tools - Generators Workspace

## Overview
Rust generators that turn CHAT specs into tests and documentation artifacts.
This is a **separate Cargo workspace** from the main crates — always `cd spec/tools` before running cargo commands.

## Key Commands
```bash
# From repo root (preferred — uses Makefile):
make test-gen           # Regenerate all tests from specs
make generated-check    # Verify generated artifacts are in sync

# Manual (from spec/tools/):
cargo run --bin gen_tree_sitter_tests -- -o ../../grammar/test/corpus -t templates
cargo run --bin gen_rust_tests -- -o ../../crates/talkbank-parser-tests/tests/generated
cargo run --bin gen_validation_tests -- -o ../../crates/talkbank-parser-tests/tests/generated
cargo run --bin gen_error_docs -- -o ../../docs/errors
cargo run --bin validate_error_specs
cargo test
```

## Binary Reference

### Core Workflow (used regularly by contributors)

| Binary | Purpose |
|--------|---------|
| `gen_tree_sitter_tests` | Generate tree-sitter corpus tests from `spec/constructs/` |
| `gen_rust_tests` | Generate Rust parser tests from `spec/errors/` |
| `gen_validation_tests` | Generate Rust validation tests from `spec/errors/` |
| `gen_error_docs` | Generate error documentation from `spec/errors/` |
| `validate_error_specs` | Validate spec format, metadata, and cross-references |
| `validate_spec` | Validate a single spec file |

### Analysis (useful for maintainers)

| Binary | Purpose |
|--------|---------|
| `corpus_node_coverage` | Report which tree-sitter node types are covered by the reference corpus |
| `gen_coverage_dashboard` | Generate HTML coverage dashboard |
| `coverage` | Report spec coverage statistics |

### Bootstrap / Migration (one-off tools, rarely needed)

| Binary | Purpose |
|--------|---------|
| `bootstrap` | Bootstrap initial spec files from corpus examples |
| `bootstrap_tiers` | Bootstrap tier-specific specs from corpus |
| `corpus_to_specs` | Migrate legacy `tests/error_corpus/` fixtures to spec format |
| `extract_corpus_candidates` | Mine corpus for new construct spec candidates |
| `enhance_specs` | Batch-enhance specs with CHAT manual links and descriptions |
| `fix_spec_layers` | One-off migration to fix layer classifications |
| `perturb_corpus` | Generate perturbed corpus files for fuzz-like testing |

## Architecture
```
src/
├── bin/           Entry points (16 binaries)
├── spec/          Spec file loaders and parsers
├── output/        Output formatters (tree-sitter corpus, Rust tests, docs)
├── bootstrap/     Corpus bootstrap utilities
├── generated/     Generated symbol sets (do not edit)
└── templates/     Tera templates for wrapping test fragments in valid CHAT
```

## Testing
```bash
cargo test
```

## See Also
- [spec/CLAUDE.md](../CLAUDE.md) — specification structure and workflows
- [spec/errors/README.md](../errors/README.md) — error spec format reference

---
Last Updated: 2026-03-05
