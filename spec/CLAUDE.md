# spec — CHAT Specification

**Status:** Current
**Last updated:** 2026-03-24 00:22 EDT

## How This Works

Specs are the **single source of truth** for all CHAT grammar tests, parser
tests, and error documentation. You never hand-edit generated test files.

```
spec/constructs/*.md  ─┐
                       ├──► make test-gen ──► grammar/test/corpus/*.txt  (tree-sitter tests)
spec/errors/*.md      ─┤                 ──► crates/.../tests/generated/ (Rust tests)
                       │                 ──► docs/errors/*.md            (error docs)
spec/tools/templates/ ─┘
```

**`make test-gen` wipes all three output directories and recreates them.**
If you hand-edit a file in `grammar/test/corpus/` or `tests/generated/`,
it will be deleted next time someone runs `make test-gen`.

## Spec Counts (current)

| Location | Files | Purpose |
|----------|------:|---------|
| `spec/constructs/` | 112 | Valid CHAT examples with expected CSTs |
| `spec/errors/` | 187 | Invalid CHAT examples with expected error codes |
| → `grammar/test/corpus/` | 166 | Generated tree-sitter tests |
| → `tests/generated/` | 167 | Generated Rust parser/validation tests |
| → `docs/errors/` | 182 | Generated error documentation pages |

## Adding a Test

### 1. Create a spec file

Put it in the right directory under `spec/constructs/` or `spec/errors/`:

```
spec/constructs/
├── header/      # @-header examples
├── main_tier/   # *SPK: line examples
├── tiers/       # %mor, %gra, %sin, %wor etc.
├── utterance/   # Full utterance (main + dependent tiers)
└── word/        # Word-internal structure
```

### 2. Spec format (constructs)

```markdown
# example_name

Description of what this tests.

## Input

```input_type
*CHI:	hello .
```

## Expected CST

```cst
(main_tier ...)
```

## Metadata

- **Level**: main_tier
- **Category**: main_tier
```

The `input_type` in the code fence (e.g., `main_tier`, `standalone_word`,
`utterance`) tells the generator which **template** to use for wrapping the
fragment in a full CHAT document. Templates live in `spec/tools/templates/`.

### 3. Spec format (errors)

```markdown
# E999 — Description

Error for some condition.

- **Code**: E999
- **Severity**: Error
- **Layer**: parser | validation
- **Status**: implemented | not_implemented

## Example

```chat
@UTF8
@Begin
...invalid content...
@End
```

## Expected Error Codes

- E999
```

### 4. Check templates

The `input_type` must match a `.tera` template in `spec/tools/templates/`.
If no template exists for your fragment type, create one. Templates wrap the
fragment in valid CHAT scaffolding so `tree-sitter test` can parse it.

Example (`spec/tools/templates/main_tier.tera`):
```
@UTF8
@Begin
@Languages:	eng
@Participants:	CHI Target_Child
@ID:	eng|test|CHI|||||Target_Child|||
{{ input }}
@End
```

### 5. Regenerate and verify

```bash
make test-gen          # Regenerate all outputs from specs
tree-sitter test       # Verify grammar tests pass (from grammar/)
make verify            # Full verification pipeline
```

## Key Commands

```bash
# Regenerate ALL generated artifacts from specs
make test-gen

# Full CI-style check (grammar + symbols + tests + verification)
make generated-check

# Verify spec format integrity
cargo run --bin validate_error_specs \
  --manifest-path spec/runtime-tools/Cargo.toml
```

## Generator Binaries (`spec/tools/src/bin/`)

| Binary | What it generates |
|--------|-------------------|
| `gen_tree_sitter_tests` | `grammar/test/corpus/*.txt` from constructs + errors |
| `gen_rust_tests` | `crates/.../tests/generated/*.rs` from constructs + errors |
| `gen_error_docs` | `docs/errors/*.md` from errors |
| `validate_spec` | Validates spec format integrity (no output) |
| `corpus_node_coverage` | Reports which grammar node types are exercised by `corpus/reference/` |
| `extract_corpus_candidates` | Mines real `.cha` files for candidate specs (runtime-tools) |

## Cross-Spec Consistency

Error spec examples can be cross-referenced — the same `.cha` content may
appear in multiple specs with different expected error codes. When changing a
grammar rule so that previously-unparsable content now parses:

1. Update the primary error spec: change `Layer: parser` → `Layer: validation`
2. Audit `E316_auto.md`: remove examples that no longer produce E316
3. Run `make test-gen` to regenerate all artifacts
4. Run `make verify` to confirm

## See Also
- `spec/tools/CLAUDE.md` — generator implementation details
- `grammar/CLAUDE.md` — grammar change workflow
- `book/src/contributing/testing.md` — testing strategy
