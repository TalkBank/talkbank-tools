# Session 1: CLAUDE.md Accuracy Sweep + README Download Visibility

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Fix all CLAUDE.md inconsistencies across the repo and improve README installation UX for researchers discovering the project.

**Architecture:** Pure documentation changes — no code modifications. Two parallel tracks: (1) fix 6 CLAUDE.md accuracy issues, (2) improve README download visibility and CLI help text. One code change: add a `Getting Started` hint to the CLI help epilog.

**Tech Stack:** Markdown, Rust (clap attributes for CLI help text)

**Reference spec:** `docs/superpowers/specs/2026-04-13-public-release-improvements-design.md` — Section 1 (CLAUDE.md Accuracy Sweep) and Section 2a-2b (Installation UX).

---

### Task 1: Fix reference corpus count in grammar/CLAUDE.md

**Files:**
- Modify: `grammar/CLAUDE.md:22`

- [ ] **Step 1: Fix the stale count**

Change line 22 from:

```
The 74-file reference corpus in `corpus/reference/` must remain at 100% parser equivalence.
```

to:

```
The 88-file reference corpus in `corpus/reference/` must remain at 100% parser equivalence.
```

- [ ] **Step 2: Update the Last modified timestamp**

Run:
```bash
date '+%Y-%m-%d %H:%M %Z'
```

Update line 3 of `grammar/CLAUDE.md` with the output timestamp.

- [ ] **Step 3: Commit**

```bash
git add grammar/CLAUDE.md
git commit -m "docs: fix reference corpus count in grammar/CLAUDE.md (74 → 88)"
```

---

### Task 2: Fix reference corpus count in re2c parser CLAUDE.md

**Files:**
- Modify: `crates/talkbank-re2c-parser/CLAUDE.md:3,167,238`

- [ ] **Step 1: Fix the two stale counts**

Change line 167 from:

```
- **Model study:** `tests/model_study.rs` — reference corpus equivalence (87 files). All pass except 2 CA files with known raw_text divergences in the subtoken word path.
```

to:

```
- **Model study:** `tests/model_study.rs` — reference corpus equivalence (88 files). All pass except 2 CA files with known raw_text divergences in the subtoken word path.
```

Change line 238 from:

```
All 87 files validate and roundtrip successfully with `--parser re2c`.
```

to:

```
All 88 files validate and roundtrip successfully with `--parser re2c`.
```

- [ ] **Step 2: Update the Last modified timestamp**

Run:
```bash
date '+%Y-%m-%d %H:%M %Z'
```

Change line 3 from `**Last modified:** 2026-03-31 06:41 EDT` to the current timestamp.

- [ ] **Step 3: Commit**

```bash
git add crates/talkbank-re2c-parser/CLAUDE.md
git commit -m "docs: fix reference corpus count in re2c CLAUDE.md (87 → 88)"
```

---

### Task 3: Fix parser equivalence test count in root CLAUDE.md

**Files:**
- Modify: `CLAUDE.md:336`

The root CLAUDE.md says "Both must show `73 passed, 0 failed`" but actual count is 93 parser equivalence tests.

- [ ] **Step 1: Verify the actual test count**

Run:
```bash
cargo nextest list -p talkbank-parser-tests -E 'test(parser_equivalence)' 2>/dev/null | grep 'test$' | wc -l
```

Expected output: `93` (88 file-based + 5 offset/structural tests)

- [ ] **Step 2: Fix the stale count**

Change line 336 from:

```
3. Both must show `73 passed, 0 failed` before any commit.
```

to:

```
3. Both must show `93 passed, 0 failed` before any commit.
```

- [ ] **Step 3: Update the Last modified timestamp**

Run:
```bash
date '+%Y-%m-%d %H:%M %Z'
```

Update line 3 of `CLAUDE.md` with the output timestamp.

- [ ] **Step 4: Commit**

```bash
git add CLAUDE.md
git commit -m "docs: fix parser equivalence test count in root CLAUDE.md (73 → 93)"
```

---

### Task 4: Remove stale line counts from vscode/CLAUDE.md

**Files:**
- Modify: `vscode/CLAUDE.md:25-26`

Line counts rot when files grow. Replace specific counts with durable references.

- [ ] **Step 1: Remove the line counts**

Change lines 25-26 from:

```
├── DEVELOPER.md               # 603-line architecture guide (read this first)
├── GUIDE.md                   # 509-line user guide
```

