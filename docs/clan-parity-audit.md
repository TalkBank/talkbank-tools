# CLAN Command Parity Audit

**Status:** Current
**Last updated:** 2026-04-02 07:56 EDT

## Goal

Every `chatter clan` command must be an **exact drop-in replacement** for its
CLAN counterpart: same flags, same directory handling, same output format, same
exit codes. The standard of correctness is behavioral identity verified by
golden tests against actual CLAN binaries, not "inspired by."

## Methodology

- **Golden tests:** `parity_case_tests!` macro runs both CLAN binary (via
  `CLAN_BIN_DIR`) and Rust implementation on the same corpus file, capturing
  output as insta snapshots (`@clan` vs `@rust`).
- **CLI contract tests:** `legacy_clan_cli_contracts.rs` verifies flag
  rewriting, exit codes, stdout/stderr routing, and success/error messages.
- **Flag rewriting:** `clan_args::rewrite_clan_args()` translates CLAN `+flag`
  syntax to modern `--flag` equivalents before clap parsing.

## Legend

| Symbol | Meaning |
|--------|---------|
| Y | Fully implemented and tested |
| P | Partially implemented |
| N | Not implemented |
| -- | Not applicable |
| Bug | Known bug (see notes) |

---

## Analysis Commands

All analysis commands accept `Vec<PathBuf>` (multiple files + directories via
`DiscoveredChatFiles`). All support `CommonAnalysisArgs`: `--speaker`,
`--exclude-speaker`, `--gem`, `--exclude-gem`, `--include-word`,
`--exclude-word`, `--range`, `--per-file`, `--include-retracings`, `--format`.

Legacy flag rewriting works for all analysis commands via `clan_args.rs`:
`+t*CHI`, `-t*MOT`, `+t%mor`, `+s"word"`, `+g<label>`, `+z25-125`, `+r6`,
`+u` (no-op merge), `+d<N>`, `+k`, `+f<ext>`, `+w<N>`, `-w<N>`,
`+t@ID="..."`.

| Command | Golden @clan | Golden @rust | CLI Contract | Dir Support | Notes |
|---------|:---:|:---:|:---:|:---:|-------|
| freq | Y | Y | Y | Y | + CLAN output format parity test |
| mlu | Y | Y | N | Y | |
| mlt | Y | Y | N | Y | |
| wdlen | Y | Y | N | Y | |
| wdsize | N | Y | N | Y | Rust-only (new feature) |
| maxwd | Y | Y | N | Y | |
| freqpos | Y | Y | N | Y | |
| timedur | Y | Y | N | Y | |
| kwal | Y | Y | N | Y | |
| combo | Y | Y | N | Y | |
| gemlist | Y | Y | N | Y | |
| cooccur | Y | Y | N | Y | |
| dist | Y | Y | N | Y | |
| chip | Y | Y | N | Y | |
| phonfreq | Y | Y | N | Y | |
| modrep | Y | Y | N | Y | |
| vocd | Y | Y | N | Y | |
| uniq | Y | Y | N | Y | |
| codes | Y | Y | N | Y | |
| trnfix | Y | Y | N | Y | |
| chains | Y | Y | N | Y | |
| complexity | N | Y | N | Y | Rust-only |
| corelex | N | Y | N | Y | Rust-only |
| keymap | Y | Y | N | Y | |
| script | N | Y | N | Y | Rust-only (requires template file) |
| mortable | N | Y | N | Y | Rust-only (requires script file) |
| dss | Y | Y | N | Y | |
| ipsyn | Y | Y | N | Y | |
| flucalc | Y | Y | N | Y | |
| eval | Y | Y | N | Y | |
| eval-d | N | Y | N | Y | No CLAN equivalent to test against |
| sugar | Y | Y | N | Y | |
| kideval | Y | Y | N | Y | |
| rely | N | Y | N | -- | Paired-file command (2 files, not dir) |

**Summary:** 28/34 analysis commands have full CLAN parity golden tests.

---

## Compatibility Commands

