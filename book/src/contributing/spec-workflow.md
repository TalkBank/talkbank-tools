# Spec Workflow

Specifications in `spec/` are the source of truth for CHAT format intent, grammar
examples, and validation/error contracts.

They are **not** the sole source of truth for every parser-semantic behavior.
In particular, direct-parser fragment recovery and leniency now need their own
direct-parser-native tests rather than being routed entirely through the older
generation pipeline.

If a workflow still depends on synthetic tree-sitter fragment behavior, treat
it as an audit or compatibility path, not as the default semantic contract.

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

The code fence label (e.g., `utterance`, `mor_dependent_tier`) selects which
template wraps the input into a full CHAT file.

That templating is a grammar/test-generation mechanism. It should not be
confused with the semantic contract for honest isolated fragment parsing.

### 3. Generate the CST

Parse your input with tree-sitter to get the actual CST, then copy it as the Expected CST (stripping positions and field names).

### 4. Regenerate The Affected Generated Artifacts

```bash
make test-gen
```

Use `make test-gen` when you intentionally changed generated grammar corpus
tests, generated Rust tests, or generated error docs.

If your change is primarily about direct-parser fragment semantics or recovery,
prefer adding direct-parser-native tests instead of assuming generation is the
right answer.

For isolated grammar additions, keep the change small:

1. Add or adjust one grammar example.
2. Add one direct-parser-native semantic test for the new fragment behavior.
3. Add one full-file fixture if the change matters in context.
4. Regenerate only the artifacts that truly changed.

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

### 3. Regenerate The Affected Artifacts

```bash
make test-gen
make verify
```

## Updating the Symbol Registry

The symbol registry at `spec/symbols/symbol_registry.json` defines character sets used by the grammar and Rust crates.

```mermaid
flowchart TD
    registry["Edit spec/symbols/\nsymbol_registry.json"]
    validate["validate_symbol_registry.js\n(structure check)"]
    gen_grammar["Generate grammar symbols\n(for tree-sitter)"]
    gen_rust["generate_rust_symbol_sets.js\n→ talkbank-model/src/generated/symbol_sets.rs\n→ spec/tools/src/generated/symbol_sets.rs"]
    fmt["rustfmt\n(format generated code)"]
    verify["make symbols-gen\nthen make verify"]

    registry --> validate --> gen_grammar & gen_rust
    gen_rust --> fmt --> verify
    gen_grammar --> verify
```

After editing:

```bash
make symbols-gen    # Regenerate Rust and JS constants
make test-gen       # If generated grammar/tests/docs depend on the symbols
```

## Common Mistakes

- **Editing generated files** — never edit `grammar/test/corpus/` or `crates/talkbank-parser-tests/tests/generated/` by hand
- **Treating specs as the whole parser-testing story** — direct-parser fragment semantics and recovery often deserve direct tests rather than more generation
- **Running `make test-gen` reflexively** — use it when generated artifacts changed, not as a substitute for thinking about what kind of test authority the change really needs
- **Wrong layer** — parser-layer specs expect parse failure; validation-layer specs expect parse success + error report
