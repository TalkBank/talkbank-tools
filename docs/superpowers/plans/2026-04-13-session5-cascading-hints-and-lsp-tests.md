# Session 5: Cascading Error Hints + LSP Golden Tests

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add "fix structural errors first" hints to chatter validate output, and bring LSP test coverage from 136 to 170+ tests by covering 4 untested features.

**Architecture:**
- Track 1: Add a post-validation hint in the CLI output layer when structural errors (E1xx-E4xx) are present alongside missing semantic/alignment checks. ~30 lines of new code in the output renderer. TDD.
- Track 2: Add unit tests to 4 untested LSP features (Completion, Semantic Tokens, Document Highlight, Formatting) plus expand 2 minimally-tested features (Linked Editing, Workspace Symbol). Tests use existing patterns: parse CHAT string → invoke feature function → assert results.

**Tech Stack:** Rust (clap output, talkbank-lsp feature handlers)

---

### Task 1: Cascading error hints — write failing test

**Files:**
- Modify: `crates/talkbank-cli/tests/integration_tests.rs`

- [ ] **Step 1: Read the existing validate output tests**

Read `crates/talkbank-cli/tests/integration_tests.rs` to find tests that check validate output. Understand how error output is captured and asserted.

- [ ] **Step 2: Write a test for the cascading hint**

Add a test that validates a file with structural errors and checks for the hint message:

```rust
#[test]
fn cascading_error_hint_shown_for_structural_errors() -> Result<(), TestError> {
    let dir = tempdir()?;
    let file = dir.path().join("structural.cha");
    // File missing @End — structural error that prevents full validation
    fs::write(&file, "@UTF8\n@Begin\n@Languages:\teng\n@Participants:\tCHI Child\n@ID:\teng|corpus|CHI|||||Child|||\n*CHI:\thello world .\n")?;

    let output = assert_cmd::cargo::cargo_bin_cmd!("chatter")
        .args(["validate", file.to_str().unwrap(), "--tui-mode", "disable"])
        .output()?;

    let combined = format!("{}{}", 
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr));
    
    assert!(combined.contains("additional checks were skipped"),
        "should show cascading hint when structural errors present");
    Ok(())
}

#[test]
fn no_cascading_hint_for_valid_file() -> Result<(), TestError> {
    let dir = tempdir()?;
    let file = dir.path().join("valid.cha");
    fs::write(&file, "@UTF8\n@Begin\n@Languages:\teng\n@Participants:\tCHI Child\n@ID:\teng|corpus|CHI|||||Child|||\n*CHI:\thello world .\n%mor:\tn|hello n|world .\n@End\n")?;

    let output = assert_cmd::cargo::cargo_bin_cmd!("chatter")
        .args(["validate", file.to_str().unwrap(), "--tui-mode", "disable"])
        .output()?;

    let combined = format!("{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr));
    
    assert!(!combined.contains("additional checks were skipped"),
        "valid file should not show cascading hint");
    Ok(())
}
```

Adjust the assertion target (stdout vs stderr) based on how existing validate tests check output.

- [ ] **Step 3: Run tests to verify they fail**

```bash
cargo nextest run -p talkbank-cli -E 'test(cascading)'
```

Expected: `cascading_error_hint_shown_for_structural_errors` FAILS (hint not yet implemented). `no_cascading_hint_for_valid_file` should PASS (no hint = correct).

- [ ] **Step 4: Commit the failing test**

```bash
git add crates/talkbank-cli/tests/integration_tests.rs
git commit -m "test(cli): add failing test for cascading error hints"
```

---

### Task 2: Implement cascading error hints

**Files:**
- Modify: `crates/talkbank-cli/src/commands/validate/output.rs` (or wherever per-file validation results are rendered)

- [ ] **Step 1: Read the output rendering code**

Read `crates/talkbank-cli/src/commands/validate/output.rs`. Find where per-file error output is written. The hint should appear AFTER all errors for a file, BEFORE the next file.

Also check `crates/talkbank-cli/src/commands/validate_parallel/renderer.rs` for directory-mode output.

- [ ] **Step 2: Implement the hint logic**

After errors are printed for a file, analyze the error codes:
1. Check if any errors have codes in E1xx-E4xx range (structural/header/utterance errors)
2. Check if the file has NO errors in E7xx range (alignment) — meaning alignment checks may have been skipped due to parse health tainting
3. If structural errors present AND alignment errors absent, emit:

```
  note: Some alignment checks may not have run because of structural errors above.
        Fix the structural errors first, then re-validate.
```

Use the same output mechanism (eprintln, miette, or whatever the existing error output uses).

The hint should:
- Only appear once per file (not per error)
- Only appear when structural errors are present
- Not appear when the file has alignment errors too (validation ran fully)
- Respect `--quiet` mode (suppress hint)
- Respect `--format json` (include as a field, not text)

- [ ] **Step 3: Run the tests**

```bash
cargo nextest run -p talkbank-cli -E 'test(cascading)'
```

Expected: Both tests pass.

- [ ] **Step 4: Commit**

```bash
git add crates/talkbank-cli/src/commands/validate/
git commit -m "feat(cli): add cascading error hint when structural errors skip alignment checks"
```

---

### Task 3: LSP Completion tests (10 tests)