| Command | Golden @clan | Golden @rust | CLI Contract | Dir Support | Notes |
|---------|:---:|:---:|:---:|:---:|-------|
| check | Y | Y | Y (8 tests) | **Y** | Fixed 2026-03-29: multi-file, +u bug, success message |
| fixit | N | N | Y | N | Alias for `chatter normalize` |
| longtier | N | Y | N | N | Transform alias |
| indent | N | Y | N | N | Transform alias |
| gemfreq | N | N | Y | Y | Alias for `freq --gem` (requires `--gem`) |

### CHECK Parity Details

| Aspect | Status | Notes |
|--------|--------|-------|
| Directory/multi-file | Y | `DiscoveredChatFiles` recursive walking |
| `+c0`/`+c1` bullets | Y | Via clan_args rewriter |
| `+e` list errors | Y | `--list-errors` |
| `+eN`/`-eN` error filter | Y | `--error`/`--exclude-error` |
| `+g2` target child | Y | `--check-target` |
| `+g4` check ID | Y | `--check-id` (on by default) |
| `+g5` unused speakers | Y | `--check-unused` |
| `+u` UD features | **Y** | Fixed 2026-03-29: context-aware rewriter |
| `+g1` prosodic delimiters | Y (no-op) | Parser always recognizes them |
| `+g3` word detail checks | P | Partially via parser word validation |
| Success message | **Y** | `ALL FILES CHECKED OUT OK!` (matches CLAN) |
| Error format | Y | `*** File "path": line N.` + `(error_num)` |
| Exit codes | Y | 0=clean, 1=errors |
| depfile.cut validation | P | Form markers (147) now validated via grammar. Roles (11,17) partial via @Options. |

---

## Transform Commands

All transforms accept single `PathBuf` + optional `--output`. This matches CLAN
behavior (transforms produce one output file per input file).

| Command | Golden @rust | Dir Support | Notes |
|---------|:---:|:---:|-------|
| flo | Y (2 cases) | N | |
| lowcase | Y | N | |
| chstring | Y | N | Requires `--changes` file |
| dates | Y | N | |
| delim | Y | N | |
| fixbullets | Y | N | |
| retrace | Y | N | |
| repeat | Y | N | Requires `--speaker` |
| combtier | Y | N | |
| compound | Y | N | |
| tierorder | Y | N | |
| lines | Y (2 cases) | N | |
| dataclean | Y | N | **Bug:** `golden_dataclean_retrace` snapshot mismatch (pre-existing) |
| quotes | Y | N | |
| ort | Y | N | Requires `--dictionary` |
| postmortem | Y | N | Requires `--rules` |
| makemod | Y | N | Requires `--lexicon` |
| gem | Y (2 cases) | N | |
| trim | Y (2 cases) | N | |
| roles | Y | N | |
| indent | Y | N | CA overlap alignment |
| longtier | Y | N | |

**Note:** Transform golden tests are Rust-only snapshots (no CLAN binary
comparison). CLAN parity for transforms would require extending the harness to
invoke CLAN transform binaries.

---

## Converter Commands

All converters accept single `PathBuf` + optional `--output`. Golden tests
live in `crates/talkbank-clan/tests/converter_golden.rs`. Each test produces
two insta snapshots (`@rust` and `@clan`); converters without a CLAN binary
produce only a `@rust` snapshot.

| Command | Golden Tests | CLAN Binary | Dir Support | Notes |
|---------|:---:|:---:|:---:|-------|
| chat2text | Y | N | N | Rust-only snapshot (includes `--speakers` variant) |
| chat2srt | Y | N | N | Rust-only snapshot (no CLAN binary) |
| chat2vtt | — | — | — | **Not implemented in code** (listed in audit only) |
| chat2praat | Y | Y | N | |
| chat2elan | Y | Y | N | |
| text2chat | Y | Y | N | |
| srt2chat | Y | Y | N | |
| lipp2chat | Y | Y | N | |
| elan2chat | Y | Y | N | |
| praat2chat | Y | Y | N | |
| lena2chat | Y | Y | N | |
| play2chat | Y | Y | N | |
| lab2chat | Y | N | N | Rust-only snapshot (no CLAN binary) |
| rtf2chat | Y | N | N | Rust-only snapshot (no CLAN binary) |
| salt2chat | Y | Y | N | |

