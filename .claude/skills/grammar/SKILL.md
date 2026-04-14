---
name: grammar
description: Edit the tree-sitter CHAT grammar (grammar.js) and run the mandatory verification sequence. Use when modifying CHAT syntax rules, adding new constructs, or fixing parser behavior.
disable-model-invocation: true
allowed-tools: Bash, Read, Write, Edit, Glob, Grep, Agent
---

# Edit Tree-sitter CHAT Grammar

Modify the CHAT grammar and verify the change. `$ARGUMENTS` should describe what to change (e.g., "add support for @NewHeader" or "fix parsing of nested brackets").

## Critical Rules

1. **NEVER edit `src/parser.c`** — it is generated from `grammar.js`
2. **NEVER hand-edit `test/corpus/`** — it is generated from specs
3. The 88-file reference corpus must remain at 100% parser equivalence
4. If verification fails and a fix is not immediate, revert the grammar change

## Step 1: Understand Current Grammar

```bash
# Read the grammar definition (this is the source of truth)
cat $REPO_ROOT/grammar/grammar.js

# Check existing rules related to your change
grep -n "<keyword>" $REPO_ROOT/grammar/grammar.js
```

## Step 2: Make the Grammar Change

Edit `$REPO_ROOT/grammar/grammar.js`.

### Design Pattern: Strict + Catch-All

For header fields with a closed set of valid values, use the **strict + catch-all** pattern:

```javascript
// Known values as named nodes (syntax highlighting) + generic catch-all (validator flags)
option_name: $ => choice('CA', 'NoAlign', $.generic_option_name),
generic_option_name: $ => /[^\s,\r\n\t]+/,
```

Tree-sitter's disambiguation gives string literals priority over regexes at the same length. Known values win; unknown values fall through to the catch-all. The Rust validator flags unsupported values with a specific error code.

10 existing rules use this pattern: `option_name`, `media_type`, `media_status`, `recording_quality_option`, `transcription_option`, `number_option`, `date_contents`, `time_duration_contents`, `id_sex`, `id_ses`.

**When NOT to use:** free-text fields, regex-only fields, fields where regex IS the validation.

## Step 3: Mandatory Verification Sequence

Run in this exact order — **do not skip steps**:

```bash
# 1. Regenerate parser from grammar.js (MANDATORY after every edit, including reverts)
cd $REPO_ROOT/grammar && tree-sitter generate

# 2. Run tree-sitter's own test suite
cd $REPO_ROOT/grammar && tree-sitter test

# 3. Run Rust parser tests
cd $REPO_ROOT && cargo nextest run -p talkbank-parser

# 4. Run parser equivalence tests (74 files, 100% required)
cd $REPO_ROOT && cargo nextest run -p talkbank-parser-tests

# 5. Check for stale generated artifacts
cd $REPO_ROOT && make generated-check
```

### Stop Conditions

- If step 2 fails → stop and triage diffs before continuing
- If step 3 fails → stop and fix parser/model drift
- If step 4 fails → stop and fix parser-equivalence drift
- Do not update corpus expectations automatically; only update after confirming intended behavior

## Step 4: Test on Real Files

```bash
# Parse a specific file to verify
cd $REPO_ROOT/grammar && tree-sitter parse path/to/file.cha

# Run CLI validation
cd $REPO_ROOT && cargo run -p talkbank-cli -- validate path/to/file.cha
```

## Step 5: Update Specs (if adding new construct)

If the grammar change adds a new construct or changes error behavior:

1. Check if error specs need updating (new parse-layer constructs may change E316 behavior)
2. Run `make test-gen` to regenerate tests from specs
3. Run `make verify` for full verification

### Catch-All Pitfall

When adding a catch-all to a field, also audit error specs (`spec/errors/`) that referenced E316 (UnparsableContent) for that field. Those examples will now parse successfully, so:

1. Remove them from E316 specs
2. Move them to the field-specific error spec with `Layer: validation`
3. Update `tests/error_corpus/expectations.json` divergence entries
4. Re-run `make test-gen`

## Step 6: Emergency Revert

If the change fails verification and the fix is unclear:

```bash
cd $REPO_ROOT/grammar
git checkout -- grammar.js
tree-sitter generate    # MUST regenerate after revert
```

## Key Files

| Purpose | Path |
|---------|------|
| Grammar definition (source of truth) | `grammar/grammar.js` |
| Generated C parser (do not edit) | `grammar/src/parser.c` |
| Generated grammar JSON | `grammar/src/grammar.json` |
| Generated node types | `grammar/src/node-types.json` |
| Generated symbol sets | `grammar/src/generated_symbol_sets.js` |
| Highlight queries | `grammar/queries/highlights.scm` |
| Grammar tests (generated) | `grammar/test/corpus/` |
| Symbol registry | `spec/symbols/symbol_registry.json` |
| Reference corpus (sacred) | `corpus/reference/` (74 files) |
| Parser equivalence tests | `crates/talkbank-parser-tests/` |
