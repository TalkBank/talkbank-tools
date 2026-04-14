# Public Release Improvements: CLI/Tool Adoption

**Status:** Draft
**Last updated:** 2026-04-13 11:00 EDT
**Target audience:** Researchers and linguists downloading `chatter` binaries and using the CLI + VS Code extension

## Context

talkbank-tools is a public repository containing the unified TalkBank CHAT toolchain. Before announcing for broad adoption, a ruthless audit identified improvements across code, docs, CLAUDE.md accuracy, testing, and user-facing experience. The codebase is already A- quality; this plan polishes it for researcher adoption.

**Approach:** Two parallel tracks — user-facing improvements and internal quality — interleaved across sessions so both advance simultaneously.

**Binary architecture decision:** Two binaries, clean separation:
- **`chatter`** — everything a researcher types: validate, clan (80 commands), normalize, lint, watch, to-json/from-json, show-alignment, clean, new-file, cache, schema, debug
- **`talkbank-lsp`** — standalone LSP binary for VS Code, never invoked directly by users. Added as `[[bin]]` in `crates/talkbank-lsp/Cargo.toml`. Drops ~306 transitive deps the LSP never uses (ratatui, crossterm, notify, send2clan-sys, clap). ~6-8 MB vs 17 MB for full chatter. `chatter lsp` subcommand removed.

---

## Section 1: CLAUDE.md Accuracy Sweep

Every inconsistency degrades AI-assisted sessions. Six fixes:

### 1a. Reference corpus count mismatch (CRITICAL)
- `grammar/CLAUDE.md` says "74-file reference corpus" — actual is 88
- `crates/talkbank-re2c-parser/CLAUDE.md` says "87 files" — actual is 88
- Root CLAUDE.md says "88" — correct
- **Fix:** Update grammar/ and re2c/ CLAUDE.md to say 88

### 1b. Parser equivalence test count
- Root CLAUDE.md says "Both must show `73 passed, 0 failed`"
- Likely stale (was 73 when corpus was 74)
- **Fix:** Verify actual test count, update root CLAUDE.md

### 1c. VS Code CLAUDE.md stale line counts
- Claims DEVELOPER.md is 603 lines — actual is 775
- Claims GUIDE.md is 509 lines — actual is 1,116
- **Fix:** Remove specific line counts (they rot quickly). Replace with "see X for details"

### 1d. Desktop CLAUDE.md parity claim
- Opens with "must achieve full functional parity" but status tables show ~70% parity
- **Fix:** Add "Current parity status: ~70%" near the top, after the mandate

### 1e. Stale timestamps
- `crates/talkbank-re2c-parser/CLAUDE.md`: 2026-03-31 (12 days behind)
- `experiments/multi-root-grammar/variant/CLAUDE.md`: 2026-03-14 (abandoned?)
- **Fix:** Update re2c timestamp; investigate whether multi-root-grammar variant should be archived

### 1f. tree-sitter-grammar-utils dependency
- grammar/CLAUDE.md requires an unpublished private tool to regenerate `generated_traversal.rs`
- External contributors cannot perform this step
- **Fix:** Document this limitation prominently in CONTRIBUTING.md. Long-term: publish the tool (not in this plan).

---

## Section 2: Installation UX

### 2a. README download visibility
- Binary download link is buried — researchers must navigate to GitHub Releases
- **Fix:** Add a "Download" section near the top of README with direct links to latest releases for macOS (ARM + Intel), Linux, Windows. Badge or simple platform table.

### 2b. First-run guidance
- Bare `chatter` shows help text but no "try this first" hint
- **Fix:** Add Getting Started hint to help epilog: `Try: chatter validate myfile.cha` and `Try: chatter clan freq myfile.cha`

### 2c. Exit codes documentation
- Exit codes (0 = success, 1 = errors) implied but not documented
- **Fix:** Document in book's CLI reference page and in `chatter --help` epilog

### 2d. Platform-specific installers
- `.dmg`, `.msi`, Homebrew formula — significant packaging work
- **Deferred.** Not in this plan. GitHub Releases binaries are sufficient for initial release.

### 2e. VS Code extension marketplace
- Currently sideloaded via `.vsix`
- **Fix:** Addressed in Section 7 (marketplace publishing).

