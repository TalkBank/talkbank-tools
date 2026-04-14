# Session 4: Cross-Utterance Validation Flag + Spec Cleanup

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Enable E351-E355 cross-utterance validation behind an opt-in CLI flag (`--strict-linkers`), and clean up 9 mystery specs + W724 dead code.

**Architecture:**
- Track 1: Wire the existing `enable_quotation_validation` flag through the CLI as `--strict-linkers` on `chatter validate`. Also wire `check_other_completion()` into the cross-utterance dispatch (currently dead code). Update specs to `implemented`. All validation code already exists and is unit-tested.
- Track 2: Update 9 auto-generated specs (E203, E230, E243, E253, E312, E315, E360, E364, E388) from `not_implemented` to `implemented` with real descriptions. Remove W724 dead code.

**Tech Stack:** Rust (clap, talkbank-model validation pipeline)

**Key discovery:** E351-E355 validation code is fully written and optimized (O(n)) but intentionally disabled since 2025-12-28. The `enable_quotation_validation` flag exists in `ValidationContext` — we just need to surface it as a CLI flag and wire the other-completion function call. The 9 "mystery" specs are all actually emitted by the parser/validator — they just have placeholder descriptions.

---

### Task 1: Add `--strict-linkers` flag to CLI

**Files:**
- Modify: `crates/talkbank-cli/src/cli/args/core.rs` (Validate subcommand)

- [ ] **Step 1: Read the current Validate subcommand args**

Read `crates/talkbank-cli/src/cli/args/core.rs`, specifically the `Validate` variant of the `Commands` enum (around lines 103-173).

- [ ] **Step 2: Add the --strict-linkers flag**

Add a new field to the `Validate` variant, after the `suppress` field:

```rust
        /// Enable strict cross-utterance linker validation (E351-E355).
        ///
        /// Checks that self-completion (+,) and other-completion (++)
        /// linkers are paired with the correct preceding terminators
        /// (+/. and +... respectively). Disabled by default because
        /// many existing corpora do not follow these strict conventions.
        #[arg(
            long = "strict-linkers",
            help = "Enable strict linker pairing validation (E351-E355)"
        )]
        strict_linkers: bool,
```

- [ ] **Step 3: Commit**

```bash
git add crates/talkbank-cli/src/cli/args/core.rs
git commit -m "feat(cli): add --strict-linkers flag to chatter validate"
```

---

### Task 2: Thread the flag through to validation

**Files:**
- Investigate and modify: the code path from CLI args → `ParseValidateOptions` → `ValidationContext`

- [ ] **Step 1: Trace the validation pipeline**

Find how `validate` command args flow into the validation. Read:
1. `crates/talkbank-cli/src/commands/validate.rs` or wherever `Commands::Validate` is handled
2. How `ParseValidateOptions` is constructed from CLI args
3. How `ValidationContext` receives the `enable_quotation_validation` flag

The key target: `crates/talkbank-model/src/validation/context.rs` has `enable_quotation_validation: bool` on `ValidationContext`. We need the CLI flag to set this to `true`.

- [ ] **Step 2: Thread the flag**

The exact wiring depends on what you find in Step 1. The pattern should be:
1. CLI `strict_linkers: bool` → passed into whatever config struct the validate command builds
2. That config flows into `ValidationContext::enable_quotation_validation`

If `ParseValidateOptions` doesn't have a field for this, you may need to add one. Check if there's already a mechanism for passing extra validation flags.

Read `crates/talkbank-model/src/pipeline/mod.rs` or `crates/talkbank-transform/src/pipeline/parse.rs` to see how `ParseValidateOptions` maps to `ValidationContext`.

- [ ] **Step 3: Verify the flag reaches the validation context**

After wiring, check that setting `--strict-linkers` on the CLI causes `context.shared.enable_quotation_validation` to be `true` in the validation pipeline.

- [ ] **Step 4: Commit**

```bash
git add -p  # Stage only the relevant changes
git commit -m "feat(validation): thread --strict-linkers flag through to ValidationContext"
```

---

### Task 3: Wire check_other_completion() into dispatch

**Files:**
- Modify: `crates/talkbank-model/src/validation/cross_utterance/mod.rs`

- [ ] **Step 1: Read the cross-utterance dispatch**

Read `crates/talkbank-model/src/validation/cross_utterance/mod.rs`. Find where `check_self_completion_all()` is called (gated by `enable_quotation_validation`). Around line 131-136.

Also find the comment about other-completion being disabled (around line 127-129).

- [ ] **Step 2: Add the other-completion call**

After the self-completion call, add:

```rust
// Other-completion linker (E353/E354/E355) - O(n) validation
if context.shared.enable_quotation_validation {
    for (i, utterance) in utterances.iter().enumerate() {
        completion::check_other_completion(utterance, i, utterances, errors);
    }
}
```

