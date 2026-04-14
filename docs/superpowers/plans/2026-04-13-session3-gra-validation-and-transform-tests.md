# Session 3: GRA Validation Activation + Transform Pipeline Tests

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Activate E720-E724 GRA validation through the full pipeline (code exists but test fixtures are broken) and bring transform pipeline test coverage from 19 to 50+ tests.

**Architecture:** 
- Track 1 (GRA): The validation logic for E721-E724 already exists in `validate_gra_structure()` and IS wired into the utterance validation pipeline. But the error corpus test fixtures have syntactically invalid %gra lines (`.` terminators that the parser rejects as E316 before validation runs). Fix the fixtures, update spec status, regenerate tests. E720 needs error code investigation (emits E713 instead).
- Track 2 (Transform): New integration test file for `talkbank-transform` covering pipeline happy/error paths, cache behavior, JSON roundtrip, streaming, and corpus discovery.

**Tech Stack:** Rust, CHAT format, `make test-gen` for spec regeneration

**Reference spec:** `docs/superpowers/specs/2026-04-13-public-release-improvements-design.md` — Section 3b (Priority 1 error specs) and Section 6h (Transform pipeline tests).

**Key discovery:** E721-E724 are marked "not_implemented" in specs, but the Rust validation code is fully implemented and unit-tested. The problem is that the error corpus `.cha` test files have `.` terminators on %gra lines, causing parse failures (E316) before the validator runs. This is a fixture bug, not a code bug.

---

### Task 1: Fix E721 error corpus fixture

**Files:**
- Modify: `tests/error_corpus/validation_errors/not_implemented/E721_gra_non_sequential.cha`

- [ ] **Step 1: Read the current fixture**

```bash
cat tests/error_corpus/validation_errors/not_implemented/E721_gra_non_sequential.cha
```

- [ ] **Step 2: Understand the problem**

The %gra line likely ends with ` .` (a terminator), which the tree-sitter grammar doesn't expect on %gra tiers. The parser emits E316 (unparsable dependent tier) before the validator can check E721 (non-sequential indices).

Fix: Remove the `. ` terminator from the %gra line. Valid %gra format is tab-separated `index|head|RELATION` entries with no terminator.

Also: The indices must actually be non-sequential to trigger E721. Verify the fixture has indices like (1, 3, 2) instead of (1, 2, 3).

- [ ] **Step 3: Fix the fixture**

Edit the %gra line to remove the terminator. Example of correct non-sequential %gra:
```
%gra:	1|2|SUBJ 3|0|ROOT 2|3|OBJ
```

- [ ] **Step 4: Test that E721 is now emitted**

```bash
cargo run -p talkbank-cli -- validate tests/error_corpus/validation_errors/not_implemented/E721_gra_non_sequential.cha --tui-mode disable 2>&1
```

Expected: Should show `E721` error (non-sequential indices), NOT E316.

If it still shows E316, the fixture has other parse issues. Read the error and fix accordingly.

- [ ] **Step 5: Commit**

```bash
git add tests/error_corpus/validation_errors/not_implemented/E721_gra_non_sequential.cha
git commit -m "fix(test): fix E721 error corpus fixture — remove invalid %gra terminator"
```

---

### Task 2: Fix E722 error corpus fixture

**Files:**
- Modify: `tests/error_corpus/validation_errors/not_implemented/E722_gra_no_root.cha`

Same pattern as Task 1. The %gra line has a `. ` terminator causing E316.

- [ ] **Step 1: Read and fix the fixture**

Remove the terminator from the %gra line. Ensure the %gra relations have no ROOT (no relation with head=0 or head=self).

- [ ] **Step 2: Test that E722 is now emitted**

```bash
cargo run -p talkbank-cli -- validate tests/error_corpus/validation_errors/not_implemented/E722_gra_no_root.cha --tui-mode disable 2>&1
```

Expected: Should show `E722` warning (no ROOT), NOT E316.

- [ ] **Step 3: Commit**

```bash
git add tests/error_corpus/validation_errors/not_implemented/E722_gra_no_root.cha
git commit -m "fix(test): fix E722 error corpus fixture — remove invalid %gra terminator"
```

---

### Task 3: Fix E723 error corpus fixture

**Files:**
- Modify: `tests/error_corpus/validation_errors/not_implemented/E723_gra_multiple_roots.cha`

- [ ] **Step 1: Read and fix the fixture**

Remove the terminator. Ensure the %gra has multiple ROOT relations (multiple relations with head=self or head=0).

- [ ] **Step 2: Test that E723 is now emitted**

```bash
cargo run -p talkbank-cli -- validate tests/error_corpus/validation_errors/not_implemented/E723_gra_multiple_roots.cha --tui-mode disable 2>&1
```