---

## Section 3: Error Output Quality

### 3a. Cascading error hints — "fix X first"
When validation short-circuits due to structural errors, semantic checks are skipped silently. Researchers see generic errors and wonder why deeper checks didn't run.

- **Fix:** When validation short-circuits, append: `"Note: N additional checks were skipped because of structural errors above. Fix these first, then re-validate."`
- Requires tracking which validation phases were skipped and why — change to the validation pipeline.

### 3b. Implement Priority 1 error specs (HIGH impact, 17 specs)

| Group | Codes | What it enables |
|-------|-------|----------------|
| GRA validation | E720, E721, E722, E723, E724 | Dependency tree structure: count mismatch, non-sequential indices, missing ROOT, multiple ROOTs, circular deps |
| Cross-utterance | E351, E352, E353, E354, E355 | Completion linkers (`+,`, `++`, `+/`), trailing-off — requires enabling the disabled `quotation_validation` subsystem |
| Tier alignment | E600, E711 | %mor tier: skipped alignment, empty mor content |
| Parser recovery | E316, E321, E323 | Specific error codes instead of generic fallback for unparsable content |
| GRA parsing | E708, E710 | Malformed %gra relation format detection |

Each spec already has a definition in `spec/errors/`. Work: TDD — failing test first, then implementation.

### 3c. Implement Priority 2 error specs (MEDIUM impact, 11 specs)

| Group | Codes | What it enables |
|-------|-------|----------------|
| Tier parsing | E381, E384 | %pho and %sin tier parse errors |
| Word validation | E232, E242, E245, E246 | Compound markers, quotation marks, stress/lengthening placement |
| CA notation | E356, E357, E373 | Underline balance, overlap index range |
| Overlap | E704 | Speaker self-overlap detection |

### 3d. Error documentation alignment
- 69 specs describe checks that don't fire yet. Researchers reading docs expect them to work.
- **Fix:** Add status indicator to each error code page: "Active" vs "Planned". Add `chatter validate --list-checks` to show which checks are active.
- After implementing Priority 1+2, gap drops from 69 to ~41 (mostly parser recovery edge cases).

### 3e. Error message consistency pass
Audit all emitted error messages for consistent phrasing:
- "Expected X, found Y" for mismatches
- "Missing X after Y" for structural gaps
- CHAT manual section references where applicable
- Visual alignment diffs for %mor and %gra (already done for %wor — extend)

---

## Section 4: CLAN Command Discoverability

### 4a. Command categorization in help text
- `chatter clan --help` lists 80 commands flat
- **Fix:** Group by category using clap `help_heading`: Analysis, Transforms, Format Converters

### 4b. `chatter clan --list` with filtering
- `chatter clan --list` — print all commands grouped by category
- `chatter clan --list --category analysis` — filter to one category

### 4c. CLAN flag compatibility documentation
- Flag rewriting (`+t*CHI` → `--speaker CHI`) is invisible
- **Fix:** "For CLAN Users" book page with old → new flag mapping table. Reference from `chatter clan --help` epilog.

---

## Section 5: Documentation for Researchers

### 5a. Quick-start guide (new book page)
Single page, zero to productive in 5 minutes:
1. Download binary
2. Validate a file
3. Read error output (annotated example)
4. Run an analysis
5. Convert to JSON
6. Next steps: VS Code, watch mode, batch workflows

Second page in book (after Introduction). Linked from README.

### 5b. Error code reference improvements
- Verify every active error code links to full description with examples and fix guidance
- Add "Active" / "Planned" status badges (ties into 3d)

### 5c. "For CLAN Users" migration guide
- Dedicated page mapping CLAN commands to chatter equivalents
- Common flag translations in a table
- Make prominent in book navigation

### 5d. CODE_OF_CONDUCT.md
- Missing from repo. Standard for public open-source academic projects.
- **Fix:** Adopt Contributor Covenant 2.1.

### 5e. Root CHANGELOG.md
- Per-crate changelogs exist but researchers don't care about crate boundaries
- **Fix:** Root CHANGELOG.md summarizing user-visible changes. Updated at each release.

---

## Section 6: Testing Improvements

### User-facing tests