Read the `check_other_completion()` function signature to confirm the parameters are correct. It may take different arguments.

Also remove the `#[allow(dead_code)]` from `check_other_completion()` in `completion.rs`.

- [ ] **Step 3: Verify the unit tests pass**

```bash
cargo nextest run -p talkbank-model -E 'test(completion)'
```

The existing `#[ignore]`d tests should still be ignored (they test with the flag enabled via direct function calls, not through the pipeline).

- [ ] **Step 4: Commit**

```bash
git add crates/talkbank-model/src/validation/cross_utterance/
git commit -m "feat(validation): wire check_other_completion() into cross-utterance dispatch"
```

---

### Task 4: Write integration test for --strict-linkers

**Files:**
- Modify: `crates/talkbank-cli/tests/integration_tests.rs`

- [ ] **Step 1: Write a test that verifies E351 fires with --strict-linkers**

Add to integration_tests.rs:

```rust
#[test]
fn strict_linkers_flag_enables_e351() -> Result<(), TestError> {
    let dir = tempdir()?;
    let file = dir.path().join("test.cha");
    // Self-completion +, with no prior interruption → E351
    fs::write(&file, "@UTF8\n@Begin\n@Languages:\teng\n@Participants:\tCHI Child\n@ID:\teng|corpus|CHI|||||Child|||\n*CHI:\t+, hello world .\n@End\n")?;
    
    // Without --strict-linkers: should NOT report E351
    assert_cmd::cargo::cargo_bin_cmd!("chatter")
        .args(["validate", file.to_str().unwrap(), "--tui-mode", "disable"])
        .assert()
        .success();  // No E351 without flag
    
    // With --strict-linkers: should report E351
    assert_cmd::cargo::cargo_bin_cmd!("chatter")
        .args(["validate", file.to_str().unwrap(), "--tui-mode", "disable", "--strict-linkers"])
        .assert()
        .failure()
        .stderr(predicates::str::contains("E351"));
    
    Ok(())
}
```

IMPORTANT: Read how existing tests handle error output — errors may go to stdout, not stderr. Check by running a failing validate command and seeing which stream gets the error. Adjust the assertion accordingly.

Also: the `+,` linker syntax may need to be at the START of the utterance content, before any words. Read the E351 spec example to confirm exact syntax.

- [ ] **Step 2: Run the test**

```bash
cargo nextest run -p talkbank-cli -E 'test(strict_linkers)'
```

Expected: PASS — E351 fires only when --strict-linkers is present.

If the test fails, debug:
- Check if the flag reaches the validation context (add tracing)
- Check if the CHAT content actually triggers the linker detection
- Read the generated error test fixture for E351 to get known-good input

- [ ] **Step 3: Commit**

```bash
git add crates/talkbank-cli/tests/integration_tests.rs
git commit -m "test(cli): verify --strict-linkers enables E351 validation"
```

---

### Task 5: Update E351-E355 spec status

**Files:**
- Modify: `spec/errors/E351_auto.md`, `E352_auto.md`, `E353_auto.md`, `E354_auto.md`, `E355_auto.md`

- [ ] **Step 1: Update each spec**

For each of the 5 spec files, change:
```
- **Status**: not_implemented
```
to:
```
- **Status**: implemented
```

Do NOT change anything else.

- [ ] **Step 2: Move test fixtures out of not_implemented/ (if they exist there)**

Check if there are fixture files in `tests/error_corpus/validation_errors/not_implemented/` for E351-E355. If so, move them to `tests/error_corpus/validation_errors/`.

- [ ] **Step 3: Regenerate tests**

```bash
make test-gen
```

The E351-E355 generated tests will now be non-`#[ignore]`. But they may fail because they run without `--strict-linkers` (the generated test framework uses default validation options).

Check: does the generated test framework support setting `enable_quotation_validation: true`? If not, the generated tests may need to stay `#[ignore]` with a different annotation, or the test harness needs updating.

**If generated tests fail:** This is expected — they run through the standard pipeline which has the flag off by default. Report this as a finding. The specs are correctly `implemented` (the code works), but the generated test harness doesn't support flag-gated validation yet. The unit tests in `cross_utterance/tests/` cover correctness.

- [ ] **Step 4: Un-ignore the unit tests**

In `crates/talkbank-model/src/validation/cross_utterance/tests/`:
- `self_completion.rs`: Remove `#[ignore]` from `test_e351_self_completion_no_preceding_utterance` and `test_e352_self_completion_wrong_terminator`
- `other_completion.rs`: Remove `#[ignore]` from `test_e353_other_completion_no_preceding`, `test_e354_other_completion_wrong_terminator`, `test_e355_other_completion_same_speaker`

These unit tests call the validation functions directly (not through the pipeline), so they don't depend on the CLI flag.

