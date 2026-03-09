# Spec Workflow

Specifications in `spec/` are the source of truth for CHAT format definitions. All test fixtures and error documentation are generated from specs.

## Adding a Construct Spec

Construct specs define valid CHAT patterns with expected parse trees.

### 1. Create the Spec File

Create a new markdown file in the appropriate `spec/constructs/` subdirectory:

```
spec/constructs/
├── header/         # Header-related constructs
├── main_tier/      # Main tier patterns
├── tiers/          # Dependent tier patterns
├── utterance/      # Utterance-level patterns
└── word/           # Word syntax patterns
```

### 2. Write the Spec

```markdown
# my_example

Description of what this example demonstrates.

## Input

\```utterance
*CHI:	hello world .
\```

## Expected CST

\```cst
(utterance
  (main_tier
    ...))
\```

## Metadata

- **Level**: utterance
- **Category**: main_tier
```

The code fence label (e.g., `utterance`, `mor_dependent_tier`) selects which template wraps the input into a full CHAT file.

### 3. Generate the CST

Parse your input with tree-sitter to get the actual CST, then copy it as the Expected CST (stripping positions and field names).

### 4. Regenerate Tests

```bash
make test-gen
```

## Adding an Error Spec

Error specs define invalid CHAT patterns with expected error codes.

### 1. Create the Spec File

Error specs live in `spec/errors/`, named by error code:

```
spec/errors/E301_missing_participants.md
```

### 2. Write the Spec

```markdown
# Error E301

## Metadata

- Code: E301
- Name: missing_participants
- Severity: Error
- Layer: parser

## Description

The @Participants header is required in every CHAT file.

## Examples

### missing_participants_basic

\```chat
@UTF8
@Begin
*CHI:	hello .
@End
\```
```

### Key Metadata Fields

- **Layer: parser** — the error is caught during `parse_chat_file()` (file fails to parse)
- **Layer: validation** — the error is caught by `validate_with_alignment()` after successful parse
- **Status: not_implemented** — generates `#[ignore]` tests (validation logic not yet coded)

### 3. Regenerate

```bash
make test-gen
make verify
```

## Updating the Symbol Registry

The symbol registry at `spec/symbols/symbol_registry.json` defines character sets used by the grammar and Rust crates.

After editing:

```bash
make symbols-gen    # Regenerate Rust and JS constants
make test-gen       # Regenerate tests
```

## Common Mistakes

- **Editing generated files** — never edit `grammar/test/corpus/` or `crates/talkbank-parser-tests/tests/generated/` by hand
- **Forgetting `make test-gen`** — always regenerate after spec changes
- **Wrong layer** — parser-layer specs expect parse failure; validation-layer specs expect parse success + error report