Expected: Should show `E723` warning (multiple ROOTs).

- [ ] **Step 3: Commit**

```bash
git add tests/error_corpus/validation_errors/not_implemented/E723_gra_multiple_roots.cha
git commit -m "fix(test): fix E723 error corpus fixture — remove invalid %gra terminator"
```

---

### Task 4: Create E724 error corpus fixture

**Files:**
- Create: `tests/error_corpus/validation_errors/not_implemented/E724_gra_circular_dependency.cha`

No fixture exists yet for E724. Create one with a circular dependency in %gra.

- [ ] **Step 1: Create the fixture**

```chat
@UTF8
@Begin
@Languages:	eng
@Participants:	CHI Child
@ID:	eng|corpus|CHI|||||Child|||
@Comment:	Expected error: E724 (GRA circular dependency)
*CHI:	I want cookie .
%mor:	pro|I v|want n|cookie .
%gra:	1|2|SUBJ 2|3|OBJ 3|1|NMOD
@End
```

This creates a cycle: 1→2→3→1. None of the relations has head=0 or head=self, so there's no ROOT (E722 will also fire). If you want ONLY E724, add a ROOT and create a cycle among the non-root nodes:

```chat
%gra:	1|0|ROOT 2|3|SUBJ 3|2|OBJ
```

This creates: 1→ROOT (valid), 2→3→2 (cycle).

- [ ] **Step 2: Test that E724 is emitted**

```bash
cargo run -p talkbank-cli -- validate tests/error_corpus/validation_errors/not_implemented/E724_gra_circular_dependency.cha --tui-mode disable 2>&1
```

Expected: Should show `E724` warning (circular dependency).

- [ ] **Step 3: Commit**

```bash
git add tests/error_corpus/validation_errors/not_implemented/E724_gra_circular_dependency.cha
git commit -m "test: add E724 error corpus fixture (circular dependency)"
```

---

### Task 5: Investigate E720 error code mismatch

**Files:**
- Investigate: `crates/talkbank-model/src/alignment/gra/align.rs`
- Investigate: `tests/error_corpus/E4xx_alignment_errors/E720_mor_gra_count_mismatch.cha`

E720 (MorGraCountMismatch) is supposed to fire when %mor chunk count differs from %gra relation count. But the current test fixture emits E713 instead. This needs investigation.

- [ ] **Step 1: Run the existing E720 fixture**

```bash
cargo run -p talkbank-cli -- validate tests/error_corpus/E4xx_alignment_errors/E720_mor_gra_count_mismatch.cha --tui-mode disable 2>&1
```

Note what error codes are emitted.

- [ ] **Step 2: Read the alignment code**

Read `crates/talkbank-model/src/alignment/gra/align.rs` — specifically the function that detects mor/gra count mismatch. Find what error code it currently emits and whether E720 is used or if E713 covers this case.

- [ ] **Step 3: Read the E720 spec**

Read `spec/errors/E720_auto.md` to understand what's expected.

- [ ] **Step 4: Determine the fix**

Options:
a) If E713 is the correct code for this case (and E720 is a duplicate), update the spec
b) If E720 should be emitted instead of E713, change the error code in the alignment code
c) If both should exist for different cases, create separate fixtures

- [ ] **Step 5: Report findings**

Do NOT make the fix without reporting first. This is an investigation task. Report what you found and recommend a fix. The user will decide.

---

### Task 6: Update spec status for E721-E724

After Tasks 1-4 confirm the errors fire correctly through the pipeline, update the spec files.

**Files:**
- Modify: `spec/errors/E721_auto.md`
- Modify: `spec/errors/E722_auto.md`
- Modify: `spec/errors/E723_auto.md`
- Modify: `spec/errors/E724_gra_circular_dependency.md`

- [ ] **Step 1: For each spec file, change status**

Change:
```
- **Status**: not_implemented
```
to:
```
- **Status**: implemented
```

ONLY change the status line. Do not modify anything else in the spec files.