- [ ] **Step 5: Run the unit tests**

```bash
cargo nextest run -p talkbank-model -E 'test(e351) | test(e352) | test(e353) | test(e354) | test(e355)'
```

Expected: 5 tests pass (were previously ignored).

- [ ] **Step 6: Commit**

```bash
git add spec/errors/E35*.md crates/talkbank-model/src/validation/cross_utterance/tests/
git commit -m "feat(validation): activate E351-E355 cross-utterance validation behind --strict-linkers"
```

---

### Task 6: Remove W724 dead code

**Files:**
- Modify: `crates/talkbank-model/src/errors/codes/error_code.rs`
- Delete: `spec/errors/W724_auto.md`

- [ ] **Step 1: Read the W724 definition**

Find `GraRootHeadNotSelf` in `error_code.rs`. Confirm it's never emitted (grep the codebase).

- [ ] **Step 2: Remove the variant**

Delete the `GraRootHeadNotSelf` variant and its `#[code("W724")]` attribute from `error_code.rs`.

- [ ] **Step 3: Delete the spec file**

```bash
rm spec/errors/W724_auto.md
```

- [ ] **Step 4: Check for compile errors**

```bash
cargo check -p talkbank-model
```

If anything references `GraRootHeadNotSelf`, fix those references (likely only in tests).

- [ ] **Step 5: Regenerate tests**

```bash
make test-gen
```

- [ ] **Step 6: Commit**

```bash
git add crates/talkbank-model/src/errors/codes/error_code.rs
git rm spec/errors/W724_auto.md
git add tests/ crates/talkbank-parser-tests/tests/generated/
git commit -m "chore: remove W724 dead code (GraRootHeadNotSelf never emitted)"
```

---

### Task 7: Update 9 mystery specs to implemented

**Files:**
- Modify: `spec/errors/E203_auto.md`, `E230_auto.md`, `E243_auto.md`, `E253_auto.md`, `E312_auto.md`, `E315_auto.md`, `E360_auto.md`, `E364_auto.md`, `E388_auto.md`

All 9 are actively emitted by the parser/validator but have `Status: not_implemented` and placeholder descriptions.

- [ ] **Step 1: For each spec, change status**

Change `Status: not_implemented` to `Status: implemented` in all 9 files.

- [ ] **Step 2: For each spec, read the error code variant and write a real description**

For each error code, grep for the variant name in the codebase to understand what it actually detects. Then update the spec's description field. Here's the mapping:

| Code | Variant | What it detects |
|------|---------|-----------------|
| E203 | InvalidFormType | Invalid form type marker in word annotation |
| E230 | UnbalancedCADelimiter | Unbalanced CA (conversation analysis) delimiter |
| E243 | IllegalCharactersInWord | Illegal characters found in word content |
| E253 | EmptyWordContent | Word has empty text content |
| E312 | UnclosedBracket | Unclosed bracket in utterance |
| E315 | InvalidControlCharacter | Invalid control character in CHAT content |
| E360 | InvalidMediaBullet | Malformed media bullet (timing annotation) |
| E364 | MalformedWordContent | Malformed word content structure |
| E388 | ReplacementOnNonword | Replacement annotation applied to a non-word element |

Read the actual emission sites to refine these descriptions. The spec description should explain: what triggers the error, what the user should fix, and a brief example if possible.

- [ ] **Step 3: Regenerate tests**

```bash
make test-gen
```

The 9 generated tests will now be non-`#[ignore]`. They should already pass since the code is active.

- [ ] **Step 4: Run the regenerated tests**

```bash
cargo nextest run -p talkbank-parser-tests --test generated_error_tests 2>&1 | tail -10
```

Expected: All pass (these error codes are already emitted).

If any fail, it means the test fixture in the spec doesn't actually trigger the error — fix the fixture.

- [ ] **Step 5: Commit**

```bash
git add spec/errors/ crates/talkbank-parser-tests/tests/generated/
git commit -m "docs(specs): update 9 mystery specs to implemented with real descriptions"
```

---

### Task 8: Final verification

- [ ] **Step 1: Count remaining not_implemented specs**

```bash
grep -rl "Status.*not_implemented" spec/errors/ | wc -l
```

Before Session 3: 69. After E721-E724 (4) + E351-E355 (5) + mystery 9 + W724 removal = 69 - 19 = 50 remaining.

- [ ] **Step 2: Run workspace check**

```bash
make check
```

- [ ] **Step 3: Run the full model test suite**

```bash
cargo nextest run -p talkbank-model
```

- [ ] **Step 4: Verify --strict-linkers flag works end-to-end**

```bash
cargo run -p talkbank-cli -- validate --help 2>&1 | grep strict-linkers
```

Expected: Shows the `--strict-linkers` flag in help output.
