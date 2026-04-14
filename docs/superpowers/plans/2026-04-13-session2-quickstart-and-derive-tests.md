# Session 2: Quick-Start Guide + Derive Macro Tests

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Create a researcher-focused quick-start guide in the mdBook, and bring derive macro test coverage from 1 test to 30+ tests covering all 4 proc macros.

**Architecture:** Track 1 is a new markdown page in the book. Track 2 is Rust unit tests and trybuild UI tests in the `talkbank-derive` crate, using types defined locally in the test files (not importing heavy model types).

**Tech Stack:** Markdown (mdBook), Rust (proc-macro2, syn, quote, trybuild, standard test framework)

**Reference spec:** `docs/superpowers/specs/2026-04-13-public-release-improvements-design.md` — Section 5a (Quick-start guide) and Section 6g (Derive macro tests).

---

### Task 1: Create quick-start.md book page

**Files:**
- Create: `book/src/user-guide/quick-start.md`
- Modify: `book/src/SUMMARY.md` (add entry after Installation)

- [ ] **Step 1: Add SUMMARY.md entry**

In `book/src/SUMMARY.md`, find the line:
```
  - [Installation](user-guide/installation.md)
```

Insert immediately after it:
```
  - [Quick Start](user-guide/quick-start.md)
```

- [ ] **Step 2: Create quick-start.md**

Create `book/src/user-guide/quick-start.md` with this content:

```markdown
# Quick Start

**Status:** Current
**Last updated:** [RUN date '+%Y-%m-%d %H:%M %Z' AND INSERT RESULT]

This page gets you from zero to productive with `chatter` in five minutes.
[Install chatter first](installation.md) if you haven't already.

## Validate a CHAT file

Check a single transcript for errors:

```bash
chatter validate transcript.cha
```

If the file is valid:

```
✓ transcript.cha is valid
```

If there are problems, you'll see rich diagnostics with the exact location
and a stable error code:

```
  × error[E304]: missing speaker code on main tier line

   ╭─[transcript.cha:6:1]
 6 │ *	hello world .
   ·  ╰── expected speaker code (e.g., *CHI:)
   ╰────
  help: A main tier line must start with *SPEAKER:\t
```

Every error code (`E304`, `E705`, etc.) links to
[documentation with fix guidance](validation-errors.md).

## Validate an entire corpus

Point `chatter` at a directory — it walks recursively, validates in parallel,
and caches results:

```bash
chatter validate corpus/
```

The interactive TUI shows progress and lets you browse errors per file.
Use `--format json` for machine-readable output, or `--quiet` for CI
(exit code 1 on errors).

## Run an analysis

`chatter` includes 80 CLAN analysis commands. Try frequency analysis:

```bash
chatter clan freq transcript.cha
```

Output shows word frequencies by speaker. Add filters:

```bash
chatter clan freq transcript.cha --speaker CHI    # one speaker
chatter clan mlu transcript.cha --speaker CHI     # mean length of utterance
chatter clan combo transcript.cha --include-word "want"  # co-occurrence
```

All CLAN commands support `--format json` and `--format csv` for
downstream processing.

## Convert to JSON

Get a structured representation of any CHAT file:

```bash
chatter to-json transcript.cha
```

The output conforms to the [TalkBank CHAT JSON Schema](https://talkbank.org/schemas/v0.1/chat-file.json).
Convert back with `chatter from-json`.

## Watch for changes

Edit a file and get live validation feedback:

```bash
chatter watch transcript.cha
```

Every time you save, `chatter` re-validates and shows updated diagnostics.

## What next?

- **[CLI Reference](cli-reference.md)** — all commands, flags, and output formats
- **[Validation Errors](validation-errors.md)** — all 198 error codes with examples
- **[VS Code Extension](vscode-extension.md)** — live diagnostics, CLAN commands, media playback
- **[Migrating from CLAN](migrating-from-clan.md)** — flag mapping for CLAN veterans
- **[Batch Workflows](batch-workflows.md)** — corpus-scale validation and analysis
```