**Files:**
- Modify: `crates/talkbank-lsp/src/backend/features/completion.rs` (add #[cfg(test)] mod tests)

- [ ] **Step 1: Read the completion handler**

Read `crates/talkbank-lsp/src/backend/features/completion.rs` fully. Understand what completions are offered and when.

- [ ] **Step 2: Write 10 tests**

Add a `#[cfg(test)] mod tests` block. Follow existing patterns from other feature files (e.g., hover.rs, document_symbol.rs). Tests should cover:

1. Speaker code completion after `*` — should suggest declared participants
2. Tier prefix completion after `%` — should suggest %mor, %gra, %pho, etc.
3. Completion with @Participants context — suggestions match declared speakers
4. Completion without @Participants — still offers common speakers (CHI, MOT)
5. Header completion at line start — @Begin, @End, @Languages, etc.
6. No completion in middle of word — should return None
7. No completion in @End line — nothing to complete
8. Bracket annotation completion after `[` — [//], [/], [*], etc.
9. Empty document completion — should not crash
10. Multi-speaker document — all speakers offered

If the completion handler doesn't support some of these, write tests that document current behavior (what IS returned, even if empty).

- [ ] **Step 3: Run tests**

```bash
cargo nextest run -p talkbank-lsp -E 'test(completion)'
```

- [ ] **Step 4: Commit**

```bash
git add crates/talkbank-lsp/src/backend/features/completion.rs
git commit -m "test(lsp): add 10 completion handler tests"
```

---

### Task 4: LSP Semantic Tokens tests (8 tests)

**Files:**
- Modify: `crates/talkbank-lsp/src/backend/requests/semantic_tokens.rs` (or appropriate file)

- [ ] **Step 1: Read the semantic tokens handler**

Read the semantic tokens implementation. Understand what token types are assigned.

- [ ] **Step 2: Write 8 tests**

1. Full document tokens — basic CHAT file returns token list
2. Header tokens — @UTF8, @Begin get header token type
3. Speaker tokens — *CHI: gets speaker token type
4. Tier label tokens — %mor: gets tier-label token type
5. Word tokens — content words get word token type
6. Range tokens — request tokens for specific range only
7. Empty document — returns empty token list
8. File with errors — still returns tokens for parseable content

- [ ] **Step 3: Run tests and commit**

```bash
cargo nextest run -p talkbank-lsp -E 'test(semantic_token)'
git add crates/talkbank-lsp/src/backend/requests/semantic_tokens.rs
git commit -m "test(lsp): add 8 semantic token tests"
```

---

### Task 5: LSP Document Highlight tests (6 tests)

**Files:**
- Modify: `crates/talkbank-lsp/src/backend/features/highlights/mod.rs` (or relevant file)

- [ ] **Step 1: Read the highlight handler**

- [ ] **Step 2: Write 6 tests**

1. Highlight speaker — cursor on CHI highlights all CHI utterances
2. Highlight word — cursor on "hello" highlights all occurrences
3. No highlight on whitespace — returns empty
4. No highlight on terminator — returns empty
5. Multiple speakers — only matching speaker highlighted
6. Highlight in tier — %mor word highlighted

- [ ] **Step 3: Run tests and commit**

```bash
cargo nextest run -p talkbank-lsp -E 'test(highlight)'
git add crates/talkbank-lsp/src/backend/features/highlights/
git commit -m "test(lsp): add 6 document highlight tests"
```

---

### Task 6: LSP Formatting tests (6 tests)

**Files:**
- Modify: `crates/talkbank-lsp/src/backend/requests/formatting.rs` (or relevant file)

- [ ] **Step 1: Read the formatting handler**

- [ ] **Step 2: Write 6 tests**

1. Format full document — canonical tab/spacing applied
2. Format range — only specified range formatted
3. Already-formatted document — no changes (empty edit list)
4. Multiple spacing issues — all fixed in one pass
5. Preserve content — formatting doesn't change semantics
6. Empty document — returns empty edit list

- [ ] **Step 3: Run tests and commit**

```bash
cargo nextest run -p talkbank-lsp -E 'test(formatting)'
git add crates/talkbank-lsp/src/backend/requests/formatting.rs
git commit -m "test(lsp): add 6 formatting tests"
```

---

### Task 7: Expand Linked Editing + Workspace Symbol tests (7 tests)

**Files:**
- Modify: `crates/talkbank-lsp/src/backend/features/linked_editing.rs`
- Modify: `crates/talkbank-lsp/src/backend/features/workspace_symbol.rs`

- [ ] **Step 1: Read both handlers**

- [ ] **Step 2: Write 4 linked editing tests**

1. Speaker rename preparation — cursor on speaker returns all speaker ranges
2. Multiple occurrences — all instances returned
3. Invalid cursor position (on whitespace) — returns None
4. Non-renameable content (on terminator) — returns None

- [ ] **Step 3: Write 3 workspace symbol tests**

1. Query speaker code — returns matching symbols
2. Query empty string — returns all symbols
3. Case sensitivity — verify behavior

- [ ] **Step 4: Run tests and commit**

```bash
cargo nextest run -p talkbank-lsp -E 'test(linked_editing) | test(workspace_symbol)'
git add crates/talkbank-lsp/src/backend/features/linked_editing.rs crates/talkbank-lsp/src/backend/features/workspace_symbol.rs
git commit -m "test(lsp): expand linked editing (4) and workspace symbol (3) tests"
```

---

### Task 8: Final verification

- [ ] **Step 1: Run all LSP tests**

```bash
cargo nextest run -p talkbank-lsp
```

Expected: 136 existing + ~37 new = 170+ tests pass.

- [ ] **Step 2: Run CLI tests**

```bash
cargo nextest run -p talkbank-cli -E 'test(cascading)'
```

Expected: Both cascading hint tests pass.

- [ ] **Step 3: Workspace check**

```bash
make check
```
