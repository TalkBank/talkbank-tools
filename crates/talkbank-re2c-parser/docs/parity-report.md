# Re2cParser Parity Report

**Status:** Current
**Last updated:** 2026-04-01 13:48 EDT

## Overview

The re2c parser is an alternative CHAT parser using a [re2c](https://re2c.org/) DFA
lexer and [chumsky](https://docs.rs/chumsky/1.0.0-alpha.8) parser combinators. It
implements the `ChatParser` trait, making it a drop-in replacement for
`TreeSitterParser`.

TreeSitterParser is the canonical oracle. The re2c parser must match its output
on valid CHAT and detect the same errors on invalid CHAT.

## Current Metrics

| Metric | Value | Notes |
|--------|-------|-------|
| **Speed** | 4-8x faster | 7.2x on 35-file batch (divan benchmarks) |
| **Reference corpus** | 87/87 match | 100% SemanticEq on all reference files |
| **Error detection** | 140/140 (100%) | Every testable error spec is detected |
| **Error recovery** | 241/241 (100%) | Zero panics on any invalid input |
| **Error code match** | 79/140 (56.4%) | Same code as TreeSitter |
| **Both detect, diff code** | 61/140 (43.6%) | Different code, both report error |
| **Silent gaps** | 0 | No case where re2c misses an error |
| **Both empty** | 0 | No spec where neither parser reports |
| **Wild corpus** | ~98.7% parity | ~1,300 divergences at 90k files (was 10,068 pre-chumsky) |
| **Crate tests** | 278/278 pass | Plus 18 skipped (not_implemented) |

## Architecture

```
Source text → re2c DFA Lexer → Token slice → Chumsky Combinators → AST → Model
                (lexer.re)       (token.rs)    (parser/*.rs)      (ast.rs) (convert.rs)
```

- **Lexer:** re2c DFA with 13 start conditions, rich tokens (Word, MorWord, etc.)
- **Parser:** 8 chumsky modules (1,838 lines total), no legacy Parser struct
- **Convert:** `From<&ast::Type>` impls, source-free (AST carries `raw_text`)

## Error Detection

Both parsers detect errors on all 140 testable error specs. The 61 cases where
error codes differ are because the architectures produce different diagnostics:

- Re2c lexer errors (token-level) vs TreeSitter ERROR nodes (CST-level)
- Different alignment validation paths for structurally different models
- Re2c reports E321 (UnparsableUtterance) where TS reports E316 (UnparsableContent)

Neither code set is "wrong" — both report user-actionable diagnostics. The
re2c parser's error codes are specific (E321, E319, E316, E602) rather than
generic (E309 was eliminated).

## Known Divergences

### Remaining wild-corpus divergences (~1,300 files)

The pre-chumsky divergence count was 10,068 (10.1%). After the chumsky pivot
and subsequent fixes, it's ~1,300 (~1.3%). Root causes:

| Category | Impact | Status |
|----------|--------|--------|
| Bullet classification | Was #1 (6,680 files) | Fixed (extract_terminal_bullet) |
| Event trailing `>` | Was #2 (570 files) | Fixed (chumsky group scoping) |
| Missing dependent tiers | Was #3 (1,252 files) | Mostly fixed |
| Content length cascade | Was #4 (6,680 files) | Fixed (consequence of above) |
| CA terminator promotion | Dozens of files | Fixed (resolve_ca_terminator ordering) |
| Error marker spaces | MSU03b + similar | Fixed (lexer ErrorMarkerAnnotation fix) |
| Standalone shortening | Several files | Fixed (lexer rule ordering) |
| Standalone colon | Several files | Fixed (Colon separator token) |
| `&=&=label` double event | 2 files | Data quality issue (& now forbidden) |
| Skip bullet dash | 7 files | Data quality issue (skip deprecated) |

### Not-implemented error specs (88 specs)

These are error specs where the validation code hasn't been written yet. They
apply equally to both parsers (validation-layer, not parser-layer):

- 75 original `not_implemented` specs
- 13 newly added (E360 deprecated skip, etc.)

## CLI Usage

```bash
# Use re2c parser for validation (faster than default tree-sitter)
chatter validate --parser re2c corpus/

# Use re2c parser with roundtrip testing
chatter validate --parser re2c --roundtrip corpus/

# Default: tree-sitter (supports incremental reparsing for LSP)
chatter validate corpus/
```

The `--parser re2c` flag selects the re2c backend for all validation in that
run. The cache is parser-aware (separate entries for `tree-sitter` vs `re2c`).

**When to use re2c:**
- Batch validation of large corpora (4-8x faster)
- Testing parser parity (specification oracle)
- Profiling parse performance

**When to use tree-sitter (default):**
- LSP (requires incremental reparsing)
- When error code specificity matters (TreeSitter has more specific E3xx codes)
- When CST-level information is needed

## Benchmarks

Measured with divan on reference corpus files. All content pre-loaded; zero I/O.

### File-level parse (median, parser reuse)

| File | TreeSitter | Re2c+Chumsky | Speedup |
|------|-----------|-------------|---------|
| basic-conversation (13 lines) | 44 µs | 9.6 µs | 4.6x |
| mor-gra (dependent tiers) | 69 µs | 9.4 µs | 7.3x |
| intonation (CA) | 78 µs | 19 µs | 4.1x |
| zho-conversation (CJK) | 128 µs | 19 µs | 6.6x |
| impdenis (complex, large) | 7,734 µs | 970 µs | 8.0x |

### Batch (35 files)

| Parser | Time | Files/sec |
|--------|------|-----------|
| TreeSitter | 21.7 ms | 1,613 |
| Re2c+Chumsky | 3.0 ms | 11,667 |

Run benchmarks: `cargo bench -p talkbank-re2c-parser --bench parse_comparison`

## Shared Infrastructure

Both parsers share post-hoc promotion logic in the model crate:

- `TierContent::extract_terminal_bullet()` — moves trailing InternalBullet
  to the utterance-level bullet field
- `TierContent::resolve_ca_terminator()` — promotes trailing CA intonation
  arrow separator to terminator
- `parse_bullet_node_timestamps()` — extracts (start_ms, end_ms) from
  structured bullet CST nodes

These shared methods eliminate all duplication between the two parsers' convert
paths.

## Grammar Unification

The grammar now uses a single structured `bullet` rule (was: opaque `media_url`
token + structured `inline_bullet`). Both parsers extract timestamps from
structured CST/AST children. The deprecated `skip` flag (dash before closing
NAK) was removed from the grammar and model.

## Module Structure

```
src/parser/
  mod.rs              (43)   Module decls, lex_to_tokens
  main_tier.rs       (536)   Chumsky: contents, words, groups, tier_body
  dependent_tiers.rs (334)   Chumsky: %mor, %gra, %pho, %sin, %wor, text
  classify.rs        (252)   Token classification functions
  word_body.rs       (231)   Char-level word body scanner
  file.rs            (~300)  Imperative file parser + error reporting
  entry_points.rs    (155)   Public API
  headers.rs          (75)   Chumsky: @ID, @Languages, @Participants
```