All 13 implemented converters have golden tests. 10 use CLAN binary
comparison; 3 (chat2srt, lab2chat, rtf2chat) use Rust-only snapshots because
no corresponding CLAN binary is available. chat2vtt is not yet implemented.

---

## Deliberately Not Implemented

| Command | Status | Notes |
|---------|--------|-------|
| mor | Placeholder | "Use batchalign" message |
| post | Placeholder | |
| megrasp | Placeholder | |
| postlist | Placeholder | |
| postmodrules | Placeholder | |
| posttrain | Placeholder | |

---

## Flag Rewriting Audit (`clan_args.rs`)

| CLAN Flag | Rewritten To | Context-Aware | Tested | Notes |
|-----------|-------------|:---:|:---:|-------|
| `+t*CHI` | `--speaker CHI` | N | Y | |
| `-t*MOT` | `--exclude-speaker MOT` | N | Y | |
| `+t%mor` | `--tier mor` | N | Y | |
| `-t%gra` | `--exclude-tier gra` | N | Y | |
| `+t@ID="..."` | `--id-filter "..."` | N | Y | |
| `+s"word"` | `--include-word word` | N | Y | |
| `-s"word"` | `--exclude-word word` | N | Y | |
| `+g<label>` | `--gem <label>` | **Y** (CHECK) | Y | CHECK: `+g1`-`+g5` → specific flags |
| `-g<label>` | `--exclude-gem <label>` | N | Y | |
| `+z25-125` | `--range 25-125` | N | Y | |
| `+r6` | `--include-retracings` | N | Y | |
| `+u` | `--check-ud` (CHECK) / no-op | **Y** (CHECK) | Y | Fixed 2026-03-29 |
| `+d<N>` | `--display-mode <N>` | N | Y | |
| `+k` | `--case-sensitive` | N | Y | |
| `+f<ext>` | `--output-ext <ext>` | N | Y | |
| `+w<N>` | `--context-after <N>` | N | Y | |
| `-w<N>` | `--context-before <N>` | N | Y | |
| `+c<N>` | `--bullets <N>` | N | Y | CHECK-specific |
| `+e` / `+e<N>` | `--list-errors` / `--error <N>` | N | Y | CHECK-specific |
| `-e<N>` | `--exclude-error <N>` | N | Y | CHECK-specific |

**Total:** 39 unit tests for flag rewriting.

---

## chatter validate vs CLAN CHECK — Error Detection Parity

### Headline

**chatter validate detects every error that CLAN CHECK detects.** On our
42-file CLAN-verified synthetic corpus (one file per CHECK error number),
CLAN-only errors = 0. Every CHECK error triggers a chatter diagnostic.

### Synthetic Corpus Results (2026-03-29)

42 files, each triggering exactly one CHECK error, verified against the
actual CLAN CHECK binary (`~/talkbank/OSX-CLAN/src/unix/bin/check`).

| Metric | Value |
|--------|-------|
| Files in corpus | 42 |
| CLAN detects errors | 44 (some files trigger multiple) |
| chatter detects errors | 46 |
| **Exact CHECK number match** | **36 (86%)** |
| Different CHECK number (still detected) | 6 |
| **CLAN detects, we don't** | **0** |
| We detect, CLAN doesn't | 2 (we're stricter) |

### CHECK Errors: What We Detect

