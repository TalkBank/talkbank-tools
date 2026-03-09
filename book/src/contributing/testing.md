# Testing

## Test Strategy

Testing is organized in layers, from fastest to most comprehensive:

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

Runs both parsers (tree-sitter and direct) on each file in the reference corpus and compares results. Each `.cha` file is its own test, enabling per-file parallelism and failure isolation via nextest.

### Spec-Generated Tests

Part of `talkbank-parser-tests`. These are generated from specs via `make test-gen` and test:
- Construct specs: input parses correctly
- Parser-layer error specs: input fails to parse with expected error code
- Validation-layer error specs: input parses but validation reports expected error code

### Error Corpus Tests

Supplementary test fixtures in `tests/error_corpus/`. The `expectations.json` manifest maps `.cha` files to expected outcomes.

### Tree-Sitter Tests

```bash
cd grammar
tree-sitter test
```

160 tests that verify the grammar produces correct CSTs for known inputs.

## Reference Corpus

The reference corpus at `corpus/reference/` contains 74 `.cha` files across 20 languages that represent the diversity of real-world CHAT data. Both parsers must agree on every file at 100%.

This corpus is the ultimate arbiter of correctness. If a change breaks any reference file, it must be reverted.

## Verification Gates

`make verify` runs the pre-merge verification suite (G0–G10):

| Gate | Check |
|------|-------|
| G0 | Parser signature guardrail |
| G1 | Rust workspace compile check |
| G2 | Spec tools compile check |
| G3 | CHAT manual anchor links |
| G4 | Generated parser corpus equivalence suite |
| G5 | Word-level parser equivalence suite |
| G6 | Bare-timestamp regression gate |
| G7 | Reference corpus semantic equivalence (tree-sitter vs direct) |
| G8 | %wor tier parsing and alignment |
| G9 | Golden tier roundtrip (%mor, %gra, %pho, %wor) |
| G10 | Reference corpus node coverage |

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

## Adding Tests

- **Model tests**: add to the relevant crate's `tests/` directory or `#[cfg(test)]` module
- **Parser tests**: add a spec to `spec/constructs/` or `spec/errors/` and run `make test-gen`
- **Error corpus tests**: add a `.cha` file to `tests/error_corpus/` and update `expectations.json`
