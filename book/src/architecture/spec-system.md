# Spec System

Specifications in `spec/` are the authoritative source of truth for the CHAT format. They drive test generation, documentation, and grammar validation.

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

The `Input` code fence label (e.g., `mor_dependent_tier`, `utterance`) selects which template wraps the fragment into a full CHAT file for parsing.

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

Running `make test-gen` executes three generators:

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

### 3. Error Documentation

`gen_error_docs` generates markdown pages for each error code at `docs/errors/`.

## Workflow After Spec Changes

```bash
cd talkbank-tools
make test-gen     # Regenerate all tests and docs
make verify       # Run pre-merge verification gates
```

Never hand-edit generated artifacts — always regenerate from specs.