**6a. Error message regression tests**
- Snapshot tests (`insta`) for the 10 most common existing error codes
- Every Priority 1 spec implementation (3b) naturally produces these via TDD

**6b. Cascading error hint tests**
- File with structural errors → verify hint mentions skipped checks
- File with only semantic errors → verify no hint
- File with mixed → verify accuracy

**6c. CLAN command output golden tests expansion**
- Expand from ~20 to ~60 fixtures covering top 10 commands (freq, mlu, mlt, vocd, kwal, combo, flo, lowcase, eval, kideval) with 3-5 fixtures each

**6d. JSON roundtrip edge cases**
- 10-15 targeted fixtures: every tier type, CA notation, Unicode/IPA, multi-language

**6e. VS Code extension tests**
- Integration tests for diagnostic display pipeline (LSP → extension → decorations)
- Requires `@vscode/test-electron`

**6f. Exit code contract tests**
- `chatter validate valid.cha` → exit 0
- `chatter validate invalid.cha` → exit 1
- `chatter validate nonexistent.cha` → exit 1
- `chatter clan freq valid.cha` → exit 0

### Internal quality tests

**6g. Derive macro tests (CRITICAL — currently 1 test / 1,480 LOC)**
- `SemanticEq`: field skipping, nested structs, enums with data, Option/Vec fields
- `SpanShift`: offset arithmetic, nested shifting, zero-offset identity
- `ValidationTagged`: error code propagation, category tagging
- Compile-fail tests via `trybuild` (expand from 1 to ~10)
- **Target:** 30+ tests

**6h. Transform pipeline tests (CRITICAL — currently 19 tests / 4,079 LOC)**
- Pipeline happy path for each output format
- Error propagation: IO errors, parse errors, validation errors in streaming mode
- Cache: hit, miss, invalidation, corrupt recovery
- Streaming: partial results on error, ordering
- Async features: test with tokio runtime
- **Target:** 50+ tests

**6i. LSP request/response golden tests**
- Diagnostic publishing, incremental parsing, hover, folding, linked editing
- Golden approach: LSP request JSON in, response JSON out, snapshot comparison
- **Target:** 40+ tests

**6j. FFI binding tests**
- Error handling: CLAN not installed, timeout, error return
- Platform-absent: macOS FFI on Linux returns clean error
- **Target:** 20+ tests (platform-gated)

**6k. Concurrent and stress testing**
- Cache: concurrent reads/writes, invalidation under contention
- Pipeline: parallel validation with shared cache
- LSP: rapid edit sequences
- Parser: thread-per-file determinism
- **Target:** 15+ tests, run with `--ignored` (slow)

**6l. Property test gaps**
- Alignment logic: output word count == input word count
- Span/position math: shift(N) then shift(-N) is identity
- Cache keys: same content → same key
- **Target:** 5 new proptest suites

**6m. Fuzzing CI integration**
- CI job running each fuzz target for 60 seconds (smoke-level)
- Document process for triaging findings into regression fixtures
- OSS-Fuzz enrollment deferred

**6n. Benchmark CI gate**
- Use `divan` (not criterion) for benchmarks
- CI step comparing against committed baseline
- Fail if any benchmark regresses >15%
- Store results as CI artifacts

---

## Section 7: VS Code Extension Polish

### 7a. Marketplace publishing
- Set up `TalkBank` publisher account
- Add `vsce publish` to `vscode-release.yml` CI workflow

### 7b. eslint dual config cleanup
- Delete `.eslintrc.json` (legacy), keep `eslint.config.js` (flat config for ESLint 10)
- Verify CI passes

### 7c. Extension README for Marketplace
- Researcher-facing: what it does, screenshots of diagnostics and waveform, installation (one click), requirements
- Not developer-facing

### 7d. `talkbank-lsp` standalone binary
- Add `[[bin]]` target to `crates/talkbank-lsp/Cargo.toml`
- Thin `main.rs` calling `talkbank_lsp::run_stdio_server()`
- Remove `chatter lsp` subcommand from CLI
- Update CI to build both `chatter` and `talkbank-lsp` per platform
- Extension auto-downloads `talkbank-lsp` on first activation from GitHub Releases

