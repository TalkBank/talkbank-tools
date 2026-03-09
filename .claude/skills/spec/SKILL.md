---
name: spec
description: Add or modify a CHAT spec (construct or error). Use when the user wants to add a new CHAT language construct, add a new error code, or modify an existing spec file. Handles metadata, file creation, and test regeneration.
disable-model-invocation: true
allowed-tools: Bash, Read, Write, Edit, Glob, Grep, Agent
---

# Add or Modify a CHAT Spec

Guide the user through creating or editing a CHAT spec file and regenerating all derived artifacts.

## Step 1: Determine Spec Type

Ask the user (or infer from `$ARGUMENTS`):
- **Construct spec** → `spec/constructs/`
- **Error spec** → `spec/errors/`

## Step 2: Understand the Format

### Construct Spec (`spec/constructs/*.md`)

Read a few existing construct specs to understand the format:
```bash
ls $REPO_ROOT/spec/constructs/ | head -20
```
Then read one as a template. Construct specs contain CHAT code blocks with expected parse tree annotations.

### Error Spec (`spec/errors/*.md`)

Error specs have YAML frontmatter with these critical fields:

```yaml
---
Error Code: EXXX
Layer: parser | validation
Status: implemented | not_implemented
---
```

Key rules:
- `Layer: parser` → test expects `parse_chat_file()` to return `Err`
- `Layer: validation` → test uses streaming parse + `validate_with_alignment()` after successful parse
- `Status: not_implemented` → generates `#[ignore]` tests
- `Expected Error Codes` (per-example) → overrides the spec-level error code for that example
- Each code block tagged with `chat` is a test case

Read existing error specs to understand the pattern:
```bash
ls $REPO_ROOT/spec/errors/ | head -20
```

## Step 3: Create or Edit the Spec File

- Use the naming convention from existing files
- Ensure all frontmatter fields are correct
- For error specs, verify the error code doesn't collide with existing codes:
```bash
grep -r "Error Code:" $REPO_ROOT/spec/errors/ | sort
```

## Step 4: Regenerate All Derived Artifacts

This is **mandatory** after any spec change:

```bash
cd $REPO_ROOT && make test-gen
```

This regenerates three artifact sets:
1. Tree-sitter corpus tests (`grammar/test/corpus/`)
2. Rust integration tests (`crates/talkbank-parser-tests/tests/generated/`)
3. Error documentation (`docs/errors/`)

**Never hand-edit generated test files.** They will be overwritten by `make test-gen`.

## Step 5: Verify

Run the full verification gates:

```bash
cd $REPO_ROOT && make verify
```

This runs G0–G10:
- G0: cargo fmt
- G1: clippy
- G2: nextest (all crates)
- G3: doctests
- G4: parser-tests (reference corpus equivalence)
- G5-G10: various guardrails

If tests fail, the spec likely has an issue. Common problems:
- Wrong `Layer` value (parser vs validation)
- Missing terminator in CHAT examples
- Error code collision
- Malformed expected parse tree in construct specs

## Step 6: Report

Tell the user:
- What spec file was created/modified
- How many tests were generated
- Whether `make verify` passed
- If any downstream repos need attention (e.g., grammar changes needed)