- [ ] **Step 3: Update the timestamp**

Run `date '+%Y-%m-%d %H:%M %Z'` and replace the placeholder in quick-start.md.

- [ ] **Step 4: Build the book to verify**

```bash
cd book && mdbook build 2>&1 | tail -5
```

Expected: No errors. Verify the quick-start page appears in the build output.

- [ ] **Step 5: Commit**

```bash
git add book/src/user-guide/quick-start.md book/src/SUMMARY.md
git commit -m "docs: add quick-start guide to mdBook"
```

---

### Task 2: SemanticEq derive macro tests (8 tests)

**Files:**
- Create: `crates/talkbank-derive/tests/semantic_eq_tests.rs`
- Existing UI tests: `crates/talkbank-derive/tests/ui/pass_semantic_eq_basic.rs`, `crates/talkbank-derive/tests/ui/fail_semantic_eq_on_union.rs`

The SemanticEq derive generates `semantic_eq(&self, other: &Self) -> bool` that compares fields recursively, honoring `#[semantic_eq(skip)]`. We need to test it with local types (not model imports) to keep the test fast and self-contained.

- [ ] **Step 1: Create the test file with all 8 tests**

Create `crates/talkbank-derive/tests/semantic_eq_tests.rs`:

```rust
//! Tests for the SemanticEq derive macro.

use talkbank_derive::SemanticEq;

// --- Test types (local to this test file) ---

/// Trait definition needed for derive output.
/// The real trait lives in talkbank-model, but we define a minimal version
/// here to keep tests self-contained and fast.
trait SemanticEq {
    fn semantic_eq(&self, other: &Self) -> bool;
}

#[derive(Debug, talkbank_derive::SemanticEq)]
struct Simple {
    name: String,
    value: i32,
}

#[derive(Debug, talkbank_derive::SemanticEq)]
struct WithSkip {
    name: String,
    #[semantic_eq(skip)]
    cached_hash: u64,
    #[semantic_eq(skip)]
    parse_order: usize,
}

#[derive(Debug, talkbank_derive::SemanticEq)]
struct Empty;

#[derive(Debug, talkbank_derive::SemanticEq)]
struct AllSkipped {
    #[semantic_eq(skip)]
    a: i32,
    #[semantic_eq(skip)]
    b: String,
}

#[derive(Debug, talkbank_derive::SemanticEq)]
struct TupleStruct(String, i32);

#[derive(Debug, talkbank_derive::SemanticEq)]
enum Shape {
    Circle { radius: f64 },
    Rectangle { width: f64, height: f64 },
    Point,
}

#[derive(Debug, talkbank_derive::SemanticEq)]
struct Nested {
    inner: Simple,
    label: String,
}

// --- Tests ---

#[test]
fn simple_struct_equal() {
    let a = Simple { name: "hello".into(), value: 42 };
    let b = Simple { name: "hello".into(), value: 42 };
    assert!(a.semantic_eq(&b));
}

#[test]
fn simple_struct_not_equal() {
    let a = Simple { name: "hello".into(), value: 42 };
    let b = Simple { name: "hello".into(), value: 99 };
    assert!(!a.semantic_eq(&b));
}

#[test]
fn skip_fields_ignored() {
    let a = WithSkip { name: "x".into(), cached_hash: 111, parse_order: 0 };
    let b = WithSkip { name: "x".into(), cached_hash: 999, parse_order: 50 };
    assert!(a.semantic_eq(&b), "skipped fields should not affect equality");
}

#[test]
fn skip_fields_still_checks_non_skipped() {
    let a = WithSkip { name: "x".into(), cached_hash: 111, parse_order: 0 };
    let b = WithSkip { name: "y".into(), cached_hash: 111, parse_order: 0 };
    assert!(!a.semantic_eq(&b), "non-skipped field 'name' differs");
}

#[test]
fn empty_struct_always_equal() {
    assert!(Empty.semantic_eq(&Empty));
}

#[test]
fn all_skipped_always_equal() {
    let a = AllSkipped { a: 1, b: "one".into() };
    let b = AllSkipped { a: 2, b: "two".into() };
    assert!(a.semantic_eq(&b), "all fields skipped → always equal");
}

#[test]
fn enum_same_variant_equal() {
    let a = Shape::Circle { radius: 3.14 };
    let b = Shape::Circle { radius: 3.14 };
    assert!(a.semantic_eq(&b));
}

#[test]
fn enum_different_variant_not_equal() {
    let a = Shape::Circle { radius: 3.14 };
    let b = Shape::Point;
    assert!(!a.semantic_eq(&b));
}
```

