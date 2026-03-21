# Grammar Workflow

The tree-sitter grammar at `grammar/grammar.js` is the formal definition of the CHAT format. Changes require careful validation.

## Step-by-Step Procedure

### 1. Edit the Grammar

Modify `grammar.js` in the `grammar/` directory. Key design principles:

- Explicit whitespace (no `extras`)
- Precedence annotations to resolve ambiguities
- Named rules for all semantically meaningful nodes

### 2. Generate the Parser

```bash
cd grammar
tree-sitter generate
```

This produces `src/parser.c` and `src/node-types.json`. Never edit these files by hand.

### 3. Run Grammar Tests

```bash
tree-sitter test
```

All 160 tests must pass. Tests live in `test/corpus/` and are partially auto-generated from specs.

### 4. Run Parser Tests

```bash
cargo test -p talkbank-parser
```

This verifies the Rust parser wrapper handles all CST nodes correctly.

### 5. Run Parser Equivalence

```bash
cargo nextest run -p talkbank-parser-tests -E 'test(parser_equivalence)'
```

Both parsers must agree on every file in the reference corpus. Each `.cha` file is its own test — nextest runs them in parallel and reports individual failures. If the grammar change affects parsing output, the direct parser may need corresponding updates.

### 6. Regenerate Spec Tests

If the grammar change affects any spec examples:

```bash
make test-gen
```

This regenerates tree-sitter corpus tests and other generated outputs that
still depend on the spec pipeline.

Do this when the grammar change actually affects generated artifacts. Do not use
`make test-gen` as a blanket replacement for direct parser semantic testing.

### 7. Update node_types.rs

If new node types were added to the grammar, the generated `node_types.rs` in `talkbank-parser` needs updating. The spec tools handle this via `node-types.json`.

## Critical Policy

The reference corpus at `corpus/reference/` (74 files) must pass parser equivalence at 100%. If a grammar change breaks even one file, revert immediately. The reference corpus is the ultimate arbiter of correctness.

That rule does not mean every parser-semantic change should be proved by the
same corpus. When the direct parser grows a new isolated recovery or leniency
contract, add direct-parser-native tests for that contract first, then use the
reference corpus only to catch real full-file regressions.

## Common Patterns

### Adding a New Token

1. Define the token in `grammar.js`
2. Add handling in the Rust tier parser (match on the new node kind)
3. Add a spec construct example
4. Add direct-parser-native tests if the new syntax affects fragment semantics
   or recovery
5. Run the relevant generation and verification steps

For small, isolated syntax additions, the grammar workflow should stay local:

- one grammar change
- one grammar corpus example
- one direct-parser-native fragment/recovery test
- one full-file fixture if needed

### Changing a Rule

1. Modify the rule in `grammar.js`
2. `tree-sitter generate && tree-sitter test`
3. Update Rust parser if CST node structure changed
4. Update spec examples if the expected CST changed
5. Add or update direct-parser semantic tests if fragment behavior changed
6. Run full verification

Do not treat `tree-sitter test` plus `make test-gen` as a substitute for the
direct parser's own semantic contract.