### 7e. LSP binary bundling in extension
- Platform-specific `.vsix` bundles (VS Code Marketplace supports this)
- Auto-download fallback if not bundled
- Update `vscode/src/activation/lsp.ts` discovery to prefer bundled binary

---

## Section 8: Internal Quality and Maintenance

### 8a. Not-implemented spec tracking
- CI step counting not-implemented specs (visible metric, not a gate)
- GitHub tracking issue linking to prioritized list

### 8b. Stale experiment cleanup
- Investigate `experiments/multi-root-grammar/variant/` — archive or update

### 8c. Dead code: W724
- W724 (GRA ROOT head not self) defined but never emitted
- Either implement or remove the spec

### 8d. Auto-generated mystery specs
- 9 specs (E203, E230, E243, E253, E312, E315, E360, E364, E388) have no description
- Investigate against real corpus data, document, or deprecate

### 8e. Reference corpus expansion
- 88 files / ~3,947 lines is small for a parser project
- Add 20-30 new reference files using trimming tools (`chatter trim`, `scripts/analysis/trim_chat_audio.py`)
- Cover: every tier type, all 20 languages, CA edge cases, Unicode/IPA
- Update all CLAUDE.md corpus counts

---

## Session Plan (Prioritized)

| Session | Track 1 (User-Facing) | Track 2 (Internal Quality) |
|---------|----------------------|---------------------------|
| **1** | README download visibility (2a), first-run hint (2b) | CLAUDE.md accuracy sweep: all 6 fixes (Section 1) |
| **2** | Quick-start guide book page (5a) | Derive macro tests: 30+ tests (6g) |
| **3** | Implement E720-E724: GRA validation (3b) | Transform pipeline tests: 50+ tests (6h) |
| **4** | Implement E351-E355: cross-utterance validation (3b) | Dead code / mystery spec cleanup (8c, 8d) |
| **5** | Cascading error hints: "fix X first" (3a) | LSP golden tests: 40+ tests (6i) |
| **6** | Implement E600, E711, E316, E321, E323, E708, E710 (3b) | Concurrent/stress tests (6k) |
| **7** | CLAN discoverability: grouped help, --list, flag docs (Section 4) | Property test gaps (6l) |
| **8** | Implement Priority 2 specs: E381-E704 (3c) | FFI binding tests (6j) |
| **9** | Error message consistency pass (3e) | Reference corpus expansion (8e) |
| **10** | Exit code docs + CI integration guide (2c) | Fuzzing CI integration (6m) |
| **11** | Error code Active/Planned badges, --list-checks (3d) | Benchmark CI gate with divan (6n) |
| **12** | `talkbank-lsp` standalone binary (7d) | Exit code contract tests (6f) |
| **13** | VS Code Marketplace publishing + README (7a, 7c) | CLAN golden test expansion (6c) |
| **14** | "For CLAN Users" migration guide (5c) | JSON roundtrip edge fixtures (6d) |
| **15** | CODE_OF_CONDUCT.md + root CHANGELOG.md (5d, 5e) | VS Code extension tests (6e) |
| **16** | Stale experiment cleanup (8b) | Not-implemented spec CI metric (8a) |

**Key dependencies:**
- Session 1 (CLAUDE.md sweep) first — fixes information all subsequent sessions rely on
- Sessions 3, 4, 6 (spec implementations) prerequisite for Session 11 (Active/Planned badges)
- Session 12 (`talkbank-lsp`) prerequisite for Session 13 (Marketplace publishing)
- Session 3 and 6 both implement Priority 1 specs (Section 3b), split by subgroup: GRA validation first (Session 3), then tier alignment + parser recovery (Session 6)

**Estimated total:** ~16 sessions. Spec implementations (Sessions 3, 4, 6, 8) are heaviest — each involves TDD for 5-11 error codes.

---

## What This Plan Does NOT Cover

- **crates.io publication** — API surface polish, semver stability contract (future plan if needed)
- **Platform-specific installers** — `.dmg`, `.msi`, Homebrew (future, after adoption proves demand)
- **Desktop app** — explicitly experimental, excluded from release contract
- **OSS-Fuzz enrollment** — deferred to after initial release
- **tree-sitter-grammar-utils publication** — long-term, not blocking release
- **Internationalization** — error strings are English-only (appropriate for academic tool)