**IMPORTANT:** Before writing this file, check how the real `SemanticEq` trait is defined in `talkbank-model`. The derive macro generates code that calls `self.field.semantic_eq(&other.field)` — so the trait must be in scope. Read `crates/talkbank-model/src/model/semantic_eq.rs` (or wherever the trait is defined) to understand the exact trait signature. You may need to either:
- Import `talkbank_model::SemanticEq` (if talkbank-model is already a dev-dependency), OR
- Define a compatible local trait

Check `crates/talkbank-derive/Cargo.toml` dev-dependencies — `talkbank-model` IS listed there, so you can import the real trait.

- [ ] **Step 2: Run the tests to verify they pass**

```bash
cargo nextest run -p talkbank-derive -E 'test(semantic_eq)'
```

Expected: 8 tests pass. If any fail, read the compiler error — it likely means the trait import path is wrong. Fix and re-run.

- [ ] **Step 3: Commit**

```bash
git add crates/talkbank-derive/tests/semantic_eq_tests.rs
git commit -m "test(derive): add 8 SemanticEq derive macro tests"
```

---

### Task 3: SpanShift derive macro tests (8 tests)

**Files:**
- Create: `crates/talkbank-derive/tests/span_shift_tests.rs`

SpanShift generates `shift_spans_after(&mut self, offset: u32, delta: i32)` which shifts all `Span` fields >= offset by delta. It recurses into `Vec<T>`, `Option<T>`, and nested types.

- [ ] **Step 1: Create the test file**

Create `crates/talkbank-derive/tests/span_shift_tests.rs`. You need to:

1. First read `crates/talkbank-model/src/model/span.rs` to understand the `Span` type and its `shift_spans_after` behavior
2. Read `crates/talkbank-model/src/model/mod.rs` to find the `SpanShift` trait definition
3. Define test structs that use `Span` fields (import from talkbank-model)
4. Write these 8 tests:

| Test | What it checks |
|------|---------------|
| `shift_span_at_offset` | Span starting at offset gets shifted |
| `no_shift_before_offset` | Span before offset is untouched |
| `shift_positive_delta` | Insertion: spans move forward |
| `shift_negative_delta` | Deletion: spans move backward |
| `option_span_some` | `Option<Span>` with Some value shifts |
| `option_span_none` | `Option<Span>` with None is no-op |
| `vec_recursion` | `Vec<T>` where T has spans — all elements shifted |
| `skip_attribute` | `#[span_shift(skip)]` field not shifted |

Each test: create struct, call `shift_spans_after(offset, delta)`, assert span values changed (or not).

- [ ] **Step 2: Run the tests**

```bash
cargo nextest run -p talkbank-derive -E 'test(span_shift)'
```

Expected: 8 tests pass.

- [ ] **Step 3: Commit**

```bash
git add crates/talkbank-derive/tests/span_shift_tests.rs
git commit -m "test(derive): add 8 SpanShift derive macro tests"
```

---

### Task 4: ValidationTagged derive macro tests (5 tests)

**Files:**
- Create: `crates/talkbank-derive/tests/validation_tagged_tests.rs`

