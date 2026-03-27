# CLAUDE.md

**Last modified:** 2026-03-26 22:41 EDT

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What This Is

A CHAT transcript parser using **re2rust** (re2c's Rust backend) for lexing and a handwritten recursive-descent parser. Lives in the `talkbank-tools` workspace as `talkbank-re2c-parser`.

**Status:** Implements the `ChatParser` trait from `talkbank-model`. The public type is `Re2cParser`. Lexer validated against 99,907 CHAT files with zero errors.

## Architecture: Rich Word Token

The lexer emits a **single `Token::Word`** for each complete word, carrying tagged field boundaries:

```rust
Token::Word {
    raw_text: &str,            // full word text from source
    prefix: Option<&str>,      // "&-", "&~", "&+", or "0"
    body: &str,                // word body (parser handles internals)
    form_marker: Option<&str>, // "f", "z:grm" (content only, no @)
    lang_suffix: Option<&str>, // "eng+zho" (no @s:), "" for bare @s
    pos_tag: Option<&str>,     // "n", "adj" (no $)
}
```

**Design rationale:** "Word" is a first-class concept. Lex-only consumers (syntax highlighting, token counting) should see words as coherent units without parsing. The re2c DFA determines word boundaries; the parser handles body internals.

**Body parsing:** The body is too complex for fixed re2c tags (variable-length sequences of text segments, shortenings, compounds, CA markers, etc.). The parser's `parse_word_body(body: &str) -> Vec<WordBodyItem>` scans the body string.

**Source elimination:** Because the Word token carries `raw_text`, the AST's `WordWithAnnotations` has `raw_text: &str` (not a span). Conversion functions never need `source: &str` — the AST is self-contained. `From` impls work for all types.

### Other Rich Tokens

| Token | Fields | Notes |
|-------|--------|-------|
| `IdFields` | 10 pipe-delimited fields | @ID header, zero-copy |
| `TypesFields` | design, activity, group | @Types header |
| `MorWord` | pos, lemma_features | %mor word |
| `GraRelation` | index, head, relation | %gra relation |
| `MediaBullet` | start_time, end_time | Timestamp extraction |
| `OtherSpokenEvent` | speaker, text | &*SPK:word |

## Strict Adherence to grammar.js

The canonical grammar is `~/talkbank/talkbank-tools/grammar/grammar.js`. All lexer rules and parser logic must be **directly translated** from grammar.js — not invented, not approximated. When implementing a construct:

1. Find the exact rule in grammar.js
2. Translate it to re2c conditions/rules, leveraging re2c features
3. Verify with the matching spec in `spec/constructs/` or `spec/errors/`
4. **Leverage re2c to produce richer tokens** than grammar.js's flat token model can express

Key grammar.js design decisions:
- Terminators are **optional** (`optional($.terminator)` in `utterance_end`) — presence enforced by AST validation, not parsing
- Each tier type has its own content rules (`mor_contents`, `gra_contents`, `pho_groups`, `text_with_bullets`, etc.)

## Conditions (Start States)

re2c conditions are numbered states that change what rules are active:

- `INITIAL` — top-level line classification (@, *, %)
- `MAIN_CONTENT` — main tier body (words, annotations, terminators)
- `MOR_CONTENT`, `GRA_CONTENT`, `PHO_CONTENT`, `SIN_CONTENT` — tier-specific
- `ID_CONTENT`, `TYPES_CONTENT`, `LANGUAGES_CONTENT`, `PARTICIPANTS_CONTENT`, `MEDIA_CONTENT` — header-specific
- `HEADER_CONTENT`, `TIER_CONTENT` — generic structured headers/tiers

**Multiple entry points:** `Lexer::new(input, condition)` allows starting in any condition. This means we can lex a `%mor` tier body in isolation (start in `MOR_CONTENT`), a main tier content item (start in `MAIN_CONTENT`), etc.

**Continuation rule:** The lexer's continuation rule (`<*> [\r\n]+ [\t]`) must NOT reset the condition. Continuation content stays in the same lexer mode.

## Entry Points

| Entry point | Start condition | Input | Output |
|-------------|----------------|-------|--------|
| `parse_main_tier` | `INITIAL` | `*CHI:\thello .\n` | `MainTier` |
| `parse_chat_file` | `INITIAL` | full `.cha` file | `ChatFile` |
| `parse_word` | `MAIN_CONTENT` | `ice+cream@f` | `WordWithAnnotations` |
| `parse_mor_tier` | `MOR_CONTENT` | `pro\|I v\|want .\n` | `MorTier` |
| `parse_gra_tier` | `GRA_CONTENT` | `1\|2\|SUBJ 2\|0\|ROOT` | `GraTier` |
| `parse_pho_tier` | `PHO_CONTENT` | `wɑ+kɪŋ hɛloʊ .\n` | `PhoTier` |
| `parse_text_tier` | `TIER_CONTENT` | text with bullets | `TextTierParsed` |
| `parse_id_header` | `ID_CONTENT` | `eng\|corpus\|CHI\|...` | `IdHeaderParsed` |

## ChatParser Trait

`Re2cParser` implements `ChatParser` from `talkbank-model`, providing all parse methods:
- File-level: `parse_chat_file`
- Line-level: `parse_header`, `parse_utterance`, `parse_main_tier`
- Token-level: `parse_word`, `parse_mor_word`, `parse_gra_relation`
- Tier-level: `parse_mor_tier`, `parse_gra_tier`, `parse_pho_tier`, plus all text tiers

Conversion functions in `convert.rs` are source-free — all use `From` impls or take only AST types.

## Build & Test

```sh
cd ~/talkbank/talkbank-tools
cargo check -p talkbank-re2c-parser
cargo nextest run -p talkbank-re2c-parser     # prefer nextest for speed
cargo test -p talkbank-re2c-parser --jobs 1   # fallback
```

Requires `re2rust` (part of re2c) on PATH: `brew install re2c`.

The build script (`build.rs`) runs `re2rust` on `src/lexer.re` → `OUT_DIR/lexer.rs`. Edit `lexer.re`, not generated output. Use `\x00` (not `\0`) for NUL — re2c treats `\0` as octal prefix.

## Testing

- **Lexer tests:** `tests/lexer_tests.rs` — unit tests per token type using start conditions. Checks Word token fields (prefix, body, form_marker, lang_suffix, pos_tag).
- **Corpus lexer tests:** `tests/corpus_lex_tests.rs` — lex real lines from `~/talkbank/data/*-data` (99,907 .cha files). All 12 pass.
- **Parser tests:** `tests/golden_parse.rs`, `tests/parser_fixtures.rs` — parsed AST structures.
- **Equivalence tests:** `tests/equivalence_tests.rs` — Re2cParser vs TreeSitterParser comparison via `ChatParser` trait.
- **Model study:** `tests/model_study.rs` — reference corpus equivalence. 6 files have known divergences (overlaps, nonvocals, phon syllabification, wor tier, CA) — these are refinement TODOs, not regressions.
- **When a test fails, STOP and ask.** CHAT semantics are domain-specific.
- **Slow tests:** Mark with `#[ignore]` and run via `--ignored` flag.

### Lexer Validation Status

The lexer has been validated against all 99,907 .cha files in `~/talkbank/data/*-data` with ZERO errors on valid CHAT data.

## Error Token Design

Every re2c condition has a per-condition error fallback (`ErrorInMainContent`, `ErrorInMorContent`, etc.) that:
1. Consumes exactly one character
2. Stays in the same condition (lexing continues)
3. Carries context about WHERE the error occurred

The lexer NEVER fails — it always returns tokens, some of which may be error tokens.

## Rust Coding Standards

- Rust **2024 edition**, `cargo fmt` before committing.
- `thiserror` for domain errors, `miette` for rich diagnostics.
- No panics for recoverable conditions. No silent swallowing.
- `tracing` for library logging, never `println!`.
- Every `pub` type and function has a doc comment.
- File size: ≤400 lines recommended, ≤800 hard limit.

## Known Equivalence Gaps

6 reference corpus files have semantic differences between Re2cParser and TreeSitterParser:
- `nonvocal-and-long-features.cha` — nonvocal/long-feature handling
- `overlaps.cha` — overlap point placement in words
- `phon-syllabification.cha` — syllabification tier parsing
- `wor.cha` — %wor tier word extraction
- `1082.cha`, `000829.cha` — CA/word feature edge cases

These are refinement TODOs for body parser tuning, not architectural issues.