to:

```
├── DEVELOPER.md               # Architecture guide (read this first)
├── GUIDE.md                   # User guide
```

- [ ] **Step 2: Update the Last modified timestamp**

Run:
```bash
date '+%Y-%m-%d %H:%M %Z'
```

Update the `Last modified` line in `vscode/CLAUDE.md` with the output timestamp.

- [ ] **Step 3: Commit**

```bash
git add vscode/CLAUDE.md
git commit -m "docs: remove stale line counts from vscode/CLAUDE.md"
```

---

### Task 5: Add parity status to desktop/CLAUDE.md

**Files:**
- Modify: `desktop/CLAUDE.md:10`

The opening mandate claims "full functional parity" but status tables show ~70%. Add a current-status line so readers know the gap immediately.

- [ ] **Step 1: Add current parity status after the mandate**

After line 10 (the mandate paragraph), insert a blank line and then:

```
**Current parity status: ~70%.** Progress throttling and some navigation keybindings are not yet implemented. See the feature parity tables below for details.
```

- [ ] **Step 2: Update the Last modified timestamp**

Run:
```bash
date '+%Y-%m-%d %H:%M %Z'
```

Update the `Last modified` line in `desktop/CLAUDE.md` with the output timestamp.

- [ ] **Step 3: Commit**

```bash
git add desktop/CLAUDE.md
git commit -m "docs: add current parity status (~70%) to desktop/CLAUDE.md"
```

---

### Task 6: Investigate and handle stale experiment CLAUDE.md

**Files:**
- Investigate: `experiments/multi-root-grammar/variant/CLAUDE.md`

This file's timestamp is 2026-03-14 — 30 days behind the rest of the repo. Determine if the experiment is concluded or ongoing.

- [ ] **Step 1: Check git activity in the experiment directory**

Run:
```bash
git log --oneline -5 -- experiments/multi-root-grammar/
```

If there are no commits in the last 30 days, the experiment is concluded.

- [ ] **Step 2: If concluded — archive the findings**

Read `experiments/multi-root-grammar/variant/CLAUDE.md` fully. If the experiment has conclusions, add a one-paragraph summary to the top of the file:

```
> **Status: Concluded.** This experiment explored multi-root grammar variants.
> Findings have been incorporated into the main grammar. This directory is
> preserved for reference but is not actively maintained.
```

Update the timestamp.

If the experiment is ongoing (recent commits exist), just update the timestamp.

- [ ] **Step 3: Commit**

```bash
git add experiments/multi-root-grammar/
git commit -m "docs: update stale experiment CLAUDE.md (multi-root-grammar)"
```

---

### Task 7: Document tree-sitter-grammar-utils limitation in CONTRIBUTING.md

**Files:**
- Modify: `CONTRIBUTING.md` (after line 9, in the repository structure section)

External contributors cannot regenerate `generated_traversal.rs` without an unpublished tool. This must be stated clearly.

- [ ] **Step 1: Add a note about the external dependency**

After line 9 (`The grammar lives in `grammar/`.`) in `CONTRIBUTING.md`, insert:

```markdown

### External Dependency Note

The file `crates/talkbank-parser-tests/src/generated_traversal.rs` is generated
by [`tree-sitter-grammar-utils`](https://github.com/TalkBank/tree-sitter-grammar-utils),
which is not yet published. If your changes require regenerating this file
(i.e., you modified `grammar/grammar.js` in a way that changes the CST node
types), note this in your PR and a maintainer will regenerate it.

Most contributions (spec changes, validation logic, CLAN commands, CLI features)
do not require this step.
```

- [ ] **Step 2: Commit**

```bash
git add CONTRIBUTING.md
git commit -m "docs: note tree-sitter-grammar-utils limitation in CONTRIBUTING.md"
```

---

### Task 8: Improve README download visibility

**Files:**
- Modify: `README.md:14-20`

The Install section exists but is minimal. Researchers need platform-specific download guidance at a glance.

- [ ] **Step 1: Expand the Install section**

Replace lines 14-20 (the current Install section):

```markdown
## Install

Download a pre-built binary from [GitHub Releases](https://github.com/TalkBank/talkbank-tools/releases), or build from source:

```bash
cargo install --path crates/talkbank-cli
```
```

with:

```markdown
## Install

### Pre-built binaries (recommended)

Download `chatter` for your platform from **[GitHub Releases](https://github.com/TalkBank/talkbank-tools/releases)**:

| Platform | Binary |
|----------|--------|
| macOS (Apple Silicon) | `chatter-aarch64-apple-darwin.tar.gz` |
| macOS (Intel) | `chatter-x86_64-apple-darwin.tar.gz` |
| Linux (x86_64) | `chatter-x86_64-unknown-linux-gnu.tar.gz` |
| Linux (ARM) | `chatter-aarch64-unknown-linux-gnu.tar.gz` |
| Windows (x86_64) | `chatter-x86_64-pc-windows-msvc.zip` |

Extract and add to your PATH, then verify: `chatter --version`

### Build from source

Requires [Rust](https://rustup.rs/) (stable):

```bash
cargo install --path crates/talkbank-cli
```
```

- [ ] **Step 2: Verify the binary archive names match what CI actually produces**

Run:
```bash
ls -1 .github/workflows/ | head -5
```

Then read the release workflow to confirm the archive naming pattern matches the table above. If the names differ, update the table to match.

- [ ] **Step 3: Commit**

```bash
git add README.md
git commit -m "docs: expand Install section with platform-specific download table"
```

---

### Task 9: Add first-run guidance to CLI help text

**Files:**
- Modify: `crates/talkbank-cli/src/cli/args/core.rs:10`

When a researcher runs bare `chatter`, the help text should include a "try this" hint.

- [ ] **Step 1: Write a test for the help text content**

Add this test to `crates/talkbank-cli/tests/integration_tests.rs`, in the test section (near the bottom, with other CLI tests):

```rust
#[test]
fn help_text_includes_getting_started() -> Result<(), TestError> {
    assert_cmd::cargo::cargo_bin_cmd!("chatter")
        .arg("--help")
        .assert()
        .success()
        .stdout(predicates::str::contains("Getting started"));
    Ok(())
}
```

This follows the existing test pattern used throughout this file (`assert_cmd::cargo::cargo_bin_cmd!`, `predicates`).

- [ ] **Step 2: Run the test to verify it fails**

Run:
```bash
cargo nextest run -p talkbank-cli -E 'test(help_text_includes_getting_started)'
```

Expected: FAIL — the current help text does not contain "Getting started".

- [ ] **Step 3: Add the after_long_help text to the CLI struct**

In `crates/talkbank-cli/src/cli/args/core.rs`, change line 10 from:

```rust
#[command(about = "Tools for validating and transforming TalkBank CHAT files", long_about = None)]
```

to:

```rust
#[command(
    about = "Tools for validating and transforming TalkBank CHAT files",
    long_about = None,
    after_long_help = "\
Getting started:
  chatter validate myfile.cha          Validate a CHAT file
  chatter validate corpus/             Validate an entire corpus
  chatter clan freq myfile.cha         Run frequency analysis
  chatter to-json myfile.cha           Convert to JSON

Full documentation: https://talkbank.org/tools/"
)]
```

- [ ] **Step 4: Run the test to verify it passes**

Run:
```bash
cargo nextest run -p talkbank-cli -E 'test(help_text_includes_getting_started)'
```

Expected: PASS

- [ ] **Step 5: Verify visually**

Run:
```bash
cargo run -p talkbank-cli -- --help
```

Verify the "Getting started" section appears at the bottom of the help output.

- [ ] **Step 6: Commit**

```bash
git add crates/talkbank-cli/src/cli/args/core.rs crates/talkbank-cli/tests/integration_tests.rs
git commit -m "feat(cli): add Getting Started hint to chatter --help"
```

---

### Task 10: Final verification

- [ ] **Step 1: Verify all CLAUDE.md files have consistent corpus counts**

Run:
```bash
rg '(74|87|73).file' grammar/CLAUDE.md crates/talkbank-re2c-parser/CLAUDE.md CLAUDE.md
```

Expected: No matches for 74-file or 87-file or 73-file. All references should say 88 or 93 as appropriate.

- [ ] **Step 2: Verify the CLI compiles and tests pass**

Run:
```bash
cargo check -p talkbank-cli && cargo nextest run -p talkbank-cli --lib
```

Expected: All pass, no warnings.

- [ ] **Step 3: Verify the README renders correctly**

Run:
```bash
head -40 README.md
```

Visually confirm the platform table looks correct in markdown.

- [ ] **Step 4: Run make check as a broader gate**

Run:
```bash
make check
```

Expected: Clean pass — no compilation errors across the workspace.
