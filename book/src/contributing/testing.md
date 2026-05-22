# Testing

**Status:** Current
**Last updated:** 2026-05-20 20:26 EDT

## Test Generation Pipeline

Specs are the source of truth. All grammar corpus tests, Rust parser tests,
and error docs are **generated** from specs. `make test-gen` wipes the output
directories and recreates them — never hand-edit generated files.

```mermaid
flowchart LR
    subgraph sources["Source of Truth"]
        constructs["spec/constructs/\n(construct specs, see directory listing)"]
        errors["spec/errors/\n(error specs, see directory listing)"]
        templates["spec/tools/templates/\n(Tera wrappers)"]
    end

    subgraph generators["make test-gen"]
        gen_ts["gen_tree_sitter_tests"]
        gen_rust["gen_rust_tests"]
        gen_docs["gen_error_docs"]
    end

    subgraph outputs["Generated Outputs (DO NOT EDIT)"]
        ts_tests["grammar/test/corpus/\n(tree-sitter tests)"]
        rust_tests["tests/generated/\n(Rust tests)"]
        error_docs["docs/errors/\n(per-spec error pages)"]
    end

    constructs & errors --> gen_ts
    templates --> gen_ts
    constructs & errors --> gen_rust
    errors --> gen_docs

    gen_ts --> ts_tests
    gen_rust --> rust_tests
    gen_docs --> error_docs
```

To add a grammar test or error test, add a spec file in `spec/constructs/`
or `spec/errors/`, then run `make test-gen`. See `spec/CLAUDE.md` for the
spec format.

## Test Strategy

Testing is organized in layers, from fastest to most comprehensive.

```mermaid
flowchart TD
    unit["Unit + Integration Tests\n(cargo nextest run)"]
    specgen["Spec-Generated Tests\n(make test-generated)\nParser + validation layer"]
    grammar["Grammar Corpus\n(tree-sitter test)"]
    ref["Reference Corpus\n(corpus/reference/, 100% required)"]
    gates["Verification Gates\n(make verify)\nG0–G14 sequential pipeline"]

    unit --> specgen --> grammar --> ref --> gates
```

### Unit Tests (nextest)

```bash
cargo nextest run
```

Runs all unit and integration tests across all crates (~2300+ tests). These test individual functions, serialization roundtrips, and model invariants.

`cargo nextest` does not run doctests. Keep `cargo test --doc` as a separate
verification step when you change public API examples or doc comments.

### Parser Equivalence

```bash
cargo nextest run -p talkbank-parser-tests -E 'test(parser_equivalence)'
```

Runs the parser on each file in the `corpus/reference/` tree and validates
results. Each `.cha` file is its own test, enabling per-file parallelism and
failure isolation via nextest. The exact file count is whatever
`find corpus/reference -name '*.cha' -type f | wc -l` reports — do not
hard-code it here.

### Spec-Generated Tests

Part of `talkbank-parser-tests`. These are generated from specs via `make test-gen` and currently test:
- Construct specs: input parses correctly
- Parser-layer error specs: input fails to parse with expected error code
- Validation-layer error specs: input parses but validation reports expected error code

Concrete entrypoint:

```bash
make test-generated
```

### Tree-Sitter Grammar Tests

```bash
make test-grammar
```

Runs the tree-sitter grammar corpus tests. This is the right gate for
grammar structure changes.

### Error Corpus Tests

Error fixtures live in `spec/errors/` and are turned into Rust tests
via `make test-gen`. There is no separate `tests/error_corpus/`
manifest in the current layout; add a new error spec under
`spec/errors/E###_*.md` and regenerate.

### Tree-Sitter Tests

```bash
cd grammar
tree-sitter test
```

Verifies the grammar produces correct CSTs for known inputs. The
actual test count comes from `ls grammar/test/corpus/*.txt | wc -l`;
do not hard-code it.

## Reference Corpus

The reference corpus at `corpus/reference/` is organized into subdirs
(`annotation/`, `audio/`, `ca/`, `content/`, `core/`, `edge-cases/`,
`languages/`, `tiers/`, `word-features/`). The parser must handle
every file at 100% — the exact file count is whatever
`find corpus/reference -name '*.cha' -type f | wc -l` reports.

This corpus is the ultimate arbiter of correctness for full-file parsing.

## Verification Gates

`make verify` runs the pre-merge verification suite (G0–G14). All gates
run sequentially — the first failure stops the pipeline.

