# Spec System

Specifications in `spec/` are the authoritative source of truth for the CHAT format. They drive grammar artifact generation, validation/error docs, and targeted test generation.

**Important:** this system was shaped during the direct-parser bootstrap era.
That history matters. Fragment specs are still valuable, but synthetic
tree-sitter wrapper behavior should no longer be treated as the semantic oracle
for fragment parsing. That behavior is now audit-only legacy unless a page or
test explicitly says otherwise.

For the target long-term shape, see
[Post-Bootstrap Parser Testing](post-bootstrap-parser-testing.md).

## Spec Types

### Construct Specs (`spec/constructs/`)

Each construct spec defines a valid CHAT pattern with its expected parse tree:

```markdown
# example_name

Description of what this example tests.

## Input

\```mor_dependent_tier
%mor:	VERB|eat .
\```

## Expected CST

\```cst
(mor_dependent_tier
  (mor_tier_prefix)
  ...)
\```

## Metadata

- **Level**: tier
- **Category**: tiers
```

The `Input` code fence label (e.g., `mor_dependent_tier`, `utterance`) selects
which template wraps the fragment into a full CHAT file for parsing.

That is an explicit **grammar/test templating** mechanism. It is useful, but it
does **not** by itself define honest isolated-fragment semantics for the direct
parser.

### Error Specs (`spec/errors/`)

Each error spec defines an invalid CHAT pattern with expected error codes:

```markdown
# Error E301

## Metadata

- Code: E301
- Name: missing_participants
- Severity: Error
- Layer: parser

## Examples

### missing_participants_1

\```chat
@UTF8
@Begin
*CHI: hello .
@End
\```
```

Key metadata fields:
- **Layer: parser** — error caught during parsing (returns `Err`)
- **Layer: validation** — error caught after successful parse
- **Status: not_implemented** — generates `#[ignore]` tests

### Symbol Registry (`spec/symbols/`)

`symbol_registry.json` defines character sets used by both the grammar and Rust crates. Running `make symbols-gen` generates:
- JavaScript constants for `grammar.js`
- Rust constants for model validation

## Test Generation

Running `make test-gen` currently executes three generators:

### 1. Tree-sitter Corpus Tests

`gen_tree_sitter_tests` reads construct specs and error specs, then:
- Wraps each `Input` in a template to create a full CHAT file
- Parses with tree-sitter and checks for error nodes
- Writes `Expected CST` to `grammar/test/corpus/`

For error specs, it captures the actual parse (with ERROR nodes) as the expected tree.

### 2. Rust Tests

`gen_rust_tests` generates Rust test functions:
- Construct specs become parse-and-compare tests
- Parser-layer error specs become `parse_chat_file` tests expecting `Err`
- Validation-layer error specs become parse-then-validate tests

Output: `crates/talkbank-parser-tests/tests/generated/`

This is one of the main post-bootstrap reassessment points. It was useful when
the direct parser was being compared against an existing baseline, but it is too
coarse as the long-term semantic testing story for direct-parser fragment
behavior. The generated suites should now be treated as grammar/audit support,
not the sole authority for fragment semantics.

### 3. Error Documentation

`gen_error_docs` generates markdown pages for each error code at `docs/errors/`.

## Workflow After Spec Changes

```bash
cd talkbank-tools
make test-gen     # Regenerate the affected spec-driven artifacts
make verify       # Run pre-merge verification gates
```

Never hand-edit generated artifacts — always regenerate from specs.

## Post-Bootstrap Doctrine

- `spec/tools` remains the generator/validator for grammar corpus tests, error
  docs, and shared symbol artifacts.
- `talkbank-parser-tests` and `talkbank-direct-parser` own fragment semantics
  and recovery contracts.
- Synthetic tree-sitter fragment helpers are audit-only legacy unless a test
  explicitly calls out compatibility or migration behavior.
- Isolated grammar additions should usually need three things: one grammar
  corpus example, one direct-parser-native fragment test, and one full-file
  fixture. They should not require the old bootstrap ritual unless generated
  artifacts really changed.