ValidationTagged generates `validation_tag(&self) -> ValidationTag` that maps enum variants to Clean/Warning/Error based on explicit annotations or naming conventions (`*Error` suffix → Error, `*Warning`/`Unsupported` → Warning, default → Clean).

- [ ] **Step 1: Create the test file**

Create `crates/talkbank-derive/tests/validation_tagged_tests.rs`. You need to:

1. Read how `ValidationTag` and the `ValidationTagged` trait are defined in talkbank-model
2. Define a test enum with variants covering all resolution rules:

```rust
#[derive(Debug, talkbank_derive::ValidationTagged)]
enum Status {
    // Explicit annotations
    #[validation_tag(error)]
    ExplicitError,
    #[validation_tag(warning)]
    ExplicitWarning,
    #[validation_tag(clean)]
    ExplicitClean,

    // Naming conventions
    ParseError,           // *Error suffix → Error
    AlignmentWarning,     // *Warning suffix → Warning
    Unsupported,          // exact "Unsupported" → Warning
    FormatUnsupported,    // *Unsupported suffix → Warning
    Valid,                // no match → Clean
}
```

Write these 5 tests:

| Test | What it checks |
|------|---------------|
| `explicit_annotation_overrides` | ExplicitError/Warning/Clean return correct tags |
| `error_suffix_convention` | `ParseError` variant detected as Error |
| `warning_suffix_convention` | `AlignmentWarning` detected as Warning |
| `unsupported_convention` | Both `Unsupported` and `*Unsupported` detected as Warning |
| `helper_methods` | `is_validation_error()`, `is_validation_warning()`, `has_validation_issue()` |

- [ ] **Step 2: Run the tests**

```bash
cargo nextest run -p talkbank-derive -E 'test(validation_tagged)'
```

Expected: 5 tests pass.

- [ ] **Step 3: Commit**

```bash
git add crates/talkbank-derive/tests/validation_tagged_tests.rs
git commit -m "test(derive): add 5 ValidationTagged derive macro tests"
```

---

### Task 5: error_code_enum attribute macro tests (4 tests)

**Files:**
- Create: `crates/talkbank-derive/tests/error_code_enum_tests.rs`

The `#[error_code_enum]` attribute macro generates `as_str()`, `new()`, `Display`, `documentation_url()`, and serde impls for error code enums with `#[code("E###")]` per variant.

- [ ] **Step 1: Create the test file**

Create `crates/talkbank-derive/tests/error_code_enum_tests.rs`:

1. Read `crates/talkbank-derive/src/error_code_enum.rs` to understand exactly what's generated
2. Define a small test enum:

```rust
#[talkbank_derive::error_code_enum]
enum TestErrorCode {
    #[code("E001")]
    FirstError,
    #[code("E002")]
    SecondError,
    #[code("E999")]
    UnknownError,
}
```

Write these 4 tests:

| Test | What it checks |
|------|---------------|
| `as_str_returns_code` | `TestErrorCode::FirstError.as_str() == "E001"` |
| `new_parses_known_code` | `TestErrorCode::new("E001") == TestErrorCode::FirstError` |
| `new_unknown_code_returns_unknown` | `TestErrorCode::new("E999") == TestErrorCode::UnknownError` and `TestErrorCode::new("EXXX") == TestErrorCode::UnknownError` |
| `display_shows_code` | `format!("{}", TestErrorCode::FirstError) == "E001"` |

- [ ] **Step 2: Run the tests**

```bash
cargo nextest run -p talkbank-derive -E 'test(error_code_enum)'
```

Expected: 4 tests pass.

- [ ] **Step 3: Commit**

```bash
git add crates/talkbank-derive/tests/error_code_enum_tests.rs
git commit -m "test(derive): add 4 error_code_enum attribute macro tests"
```

---

### Task 6: Trybuild compile-fail tests (5 new tests)