```mermaid
flowchart TD
    start(["make verify"]) --> G0
    G0["G0: Parser signature guardrail\n(check-errorsink-option-signatures.sh)"]
    G0 -->|pass| G1["G1: Rust workspace compile\n(cargo check --workspace)"]
    G1 -->|pass| G2["G2: Spec tools compile\n(cd spec/tools && cargo check)"]
    G2 -->|pass| G3["G3: Spec runtime tools compile\n(spec/runtime-tools)"]
    G3 -->|pass| G4["G4: CHAT manual anchor links\n(check-chat-manual-anchors.sh)"]
    G4 -->|pass| G5["G5: Generated parser corpus\nequivalence suite"]
    G5 -->|pass| G6["G6: Golden fragment validity\n(words + tiers)"]
    G6 -->|pass| G7["G7: Bare-timestamp\nregression gate"]
    G7 -->|pass| G8["G8: Reference corpus\nsemantic equivalence\n(parser_equivalence_files)"]
    G8 -->|pass| G9["G9: %wor tier parsing\nand alignment"]
    G9 -->|pass| G10["G10: Golden tier roundtrip\n(%mor, %gra, %pho, %wor)"]
    G10 -->|pass| G11["G11: Reference corpus\nnode coverage audit"]
    G11 -->|pass| G12["G12: Generated artifacts\nmatch committed sources"]
    G12 -->|pass| G13["G13: Fuzz workspace\nisolation"]
    G13 -->|pass| G14["G14: Imported Batchalign\nRust/PyO3 gate"]
    G14 -->|pass| done(["All gates passed"])
    G0 & G1 & G2 & G3 & G4 & G5 & G6 & G7 & G8 & G9 & G10 & G11 & G12 & G13 & G14 -->|fail| abort(["Pipeline stops"])
```

| Gate | Check |
|------|-------|
| G0 | Parser signature guardrail |
| G1 | Rust workspace compile check |
| G2 | Spec tools compile check |
| G3 | Spec runtime tools compile check |
| G4 | CHAT manual anchor links |
| G5 | Generated parser corpus equivalence suite |
| G6 | Golden fragment validity (words + tiers) |
| G7 | Bare-timestamp regression gate |
| G8 | Reference corpus semantic equivalence (`parser_equivalence_files` over the full `corpus/reference/` tree) |
| G9 | %wor tier parsing and alignment |
| G10 | Golden tier roundtrip (%mor, %gra, %pho, %wor) |
| G11 | Reference corpus node coverage |
| G12 | Generated artifacts match committed sources |
| G13 | Fuzz workspace isolation |
| G14 | Imported Batchalign Rust/PyO3 gate |

## Running Specific Tests

```bash
# Single test by name
cargo nextest run test_name

# Tests in a specific crate
cargo nextest run -p talkbank-model

# Tests matching a pattern
cargo nextest run -- mor

# With output
cargo nextest run --no-capture
```

## What to Run When

| What you changed | Run |
|-----------------|-----|
| Grammar (`grammar.js`) | `cd grammar && tree-sitter generate && tree-sitter test` then `make test-generated` |
| Parser (CST-to-model) | `cargo nextest run -p talkbank-parser` |
| Model (types, validation, alignment) | `cargo nextest run -p talkbank-model` |
| CLAN command | `cargo nextest run -p talkbank-clan -E 'test(command_name)'` + golden test |
| CLI (chatter args, dispatch) | `cargo nextest run -p talkbank-cli` |
| LSP | `cargo nextest run -p talkbank-lsp` |
| Spec files | `make test-gen && make verify` |
| Pre-merge (any change) | `make verify` |
| Pre-push (quick) | `make ci-local` |

## Mutation Testing

Use `cargo-mutants` to find code that can be changed without any test failing — the true coverage gaps.

```bash
# Install (once)
cargo install cargo-mutants

# Run against a specific crate (--jobs 1 to avoid OOM on 64 GB machines)
cargo mutants -p talkbank-parser --timeout 120 --jobs 1

# Run against CLAN commands
cargo mutants -p talkbank-clan --timeout 120 --jobs 1

# Review results
cat mutants.out/missed.txt    # Mutations no test caught
cat mutants.out/caught.txt    # Mutations properly detected
```

Mutation testing is not part of CI but should be run periodically (after major changes) to find untested logic paths. Results guide where to add new tests.

Configuration: `mutants.toml` at the repo root excludes trivial functions.

## Adding Tests

- **Model tests**: add to the relevant crate's `tests/` directory or `#[cfg(test)]` module
- **Parser tests**: if the change is about grammar shape or validation contracts,
  add or update specs and regenerate with `make test-gen`
- **Error tests**: add a new spec under `spec/errors/E###_*.md` and run
  `make test-gen`; the Rust test under `tests/generated/` is produced automatically
- **CLAN command tests**: add golden test cases in `tests/clan_golden/` using the manifest-driven `ParityCase` / `RustSnapshotCase` pattern