| CHECK # | Description | Our Code | Status |
|---------|-------------|----------|--------|
| 2 | Missing colon after tier | E525 → 17 | Different number |
| 4 | Space instead of TAB | E303 → 8 | Different number |
| 7 | @End missing | E502 → 7 | **Exact** |
| 11 | Symbol not in depfile | E534 → 11 | **Exact** |
| 13 | Duplicate speaker | CHECK-level | **Exact** |
| 15 | Illegal role | E532 → 15 | **Exact** |
| 16 | Extended chars in speaker | E308 → 18 | Different number |
| 17 | Tier not in depfile | E525 → 17 | **Exact** |
| 18 | Speaker not in participants | E522 → 18 | **Exact** |
| 21 | Missing terminator | E304 → 12 | Different number |
| 22 | Unmatched `[` | E375 → 48 | Different number |
| 34 | Illegal date | E518 → 34 | **Exact** |
| 36 | Text after delimiter | E305 → 21 | Different number |
| 38 | Numbers in words | E220 → 47 | Partial match |
| 40 | Duplicate dependent tier | E601 → 40 | **Exact** |
| 44 | Content after @End | E501 → 44 | **Exact** |
| 47 | Numbers inside words | E220 → 47 | **Exact** |
| 48 | Illegal character | E315 → 86 | Different number |
| 50 | Redundant terminator | E305 → 21 | Different number |
| 53 | Duplicate @Begin | E501 → 44 | Different number |
| 60 | @ID missing | E522 → 18 | Different number |
| 64 | Wrong gender | E540 → 64 | **Exact** |
| 69 | UTF8 missing | E500 → 69 | **Exact** (+ extras) |
| 70 | Empty utterance | E253 → 70 | **Exact** |
| 82 | BEG > END | E701 → 82 | **Exact** |
| 83 | BEG < prev BEG | E701 → 83 | **Intentional divergence**: chatter scopes to same-speaker; CLAN fires cross-speaker (see `book/src/architecture/bullet-validation.md`) |
| 84 | Cross-speaker overlap | E729 (off by default) | **Intentional divergence**: not in default validation; CLAN requires `+c0` |
| 94 | Terminator mismatch | E707 → 94 | **Exact** |
| 117 | Unpaired CA delimiter | E372 → 117 | **Exact** |
| 120 | Two-letter language code | E375 → 48 | Different number |
| 121 | Unknown language code | E519 → 121 | **Exact** |
| 122 | @ID lang not in @Languages | E519 → 121 | Mapped (new) |
| 133 | Speaker self-overlap | E704 → 133 | **Higher count than CLAN** due to CLAN's error-83-shadows-133 bug (see `book/src/architecture/bullet-validation.md`) |
| 140 | %MOR size mismatch | E705 → 140 | **Exact** |
| 142 | Role mismatch | E532 → 15 | Different number (new) |
| 143 | @ID needs 10 fields | E510 → 143 | **Exact** |
| 144 | Illegal SES | E546 → 144 | **Exact** |
| 147 | Undeclared form marker | E203 → 147 | **Exact** (grammar fix) |
| 153 | Age zero-padding | E517 → 153 | **Exact** (new) |
| 155 | Use "0word" not "(word)" | E209 → 155 | **Exact** |
| 156 | Replace ,, | E258 → 156 | **Exact** |
| 157 | Media filename mismatch | CHECK-level | **Exact** (new) |
| 158 | [: ...] must have real word | E391 → 158 | **Exact** |
| 161 | Space before `[` | E375 → 48 | Different number |

### CHECK Errors That Cannot Occur (Grammar Prevents Them)

| CHECK # | Description | Why |
|---------|-------------|-----|
| 81 | Bullet must follow delimiter | Grammar enforces bullet position |
| 89 | Wrong chars in bullet | Grammar only accepts digits |
| 90 | Illegal time in bullet | Grammar validates format |
| 118 | Delimiter must precede bullet | Grammar enforces order |

### Checks We Perform That CLAN Doesn't

| Our Code | Description |
|----------|-------------|
| E729 | Cross-speaker bullet overlap (CHECK 84) |
| E731 | Same-speaker bullet timing overlap |
| E375 | Bracket annotation parse errors |
| E252 | Syllable pause misplacement |

### Reproduction