**Files:**
- Create: 5 new files in `crates/talkbank-derive/tests/ui/`
- Modify: `crates/talkbank-derive/tests/ui_tests.rs` (if needed to add new test patterns)

Currently there is 1 fail test (`fail_semantic_eq_on_union.rs`). Add 5 more:

- [ ] **Step 1: Read the existing trybuild setup**

Read `crates/talkbank-derive/tests/ui_tests.rs` to understand the test runner pattern. Also read the existing `fail_semantic_eq_on_union.rs` and its `.stderr` companion.

- [ ] **Step 2: Create 5 new fail test files**

Each file goes in `crates/talkbank-derive/tests/ui/`:

1. **`fail_span_shift_on_union.rs`** — `#[derive(SpanShift)]` on a union (should fail)
2. **`fail_validation_tagged_on_struct.rs`** — `#[derive(ValidationTagged)]` on a struct (should fail, enum only)
3. **`fail_error_code_enum_missing_unknown.rs`** — `#[error_code_enum]` without `UnknownError` variant
4. **`fail_error_code_enum_missing_code_attr.rs`** — `#[error_code_enum]` variant without `#[code(...)]`
5. **`fail_error_code_enum_non_unit_variant.rs`** — `#[error_code_enum]` with tuple variant

For each file, write the Rust source that should fail to compile, then run the trybuild tests to capture the actual compiler error, and save it as the `.stderr` companion file.

**Process per file:**
1. Write the `.rs` file with the invalid derive usage
2. Run `cargo test -p talkbank-derive ui` — trybuild will show the actual error
3. If the error message is correct (describes the problem clearly), bless the output:
   `TRYBUILD=overwrite cargo test -p talkbank-derive ui` to generate the `.stderr` files
4. Review the `.stderr` files to confirm they contain helpful error messages

- [ ] **Step 3: Verify all UI tests pass (including existing ones)**

```bash
cargo test -p talkbank-derive ui
```

Expected: All pass (existing 3 + new 5 = 8 UI tests).

- [ ] **Step 4: Commit**

```bash
git add crates/talkbank-derive/tests/ui/
git commit -m "test(derive): add 5 compile-fail trybuild tests"
```

---

### Task 7: Nested and complex SemanticEq tests (5 bonus tests)

**Files:**
- Modify: `crates/talkbank-derive/tests/semantic_eq_tests.rs`

Add more complex scenarios to the existing test file:

- [ ] **Step 1: Add 5 more tests**

| Test | What it checks |
|------|---------------|
| `nested_struct_equality` | Struct containing another SemanticEq struct — both equal |
| `nested_struct_inner_differs` | Inner struct field differs → not equal |
| `tuple_struct_equal` | `TupleStruct("a".into(), 1).semantic_eq(&TupleStruct("a".into(), 1))` |
| `tuple_struct_not_equal` | Positional field differs → not equal |
| `enum_unit_variants_equal` | `Shape::Point.semantic_eq(&Shape::Point)` |

- [ ] **Step 2: Run tests**

```bash
cargo nextest run -p talkbank-derive -E 'test(semantic_eq)'
```

Expected: 13 tests pass (8 original + 5 new).

- [ ] **Step 3: Commit**

```bash
git add crates/talkbank-derive/tests/semantic_eq_tests.rs
git commit -m "test(derive): add 5 nested/complex SemanticEq tests"
```

---

### Task 8: Final verification

- [ ] **Step 1: Run all derive crate tests**

```bash
cargo nextest run -p talkbank-derive
cargo test -p talkbank-derive ui
```

Expected: 30+ tests pass (13 SemanticEq + 8 SpanShift + 5 ValidationTagged + 4 error_code_enum + UI tests).

- [ ] **Step 2: Run workspace check**

```bash
make check
```

Expected: Clean compilation across the workspace.

- [ ] **Step 3: Verify book builds**

```bash
cd book && mdbook build 2>&1 | grep -i error
```

Expected: No errors.
