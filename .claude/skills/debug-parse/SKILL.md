---
name: debug-parse
description: Diagnose CHAT parsing and validation errors. Use when a .cha file doesn't parse correctly, validation gives wrong errors, or the two parsers disagree.
disable-model-invocation: true
allowed-tools: Bash, Read, Glob, Grep, Agent
---

# Diagnose CHAT Parsing/Validation Errors

Investigate a CHAT file that parses incorrectly or triggers wrong validation errors. `$ARGUMENTS` can be a file path or error code.

## Step 1: Isolate the Layer

CHAT errors come from two distinct layers:
- **Parser layer** (E2xx, E3xx, E4xx) — syntax errors caught during parsing
- **Validation layer** (E5xx, E6xx, E7xx) — semantic errors caught after successful parse

```bash
# Full validation (both layers)
cd $REPO_ROOT
cargo run -p talkbank-cli -- validate <file.cha>

# Parse only (compare both parsers)
cd $REPO_ROOT
cargo run -p talkbank-parser-tests --bin compare_parsers -- <file.cha>
```

## Step 2: Look Up the Error Code

```bash
# Find error definition
grep -n "<ERROR_CODE>" $REPO_ROOT/crates/talkbank-model/src/errors/codes/error_code.rs

# Find spec for this error
ls $REPO_ROOT/spec/errors/ | grep -i "<ERROR_CODE>"

# Read the spec
cat $REPO_ROOT/spec/errors/<ERROR_CODE>*.md
```

## Step 3: Create a Minimal Reproducer

Reduce the file to the smallest example that triggers the error:

```bash
cat > /tmp/minimal.cha << 'EOF'
@UTF8
@Begin
@Languages: eng
@Participants: CHI Child
@ID: eng|test|CHI|||||Child|||
*CHI: hello .
@End
EOF

cd $REPO_ROOT
cargo run -p talkbank-cli -- validate /tmp/minimal.cha
```

Add back lines from the original file until the error appears. The last addition is the trigger.

## Step 4: Check Parser Agreement

```bash
cd $REPO_ROOT
cargo run -p talkbank-parser-tests --bin compare_parsers -- <file.cha>
```

This outputs side-by-side JSON from both parsers. If they disagree:
- Check `tests/error_corpus/expectations.json` for documented divergences
- The tree-sitter parser is canonical — if it's wrong, the grammar needs fixing (`/grammar`)
- The direct parser is experimental — divergences may be acceptable

## Step 5: Check Reference Corpus

Does the reference corpus have a similar valid pattern?
```bash
grep -r "<pattern>" $REPO_ROOT/corpus/reference/ --include="*.cha" | head -5
```

- If reference corpus has similar file that works → user's file has a data quality issue
- If reference corpus doesn't have similar pattern → may need to add one
- If reference corpus file also fails → parser/validation bug

Run full corpus check:
```bash
cd $REPO_ROOT
cargo nextest run -p talkbank-parser-tests -E 'test(parser_equivalence)'
# Must be: 73 passed, 0 failed
```

## Step 6: Roundtrip Test

Check parse → serialize → reparse → serialize idempotency:
```bash
cd $REPO_ROOT
cargo run -p talkbank-cli -- validate --roundtrip <file.cha>
```

If roundtrip fails but parse succeeds → serialization bug in the transform crate.

## Step 7: Diagnosis Decision Tree

| Finding | Likely Cause | Action |
|---------|-------------|--------|
| Minimal case parses fine | Original file has different issue | Expand minimal case incrementally |
| Parse fails, should succeed | Parser bug | Check grammar, file bug |
| Parse succeeds, validation wrong | Validator bug or spec gap | Check spec, may need new error spec |
| Tree-sitter != direct parser | Known divergence? | Check expectations.json |
| Roundtrip fails | Serialization bug | File bug against talkbank-transform |
| Error is correct but user disagrees | CHAT spec question | Check CHAT manual at talkbank.org |

## Step 8: Fix Workflow

### If it's a parser bug:
Use `/grammar` skill to fix grammar.js.

### If it's a missing validation rule:
Use `/spec` skill to add an error spec, then implement the validation.

### If it's a wrong validation rule:
1. Find the validation code:
```bash
grep -rn "<ERROR_CODE>" $REPO_ROOT/crates/talkbank-model/src/validation/
```
2. Fix the validation logic
3. Update the spec if needed
4. Run `make verify`

## Key Files

| Purpose | Path |
|---------|------|
| Error codes (all ~100+) | `crates/talkbank-model/src/errors/codes/error_code.rs` |
| Validation trait impls | `crates/talkbank-model/src/validation/` |
| Tree-sitter parser | `crates/talkbank-parser/src/lib.rs` |
| Direct parser | `crates/talkbank-direct-parser/src/` |
| Parser equivalence tests | `crates/talkbank-parser-tests/tests/parser_equivalence_files.rs` |
| Error specs | `spec/errors/` |
| Legacy error corpus | `tests/error_corpus/` |
| Expectations manifest | `tests/error_corpus/expectations.json` |
| Reference corpus (sacred) | `corpus/reference/` (74 files, 100% pass required) |
| Pipeline orchestration | `crates/talkbank-transform/src/pipeline/` |