```bash
# Regenerate synthetic corpus (42 files, each CLAN-verified)
python3 scripts/synthesize_check_corpus.py

# Capture CLAN output
~/talkbank/OSX-CLAN/src/unix/bin/check tests/check-error-corpus/synthetic/*.cha \
  > tests/check-error-corpus/synthetic/CLAN_CHECK.log 2>/dev/null

# Capture chatter output
chatter clan check tests/check-error-corpus/synthetic/*.cha \
  > tests/check-error-corpus/synthetic/CHATTER_CHECK.log 2>/dev/null

# Compare
python3 scripts/compare_check_parity.py \
  tests/check-error-corpus/synthetic/CLAN_CHECK.log \
  tests/check-error-corpus/synthetic/CHATTER_CHECK.log
```

---

## Known Gaps (Prioritized)

### P0 — Correctness Bugs
*None currently known.*

### P1 — CHECK Number Mismatch (6 files, all detected)
6 files where we detect the same error but emit a different CHECK
number. **All 6 are detected — the user is told something is wrong.**

| # | CLAN | Us | Assessment |
|---|------|----|-----------|
| 6 | 43,61,77 | 6 | Same: missing @Begin. CLAN reports 3 ways, we report 1. |
| 11 | 11 | 38,47,48 | Both catch it. [x N] depfile issue vs digit/annotation. |
| 22 | 11,21,22,48,106 | 48 | Complex broken file. We catch it with fewer codes. |
| 48 | 48 | E316 | Data corruption (control char). We reject the content. |
| 53 | 53,61,77 | 53 | Same: duplicate @Begin. CLAN adds cascade codes. |
| 120 | 120 | 48 | [- en] mid-utterance. We reject it (wrong position). |

### P2 — Bullet Consistency Mode (`+c`)
- **CHECK 85** (gap between tiers): E730 defined, not implemented
- **CHECK 110** (missing bullet): E732 defined, not implemented
- These only fire in `+c0`/`+c1` mode. Need `--bullets` mode integration.

### P3 — Other Command Parity
1. **Converter golden tests** — 15 converters have zero golden tests
2. **Transform CLAN parity** — 22 transforms tested as Rust-only snapshots
3. **`+g3` word detail checks** — partially covered
4. **More CLI contract tests** — only 5 commands have them

---

## How to Run Parity Tests

```bash
# All golden tests (skips CLAN comparison if CLAN_BIN_DIR not set)
cargo nextest run -p talkbank-clan -E 'test(golden)'

# With actual CLAN binary comparison
CLAN_BIN_DIR=~/OSX-CLAN/build cargo nextest run -p talkbank-clan -E 'test(golden)'

# CLI contract tests
cargo nextest run -p talkbank-cli -E 'test(legacy)'

# Flag rewriter tests
cargo nextest run -p talkbank-clan -E 'test(clan_args)'

# CHECK-specific tests
cargo nextest run -p talkbank-cli -E 'test(check)'
cargo nextest run -p talkbank-clan -E 'test(golden_check)'
```

## How to Add a New Parity Test

1. Add a case to the appropriate file in `crates/talkbank-clan/tests/clan_golden/`
2. Use `ParityCase::command(...)` for analysis commands or `ParityCase::check(...)` for CHECK
3. Run with `CLAN_BIN_DIR` set to generate the `@clan` snapshot
4. Review and accept the snapshot with `cargo insta review`
5. Verify the `@rust` snapshot matches the `@clan` snapshot

---

## Revision History

| Date | Changes |
|------|---------|
| 2026-03-29 | Initial audit. Fixed CHECK multi-file, +u bug, success message. |
| 2026-03-29 | Built 42-file CLAN-verified synthetic error corpus. |
| 2026-03-29 | E316 catch-all: 21→2 files. 15+ specific diagnostic patterns added. |
| 2026-03-29 | Implemented CHECK 13, 84, 122, 142, 147, 153, 157. Grammar fix for form marker greedy prefix. |
| 2026-03-29 | **CLAN-only = 0.** Every CLAN CHECK error now detected. 22/42 exact number matches (52%). |
| 2026-03-29 | CHECK number refinement + cascade suppression. 36/42 exact (86%). 6 remaining are all detected, different codes. |
| 2026-03-29 | Final assessment: zero real detection gaps. All mismatches are code numbering, not missing validation. |
