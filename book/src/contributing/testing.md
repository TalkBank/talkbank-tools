# Testing

**Status:** Current
**Last updated:** 2026-03-24 00:22 EDT

## Test Generation Pipeline

Specs are the source of truth. All grammar corpus tests, Rust parser tests,
and error docs are **generated** from specs. `make test-gen` wipes the output
directories and recreates them — never hand-edit generated files.

```mermaid
flowchart LR
    subgraph sources["Source of Truth"]
        constructs["spec/constructs/\n(112 construct specs)"]
        errors["spec/errors/\n(187 error specs)"]
        templates["spec/tools/templates/\n(Tera wrappers)"]
    end

    subgraph generators["make test-gen"]
        gen_ts["gen_tree_sitter_tests"]
        gen_rust["gen_rust_tests"]
        gen_docs["gen_error_docs"]
    end

    subgraph outputs["Generated Outputs (DO NOT EDIT)"]
        ts_tests["grammar/test/corpus/\n(166 tree-sitter tests)"]
        rust_tests["tests/generated/\n(167 Rust tests)"]
        error_docs["docs/errors/\n(182 error doc pages)"]
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
    unit["Unit + Integration Tests\n(cargo nextest run)\n~2300 tests, ~5s"]
    specgen["Spec-Generated Tests\n(make test-generated)\nParser + validation layer"]
    grammar["Grammar Corpus\n(tree-sitter test)\n166 tree-sitter tests"]
    ref["Reference Corpus\n(78 files, 100% required)"]
    gates["Verification Gates\n(make verify)\nG0–G11 sequential pipeline"]

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

Runs the parser on each file in the 78-file reference corpus and validates
results. Each `.cha` file is its own test, enabling per-file parallelism and
failure isolation via nextest.

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

Supplementary test fixtures in `tests/error_corpus/`. The `expectations.json` manifest maps `.cha` files to expected outcomes.

### Tree-Sitter Tests

```bash
cd grammar
tree-sitter test
```

160+ tests that verify the grammar produces correct CSTs for known inputs.

## Reference Corpus

The reference corpus at `corpus/reference/` contains 78 `.cha` files across 20 languages that represent the diversity of real-world CHAT data. The parser must handle every file at 100%.

This corpus is the ultimate arbiter of correctness for full-file parsing.

## Verification Gates

`make verify` runs the pre-merge verification suite (G0–G11). All gates
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
    G5 -->|pass| G6["G6: Fragment parsing\nsemantics"]
    G6 -->|pass| G7["G7: Bare-timestamp\nregression gate"]
    G7 -->|pass| G8["G8: Reference corpus\nsemantic equivalence\n(78 files)"]
    G8 -->|pass| G9["G9: %wor tier parsing\nand alignment"]
    G9 -->|pass| G10["G10: Golden tier roundtrip\n(%mor, %gra, %pho, %wor)"]
    G10 -->|pass| G11["G11: Reference corpus\nnode coverage audit"]
    G11 -->|pass| done(["All gates passed"])
    G0 & G1 & G2 & G3 & G4 & G5 & G6 & G7 & G8 & G9 & G10 & G11 -->|fail| abort(["Pipeline stops"])
```

| Gate | Check |
|------|-------|
| G0 | Parser signature guardrail |
| G1 | Rust workspace compile check |
| G2 | Spec tools compile check |
| G3 | Spec runtime tools compile check |
| G4 | CHAT manual anchor links |
| G5 | Generated parser corpus equivalence suite |
| G6 | Fragment parsing semantics |
| G7 | Bare-timestamp regression gate |
| G8 | Reference corpus semantic equivalence (78 files) |
| G9 | %wor tier parsing and alignment |
| G10 | Golden tier roundtrip (%mor, %gra, %pho, %wor) |
| G11 | Reference corpus node coverage |

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
- **Error corpus tests**: add a `.cha` file to `tests/error_corpus/` and update `expectations.json`
- **CLAN command tests**: add golden test cases in `tests/clan_golden/` using the manifest-driven `ParityCase` / `RustSnapshotCase` pattern
