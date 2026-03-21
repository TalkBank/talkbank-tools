# spec - CHAT Specification

## Overview
Markdown specification files define valid constructs and error cases for CHAT.
Generators in `spec/tools` turn these specs into:
1. **Tree-sitter corpus tests** (`../grammar/test/corpus/`)
2. **Rust tests** (future)
3. **Documentation** (future)

## Key Commands

### Preferred (Makefile)
The root `Makefile` automates all generation tasks correctly.
```bash
# Regenerate ALL tests (tree-sitter and Rust) from specs
make test-gen

# Full CI-style regeneration and check (grammar + symbols + tests)
make generated-check
```

### Manual (Debugging)
Run these from the **repository root**:

```bash
# Regenerate ALL tree-sitter tests from specs
cargo run --bin gen_tree_sitter_tests \
  --manifest-path spec/tools/Cargo.toml \
  -- -o ../grammar/test/corpus \
  -t spec/tools/templates

# Regenerate Rust validation tests (if active)
cargo run --bin gen_rust_tests \
  --manifest-path spec/tools/Cargo.toml \
  -- -o crates/talkbank-parser-tests/tests/generated

# Validate spec format integrity
cargo run --bin validate_error_specs \
  --manifest-path spec/runtime-tools/Cargo.toml
```

## Workflows

### Adding a New Construct / Test Case
**Never manually edit files in `../grammar/test/corpus/`.** They will be overwritten.
Mine candidates first, then curate. Do not copy mined files directly into generated corpus.

Quick mining command (staging only):
```bash
cargo run --manifest-path spec/runtime-tools/Cargo.toml --bin extract_corpus_candidates -- \
  --data-dir ../data \
  --languages eng \
  --node-types grammar/src/node-types.json \
  --max-files 20000 \
  --top 50 \
  --require-rust-parse=true \
  --require-rust-validation=true \
  --validate-alignment=true \
  --json \
  --output spec/tmp/mined/candidates.eng.json
```

1. **Create/Edit Spec File**:
   - Locate the appropriate category in `spec/constructs/` (e.g., `tiers/`, `word/`).
   - Create a new `.md` file or add to an existing one.
   - Format:
     ```markdown
     # example_name

     Description of the example.

     ## Input

     ```input_type
     %wor: word 123_456 .
     ```

     ## Expected CST

     ```cst
     (wor_dependent_tier ...)
     ```

     ## Metadata
     - **Level**: tier
     - **Category**: tiers
     ```

2. **Check Templates** (Crucial for new constructs):
   - The `input_type` in the markdown code fence (e.g., `wor_dependent_tier` above) MUST match a template name.
   - Check `spec/tools/templates/`. If `wor_dependent_tier.tera` does not exist, **create it**.
   - Templates wrap the fragment in a valid CHAT document structure so `tree-sitter test` can parse it.
   - Example template (`spec/tools/templates/wor_dependent_tier.tera`):
     ```tera
     @UTF8
     @Begin
     @Languages: eng
     @Participants: CHI Child
     @ID: eng|corpus|CHI|||||Child|||
     *CHI: word .
     {{ input }}
     @End
     ```

3. **Regenerate Tests**:
   - Run the generation command (see Key Commands above).
   - If successful, `../grammar/test/corpus/<category>/<example_name>.txt` will be created/updated.

4. **Verify**:
   - Run `cd ../grammar && tree-sitter test` to confirm the new test passes.

## Architecture
```
spec/
├── constructs/       # Valid CHAT examples (Source of Truth)
│   ├── tiers/        # Dependent tiers (%mor, %gra, %wor...)
│   ├── word/         # Word structure
│   └── ...
├── errors/           # Invalid CHAT examples (Source of Truth for errors)
├── tools/            # Core generator binaries and logic
└── runtime-tools/    # Runtime-aware bootstrap/mining/validation tooling
    ├── src/bin/      # Entry points (gen_tree_sitter_tests, etc.)
    └── templates/    # Tera templates for wrapping test fragments
```

## Cross-Spec Example Consistency

Error spec examples can be cross-referenced — the same `.cha` file may appear in multiple specs with different expected error codes. When changing a grammar rule so that previously-unparsable content now parses:

1. **Update the primary error spec** (e.g., `E518_auto.md`): change `Layer: parser` → `Layer: validation`, update `Expected Error Codes` from `E316` to the specific code.
2. **Audit E316_auto.md**: Remove examples that referenced the same `.cha` files, since they no longer produce E316 parse errors.
3. **Update expectations.json**: Align tree-sitter error codes with direct parser codes and clear `divergence_note`.
4. **Beware accidental test passes**: An E316 test can pass for the wrong reason if the example has *other* E316-triggering content (e.g., main tier after `@End`). Always remove all date/time/sex examples from E316 when their fields get catch-all grammar alternatives.
5. **Run `make test-gen`** after all spec edits to regenerate all test artifacts.

## Status and Limitations
- Specs are the source of truth; regenerate downstream artifacts after changes.
- Generated outputs should not be edited manually.
- Error specs must be classified as parser vs validation and validated.
- **IMPORTANT**: `spec/errors/` is the ONLY place to define error test cases. Never create hand-written `.cha` test fixtures in `tests/error_corpus/` or elsewhere.

## See Also
- tools/CLAUDE.md

---
Last Updated: 2026-03-10