- [ ] **Step 2: Move fixture files out of not_implemented/**

The error corpus fixtures should move from `validation_errors/not_implemented/` to a non-not_implemented location. Check the directory structure:

```bash
ls tests/error_corpus/
```

Determine the correct destination directory (e.g., `validation_errors/` or `E7xx_tier_validation/`).

Move the files:
```bash
git mv tests/error_corpus/validation_errors/not_implemented/E721_gra_non_sequential.cha tests/error_corpus/validation_errors/
git mv tests/error_corpus/validation_errors/not_implemented/E722_gra_no_root.cha tests/error_corpus/validation_errors/
git mv tests/error_corpus/validation_errors/not_implemented/E723_gra_multiple_roots.cha tests/error_corpus/validation_errors/
git mv tests/error_corpus/validation_errors/not_implemented/E724_gra_circular_dependency.cha tests/error_corpus/validation_errors/
```

- [ ] **Step 3: Regenerate tests**

```bash
make test-gen
```

This regenerates the generated error tests. The E721-E724 tests should now be non-ignored (since status changed from not_implemented to implemented).

- [ ] **Step 4: Run the regenerated tests**

```bash
cargo nextest run --test generated_error_tests -E 'test(E72)' 2>&1
```

If the generated tests don't match this pattern, try:
```bash
cargo nextest run -p talkbank-parser-tests -E 'test(E72)' 2>&1
```

Expected: Tests for E721-E724 should pass (they were `#[ignore]` before, now they run).

- [ ] **Step 5: Commit**

```bash
git add spec/errors/ tests/error_corpus/ tests/ crates/talkbank-parser-tests/tests/generated/
git commit -m "feat(validation): activate E721-E724 GRA validation — fix fixtures, update spec status"
```

---

### Task 7: Transform pipeline tests — pipeline happy paths (15 tests)

**Files:**
- Create: `crates/talkbank-transform/tests/pipeline_tests.rs`

Create comprehensive integration tests for the pipeline module. Use `tempfile` for temporary directories.

- [ ] **Step 1: Read existing tests for patterns**

Read:
- `crates/talkbank-transform/src/pipeline/parse.rs` (existing tests at bottom)
- `crates/talkbank-transform/src/pipeline/convert.rs` (existing tests at bottom)

Understand the test patterns: how they create CHAT content strings, call pipeline functions, and assert results.

- [ ] **Step 2: Create the test file with 15 tests**

The test file should cover:

**Parse & validate (5 tests):**
1. `parse_valid_minimal_chat` — minimal valid file, no errors
2. `parse_with_validation_options` — alignment on vs off produces different results
3. `parse_invalid_missing_end` — missing @End triggers validation error
4. `parse_invalid_missing_begin` — missing @Begin triggers validation error
5. `parse_empty_content` — empty string returns error (not panic)

**Streaming parse (3 tests):**
6. `streaming_parse_collects_errors` — errors reported via ErrorSink, file still returned
7. `streaming_parse_valid_no_errors` — valid file produces no errors in sink
8. `streaming_parse_with_parser_reuse` — create parser once, parse multiple files

**File I/O (3 tests):**
9. `parse_file_valid` — read from temp file, validate
10. `parse_file_not_found` — nonexistent path returns IO error
11. `parse_file_empty` — empty file returns parse error (not panic)

**Conversion (4 tests):**
12. `chat_to_json_valid` — produces valid JSON, passes schema validation
13. `chat_to_json_compact_vs_pretty` — compact is shorter, both parse identically
14. `chat_to_json_unvalidated` — skip schema check, still produces JSON
15. `normalize_chat_idempotent` — normalize twice produces same output

Use this CHAT content for valid tests:
```rust
const VALID_CHAT: &str = "@UTF8\n@Begin\n@Languages:\teng\n@Participants:\tCHI Child\n@ID:\teng|corpus|CHI|||||Child|||\n*CHI:\thello world .\n%mor:\tn|hello n|world .\n@End\n";
```

- [ ] **Step 3: Run the tests**

```bash
cargo nextest run -p talkbank-transform --test pipeline_tests
```

Expected: 15 tests pass.

- [ ] **Step 4: Commit**

```bash
git add crates/talkbank-transform/tests/pipeline_tests.rs
git commit -m "test(transform): add 15 pipeline integration tests"
```

---

### Task 8: Transform pipeline tests — cache behavior (12 tests)

**Files:**
- Create: `crates/talkbank-transform/tests/cache_tests.rs`

Test the unified cache using in-memory mode (`CachePool::in_memory()`).

- [ ] **Step 1: Read the cache API**

Read:
- `crates/talkbank-transform/src/unified_cache/cache_impl.rs`
- `crates/talkbank-transform/src/validation_runner/cache.rs` (ValidationCache trait)

- [ ] **Step 2: Create the test file with 12 tests**

**Basic cache operations (6 tests):**
1. `cache_set_and_get_valid` — set validation=valid, get returns valid
2. `cache_set_and_get_invalid` — set validation=invalid, get returns invalid
3. `cache_miss_returns_none` — uncached path returns None
4. `cache_alignment_flag_is_key` — same path with alignment=true vs false are separate entries
5. `cache_roundtrip_parser_kind_is_key` — same path with different parser_kind are separate
6. `cache_clear_all` — after clear_all, get returns None

**Maintenance (3 tests):**
7. `cache_clear_prefix` — clear entries matching a prefix
8. `cache_stats_count` — stats reflect number of entries
9. `cache_purge_nonexistent` — purge entries for deleted files

**Edge cases (3 tests):**
10. `cache_overwrite_entry` — setting same key twice overwrites
11. `cache_empty_path` — empty path doesn't crash
12. `cache_concurrent_access` — two CachePool instances on same in-memory DB don't corrupt

- [ ] **Step 3: Run the tests**

```bash
cargo nextest run -p talkbank-transform --test cache_tests
```

Expected: 12 tests pass.

- [ ] **Step 4: Commit**

```bash
git add crates/talkbank-transform/tests/cache_tests.rs
git commit -m "test(transform): add 12 cache behavior tests"
```

---

### Task 9: Transform pipeline tests — JSON and rendering (10 tests)

**Files:**
- Create: `crates/talkbank-transform/tests/json_tests.rs`

- [ ] **Step 1: Read the JSON module**

Read `crates/talkbank-transform/src/json.rs` fully.

- [ ] **Step 2: Create the test file with 10 tests**

**Schema validation (4 tests):**
1. `schema_is_available` — `is_schema_validation_available()` returns true
2. `valid_json_passes_schema` — a ChatFile serialized to JSON passes schema validation
3. `invalid_json_fails_schema` — `{"not": "a chat file"}` fails schema validation
4. `schema_load_error_is_none` — no load error when schema is embedded

**Serialization variants (3 tests):**
5. `to_json_pretty_has_newlines` — pretty output contains newlines
6. `to_json_unvalidated_skips_schema` — produces JSON even if schema would reject
7. `validate_json_string_on_valid` — round-trip: serialize then validate string

**Error rendering (3 tests):**
8. `render_error_includes_code` — rendered error string contains the error code
9. `render_error_with_source_includes_line` — rendered error includes source line
10. `render_error_with_named_source_includes_filename` — rendered error includes filename

- [ ] **Step 3: Run the tests**

```bash
cargo nextest run -p talkbank-transform --test json_tests
```

Expected: 10 tests pass.

- [ ] **Step 4: Commit**

```bash
git add crates/talkbank-transform/tests/json_tests.rs
git commit -m "test(transform): add 10 JSON and rendering tests"
```

---

### Task 10: Transform pipeline tests — streaming validation and corpus (13 tests)

**Files:**
- Create: `crates/talkbank-transform/tests/validation_runner_tests.rs`

- [ ] **Step 1: Read the validation runner**

Read:
- `crates/talkbank-transform/src/validation_runner/runner.rs`
- `crates/talkbank-transform/src/validation_runner/types.rs`
- `crates/talkbank-transform/src/validation_runner/config.rs`
- `crates/talkbank-transform/src/corpus/mod.rs`

- [ ] **Step 2: Create the test file with 13 tests**

**ValidationConfig (3 tests):**
1. `default_config` — default values are sensible (alignment=true, cache=enabled, recursive)
2. `config_jobs_none_means_all_cpus` — None jobs = num_cpus default
3. `config_parser_kind_default` — default is TreeSitter

**ValidationStats (4 tests):**
4. `stats_initial_zero` — fresh stats have all zeros
5. `stats_record_valid` — recording valid increments counter
6. `stats_record_invalid` — recording invalid increments counter
7. `stats_snapshot_consistent` — snapshot captures atomic state correctly

**Streaming validation (3 tests):**
8. `validate_directory_valid_corpus` — temp dir with valid .cha files → all pass
9. `validate_directory_invalid_file` — temp dir with invalid .cha → errors reported
10. `validate_directory_empty` — empty dir → Started{0}, Finished immediately

**Corpus module (3 tests):**
11. `build_manifest_from_directory` — temp dir with .cha files → manifest with entries
12. `build_manifest_empty_dir` — empty dir → empty manifest
13. `corpus_summary_format` — summary string includes file count

Use `tempfile::tempdir()` for all directory tests. Write minimal .cha files to the temp dir.

- [ ] **Step 3: Run the tests**

```bash
cargo nextest run -p talkbank-transform --test validation_runner_tests
```

Expected: 13 tests pass.

- [ ] **Step 4: Commit**

```bash
git add crates/talkbank-transform/tests/validation_runner_tests.rs
git commit -m "test(transform): add 13 streaming validation and corpus tests"
```

---

### Task 11: Final verification

- [ ] **Step 1: Run all transform tests**

```bash
cargo nextest run -p talkbank-transform
```

Expected: 19 existing + 50 new = 69+ tests pass.

- [ ] **Step 2: Run GRA-related tests**

```bash
cargo nextest run -p talkbank-model -E 'test(gra)'
```

Expected: All pass.

- [ ] **Step 3: Run make check**

```bash
make check
```

Expected: Clean pass.
