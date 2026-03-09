# Parser Error Recovery: Direct Parser vs Tree-Sitter Parser

**Status:** Reference document
**Date:** 2026-02-18

This document audits the error recovery characteristics of both parsers, compares their leniency, and maps the parse/validate boundary for each. It directly informs the grammar redesign question: "Can we safely make tree-sitter tokens coarser and delegate fine-grained parsing to the direct parser?"

## Table of Contents

1. [Executive Summary](#1-executive-summary)
2. [Error Recovery Architecture](#2-error-recovery-architecture)
3. [Blast Radius Comparison](#3-blast-radius-comparison)
4. [Word Parsing: The Critical Case](#4-word-parsing-the-critical-case)
5. [Dependent Tier Recovery](#5-dependent-tier-recovery)
6. [Parse/Validate Boundary](#6-parsevalidate-boundary)
7. [Leniency Comparison](#7-leniency-comparison)
8. [Implications for Grammar Redesign](#8-implications-for-grammar-redesign)

---

## 1. Executive Summary

The two parsers have fundamentally different error recovery strategies:

| Property | Tree-Sitter Parser | Direct Parser (chumsky) |
|----------|-------------------|------------------------|
| **Recovery source** | Tree-sitter GLR engine provides ERROR/MISSING nodes in a complete CST | chumsky combinators fail-fast; recovery is hand-coded at tier boundaries |
| **Word-level recovery** | Partial — ERROR nodes skipped, valid sub-components preserved | **None** — one malformed sub-component kills the word, which kills the utterance |
| **Dependent tier recovery** | Partial — bad items skipped, tier partially populated | **None** — one bad morpheme/relation kills the entire tier |
| **File-level recovery** | Always produces a ChatFile (may have gaps) | Produces ChatFile only if all headers + main tiers parse; dependent tier failures are recoverable |
| **Dummy fabrication** | One case: `recover_error_as_word()` creates Word from ERROR if text looks word-like | Never fabricates model values |

**Bottom line:** The direct parser is significantly less fault-tolerant than the tree-sitter parser. It was designed for clean input (well-formed CHAT files) with the assumption that tree-sitter handles initial structural parsing and error recovery. It cannot safely replace tree-sitter as the primary parser for arbitrary user input.

---

## 2. Error Recovery Architecture

### 2A. Tree-Sitter Parser: CST-Guided Recovery

Tree-sitter's GLR parser always produces a complete Concrete Syntax Tree, even for malformed input. Invalid regions are wrapped in ERROR nodes, missing required elements become MISSING nodes. The Rust layer walks this CST and extracts as much structure as possible:

```
Input: "*CHI:\tI wo↑rd@s: cookie ."
                        ^^^ malformed (missing lang code after @s:)

Tree-sitter CST:
  (utterance
    (main_tier
      (star) (speaker) (colon) (tab)
      (tier_body
        (contents
          (word_with_optional_annotations
            (standalone_word
              (word_body (initial_word_segment "I"))))
          (word_with_optional_annotations
            (standalone_word
              (word_body ...)
              (word_langs ERROR)))    ← ERROR node for malformed @s:
          (word_with_optional_annotations
            (standalone_word
              (word_body (initial_word_segment "cookie"))))
        )
        (utterance_end (period) (newline)))))
```

The Rust parser:
1. Walks children of `contents` node
2. For "I": normal Word created
3. For "wo↑rd@s:": Word created with `raw_text` preserved, language marker skipped with error reported
4. For "cookie": normal Word created
5. **Result: Utterance with 3 words, 1 diagnostic** — parsing continues

Key recovery mechanisms in the Rust layer:
- **ERROR node analysis** (`helpers.rs:13-143`): Pattern-matches ERROR content to produce user-friendly messages
- **ERROR-as-word recovery** (`structure/convert/body.rs:306-331`): If ERROR text looks word-like (alphanumeric only), creates a Word from it — the only place dummy values are fabricated
- **ERROR fragment attachment** (`structure/contents.rs:79-139`): ERROR fragments after `@` markers silently merged into preceding word's `raw_text`
- **MISSING node detection** (`word/mod.rs:269-283`): Reports error, returns `ParseOutcome::rejected()` for that component, parsing continues
- **Per-item skip** (`mor/tier.rs`, `gra/tier.rs`): Malformed MOR/GRA items are skipped; tier is partially populated

### 2B. Direct Parser: Fail-Fast with Tier-Level Recovery

The direct parser uses chumsky 0.12 combinators. Chumsky does not use built-in recovery combinators (`recover_with()`, etc.). Instead, recovery is engineered at the file level:

```
Input: "*CHI:\tI wo↑rd@s: cookie ."
                        ^^^ malformed

Chumsky parsing:
  1. word_parser("I")        → OK
  2. word_parser("wo↑rd@s:") → try_map() fails → Rich::custom error
  3. content sequence aborts  → entire contents fails
  4. main tier parse fails    → fatal_tier_parse_failed = true
  5. FILE REJECTED
```

The direct parser's 5-phase pipeline (`file.rs`):

| Phase | What | Fatal condition | Recovery |
|-------|------|-----------------|----------|
| 1. Structure | chumsky parses tier boundaries | Any structural error | File rejected |
| 2. Content | Each tier's content parsed | Header or main tier fails | File rejected |
| 2. Content | (continued) | Dependent tier fails | Tier discarded, parse-taint set, file continues |
| 3. Grouping | Main + dependent tiers grouped into utterances | Orphaned dependent tier | Error reported, tier discarded |
| 4. Metadata | Participants built, CA normalization | — | Errors collected, continues |
| 5. Build | ChatFile constructed | — | Always succeeds if phase 2 OK |

Key recovery mechanisms:
- **`.or_not()` for optional elements** (`main_tier.rs`): Language codes, terminators, bullets, annotations are all optional — missing ones don't block parsing
- **Annotation fallback** (`main_tier.rs:1360-1374`): Unknown annotations parsed as explanation text — prevents annotation parse failures from killing the utterance
- **Parse-health classification** (`dependent_tier.rs:134-144`): Even malformed dependent tiers have their alignment domain identified (via byte-level label extraction) so only the affected domain is tainted
- **No word-level recovery**: A malformed word kills the content sequence, which kills the main tier, which kills the file

---

## 3. Blast Radius Comparison

### What Happens When One Thing Goes Wrong

| Error Location | Tree-Sitter Parser | Direct Parser |
|---------------|-------------------|---------------|
| **Bad character in word** | Word created with raw_text; error reported | Word rejected → **main tier rejected → file rejected** |
| **Malformed @s: marker** | Word created without language; error reported | Word rejected → **main tier rejected → file rejected** |
| **Broken shortening (unbalanced parens)** | ERROR node; word may still be created | Word rejected → **main tier rejected → file rejected** |
| **Bad CA element inside word** | ERROR node skipped; other word content preserved | Word rejected → **main tier rejected → file rejected** |
| **Malformed annotation \[= ...]** | Annotation skipped; word preserved | **Fallback**: parsed as explanation text (recoverable) |
| **One bad MOR morpheme** | Morpheme skipped; other morphemes preserved in tier | **Tier rejected** → parse-taint set |
| **One bad GRA relation** | Relation skipped; other relations preserved | **Tier rejected** → parse-taint set |
| **Missing %pho close bracket ›** | ERROR node; partial content preserved | **Tier rejected** → parse-taint set |
| **Malformed header** | ERROR nodes in header; file continues | **File rejected** |
| **Orphaned dependent tier** | Reported; tier discarded | Reported; tier discarded |

### Visual Summary

```
Tree-sitter parser error blast radius:
  Bad word sub-component  →  [word partially preserved] → utterance OK → file OK
  Bad dependent tier item →  [item skipped]             → tier partial → file OK
  Bad header              →  [header partial]           → file OK

Direct parser error blast radius:
  Bad word sub-component  →  word DEAD → main tier DEAD → FILE DEAD
  Bad dependent tier item →  tier DEAD → [taint set]    → file OK
  Bad header              →  FILE DEAD
```

---

## 4. Word Parsing: The Critical Case

This is the most important comparison because the grammar redesign proposes making words into opaque tokens parsed by the direct parser.

### 4A. Tree-Sitter Word Parsing

Tree-sitter's grammar decomposes a word into fine-grained nodes:

```
(standalone_word
  (word_prefix (zero))                    ← optional prefix
  (word_body
    (initial_word_segment "wo")
    (ca_element (pitch_up))               ← ↑ parsed as separate node
    (word_segment "rd"))
  (word_langs                             ← @s:lang parsed structurally
    (colon)
    (language_code "fra")))
```

When something is malformed (e.g., `@s:` with no language code):
- Tree-sitter still produces a `standalone_word` node
- The malformed part becomes an ERROR child
- The Rust layer walks children, reports the ERROR, and creates a Word with what it could extract
- **The word survives. The utterance survives. The file survives.**

Recovery mechanisms in word parsing (`word/mod.rs:255-436`):
- `is_missing()` check on each child (line 269) — rejects component, not word
- ERROR nodes in word children (line 397) — reported, parsing continues to next child
- Language form parsing failure (lines 303-338) — word created without language marker
- Word body empty fallback (line 420) — uses raw text as content

### 4B. Direct Parser Word Parsing

The direct parser treats words as chumsky combinator chains:

```rust
// main_tier.rs word parsing (simplified)
word_prefix.or_not()
  .then(word_body)                // REQUIRED — failure kills word
  .then(choice(language_marker, form_marker, pos_tag).or_not())
  .try_map(|parts, span| {
    parse_word_impl(...)          // validation + construction
      .ok_or_else(|| Rich::custom(span, "Failed to parse word"))
  })
```

When something is malformed (e.g., `@s:` with no language code):
- `language_marker` parser fails to match the incomplete `@s:`
- chumsky backtracks to `.or_not()` — **BUT** if `@s:` was partially consumed (the `@s` matched as token.immediate), there is no clean recovery point
- `try_map` error propagates → word rejected → content sequence aborted → main tier rejected → **file rejected**

**Critical gap**: There is no mechanism to say "the word was mostly OK, just the language marker was bad, keep the word." The word is either fully parsed or fully rejected.

### 4C. Concrete Example: `wo↑rd@s:`

| Step | Tree-Sitter Parser | Direct Parser |
|------|-------------------|---------------|
| Parse `wo` | `initial_word_segment` node | `initial_word_segment` regex match |
| Parse `↑` | `ca_element > pitch_up` node | CA element parser match |
| Parse `rd` | `word_segment` node | `word_segment` regex match |
| Parse `@s:` | `word_langs` node with ERROR (missing lang code) | `language_marker` parser → fail |
| **Outcome** | Word("wo↑rd") with error on @s: | `.try_map` → `Rich::custom` error |
| **Blast radius** | Word preserved, error reported | Word dead → utterance dead → file dead |

---

## 5. Dependent Tier Recovery

Both parsers handle dependent tier failures similarly — the key difference is granularity within a tier.

### 5A. Tree-Sitter: Item-Level Granularity

```
%mor:  pro|I  v|want  BROKEN  det|a  n|cookie .
```

- `pro|I` → parsed, added to tier
- `v|want` → parsed, added to tier
- `BROKEN` → ERROR node, skipped, error reported
- `det|a` → parsed, added to tier
- `n|cookie` → parsed, added to tier
- **Result: MOR tier with 4 items, 1 error** — alignment can proceed (with caveat about count)

### 5B. Direct Parser: Tier-Level Granularity

```
%mor:  pro|I  v|want  BROKEN  det|a  n|cookie .
```

- `pro|I` → parsed
- `v|want` → parsed
- `BROKEN` → `mor_word_parser` fails → `chunk_parser` fails → `mor_item_parser` fails
- `separated_by()` aborts → entire tier parse fails
- **Result: MOR tier rejected** — parse-taint set for Mor domain

### 5C. Parse-Health Classification

Both parsers use the same `ParseHealth` mechanism to gate downstream alignment:

```rust
// ParseHealth (from talkbank-model)
pub struct ParseHealth {
    pub main_clean: bool,
    pub mor_clean: bool,    // false → skip main↔mor alignment
    pub gra_clean: bool,    // false → skip mor↔gra alignment
    pub pho_clean: bool,    // false → skip main↔pho alignment
    pub wor_clean: bool,    // false → skip main↔wor alignment
    pub mod_clean: bool,
    pub sin_clean: bool,
}
```

The direct parser's `classify_dependent_tier_parse_health()` works even on malformed tier lines (extracts label via byte-level prefix matching), ensuring the right domain is tainted even when the full parse fails.

---

## 6. Parse/Validate Boundary

### What Each Layer Is Responsible For

```
┌─────────────────────────────────────────────────────┐
│  GRAMMAR (tree-sitter)                               │
│  • Structure: headers, tiers, utterances, brackets   │
│  • Token boundaries: words, annotations, bullets     │
│  • Paired delimiters: (), [], <>, "", ‹›, 〔〕        │
│  • ERROR recovery for malformed input                │
└──────────────────────┬──────────────────────────────┘
                       ▼
┌─────────────────────────────────────────────────────┐
│  PARSER (tree-sitter Rust layer / direct parser)     │
│  • CST → model conversion (tree-sitter)              │
│  • Content parsing (direct parser)                   │
│  • Parse-health taint tracking                       │
│  • ERROR/MISSING node analysis                       │
│  • Word construction (prefix, body, markers)         │
│  • MOR/GRA/PHO item parsing                          │
│  Checks: structural integrity, node presence,        │
│          well-formedness of parsed sub-components     │
│  Does NOT check: cross-tier consistency, semantics,  │
│          required headers, speaker validity           │
└──────────────────────┬──────────────────────────────┘
                       ▼
┌─────────────────────────────────────────────────────┐
│  VALIDATION (talkbank-model validation layer)        │
│  Runs after parsing produces a ChatFile.             │
│  Requires file-level context (participants,          │
│  languages, CA mode).                                │
│                                                      │
│  header/     Required headers (E502-E504),           │
│              speaker declarations, @Bg/@Eg scope     │
│  word/       Illegal characters, language codes,     │
│              digit legality per language              │
│  main_tier   E371 (pauses in pho groups),            │
│              E372 (nested quotations)                 │
│  utterance/  CA delimiter balance, overlap indices,  │
│              underline balance, duplicate tiers       │
│  temporal    E701 (timeline monotonicity),           │
│              E704 (self-overlap)                      │
│  cross_utt/  Quotation continuation, completion      │
│              pairing, terminator-linker matching      │
│  alignment   E714/E715 (main↔mor count),             │
│              E600 (gra indices), pho/wor alignment    │
│              — GATED by ParseHealth                   │
└─────────────────────────────────────────────────────┘
```

### What Each Parser Validates During Parsing

| Check | Tree-Sitter Parser | Direct Parser | Validation Layer |
|-------|-------------------|---------------|-----------------|
| File structure (headers present) | CST structure | chumsky structure | E502, E503, E504 |
| Speaker codes format | Node extraction | chumsky parser | `has_invalid_speaker_chars()` |
| Speaker in @Participants | No | No | `SpeakerNotDefined` |
| Word well-formedness | ERROR node analysis | chumsky `try_map` | `word/structure.rs` |
| Language code format | Node extraction | chumsky parser | `word/language/resolve.rs` |
| Language code declared | No | No | ~~E254~~ (removed, Decision 3) |
| MOR structure | CST walking, item-skip on error | chumsky parser, tier-fail on error | Alignment gating |
| GRA indices valid | CST field extraction | `text::int()` parsing | E600 (index out of range) |
| Tier alignment counts | No | No | E714, E715 (gated by ParseHealth) |
| Overlap balance | No | No | E347, E348 (not yet implemented) |
| Temporal monotonicity | No | No | E701, E704 |
| CA delimiter balance | No | No | E535+ |
| Cross-utterance pairing | No | No | `cross_utterance/` module |

---

## 7. Leniency Comparison

### Where Tree-Sitter Parser Is More Lenient

1. **Word-level recovery**: Preserves partially-parsed words; direct parser rejects entirely
2. **Content sequence continuation**: Skips ERROR items and continues; direct parser aborts content on first bad item
3. **Dependent tier partial population**: Skips bad MOR/GRA items; direct parser rejects entire tier
4. **ERROR-as-word fabrication**: Creates Word from ERROR text that looks word-like (`body.rs:306-331`); direct parser never fabricates

### Where Direct Parser Is More Lenient

1. **Annotation fallback**: Unknown annotations parsed as explanation text (`main_tier.rs:1360-1374`); tree-sitter depends on grammar accepting the annotation
2. **Pause/shortening disambiguation**: Lookahead prevents partial matches (`main_tier.rs:643`); tree-sitter relies on grammar precedence

### Both Parsers Are Equally Lenient On

1. **Optional elements**: Terminators, language codes, bullets, linkers, postcodes are all optional in both
2. **Parse-health taint**: Both use identical `ParseHealth` mechanism
3. **Dependent tier boundary recovery**: Both discard bad dependent tiers and continue with the file
4. **"Parse, don't validate"**: Neither parser checks cross-tier consistency, temporal ordering, or speaker declarations

### Leniency Policy Alignment

Both parsers implement the three-tier leniency classification from `leniency-policy.md`:
- **Tier A** (parse-lenient + validate-strict): Grammar/parsers accept; validation catches (E305, E503, etc.)
- **Tier B** (parse-lenient + validate-warning): Grammar/parsers accept; validation warns
- **Tier C** (parse-lenient only): Genuinely optional or by design

The 8 permissiveness regression decisions (E214, E248, E254, etc.) apply to the validation layer, not the parsers.

---

## 8. Direct Parser Recovery Roadmap

Chumsky 0.12 ships with a full `recovery` module — `skip_until`, `skip_then_retry_until`, `nested_delimiters`, `via_parser`, and the custom `Strategy` trait — but **none of these are used anywhere in the direct parser**. All recovery is hand-coded at tier boundaries. Adding recovery is straightforward engineering, not a redesign.

### Recovery Fix 1: Fallback Word at Delegation Point (Cheapest Win)

**File:** `main_tier.rs:485` — where word parsing is delegated
**Effort:** ~15 lines
**Impact:** Eliminates "one bad word kills the file"

Currently:
```rust
parse_word_impl(word_text, ...).ok_or_else(|| Rich::custom(..., "Failed to parse word"))
```

Proposed:
```rust
match parse_word_impl(word_text, offset + span.start, &errors) {
    ParseOutcome::Parsed(word) => Ok(word),
    ParseOutcome::Rejected => {
        // Propagate collected errors to outer sink
        for error in errors.to_vec() { outer_errors.report(error); }
        // Create fallback Word with raw text only
        Ok(Word::new_unchecked(word_text, word_text)
            .with_span(Span::from_usize(offset + span.start, offset + span.end)))
    }
}
```

This mirrors the tree-sitter parser's `recover_error_as_word()` pattern. The word survives as raw text. Errors are still reported. The utterance survives. The file survives.

### Recovery Fix 2: Word-Internal `.or()` Fallback

**File:** `word.rs:84-138` — the word structure parser
**Effort:** ~20 lines
**Impact:** Word parser itself never fails

Add a recovery branch after the main parse chain:
```rust
category.then(body).then(form).then(lang).then(pos)
    .map_with(|..., extra| { /* build full Word */ })
    .or(
        any().repeated().at_least(1).to_slice()
            .map_with(move |raw, extra| {
                Word::new_unchecked(raw, raw)
                    .with_span(Span::from_usize(extra.span().start + offset, extra.span().end + offset))
            })
    )
```

With this, `word_parser()` always returns `Ok` — it either produces a fully-parsed Word or a raw-text-only Word.

### Recovery Fix 3: Content-Sequence `recover_with()`

**File:** `main_tier.rs` — content item parser
**Effort:** ~50 lines
**Impact:** Failed content items (words, groups, events) are skipped; parsing continues at next whitespace boundary

```rust
content_item_parser()
    .recover_with(skip_then_retry_until([' ', '\t', '\n']))
```

This uses chumsky's built-in recovery. A completely unparseable content item is skipped, and the parser retries from the next whitespace-delimited token.

### Recovery Fix 4: Dependent Tier Item-Level Recovery

**Files:** `mor_tier.rs`, `gra_tier.rs`, `pho_tier.rs`, `sin_tier.rs`
**Effort:** ~30 lines per tier
**Impact:** One bad morpheme/relation no longer kills the entire tier

All four tiers use `separated_by()` which aborts on the first bad item. Replace with recovery-aware parsing:

```rust
mor_item_parser()
    .recover_with(skip_then_retry_until([' ', '\t']))
    .separated_by(ws_parser())
    .at_least(1)
```

This brings the direct parser to parity with the tree-sitter parser's behavior of partial tier population.

### Recovery Summary

| Fix | Effort | Impact | Prerequisite for Grammar Coarsening? |
|-----|--------|--------|--------------------------------------|
| 1. Fallback word at delegation | ~15 lines | Eliminates file-kill from bad word | **Yes** — required for Phase 2 (word coarsening) |
| 2. Word-internal `.or()` fallback | ~20 lines | Word parser never fails | Nice-to-have (Fix 1 is sufficient) |
| 3. Content-sequence `recover_with()` | ~50 lines | Skip unparseable content items | **Yes** — required for Phase 3 (contents simplification) |
| 4. Dependent tier item recovery | ~120 lines (4 tiers) | Partial tier population | Desirable but not blocking |

**Fixes 1 and 3 together (~65 lines) unlock the grammar coarsening.** With these, the direct parser has sufficient resilience for the proposed architecture where tree-sitter provides coarse structure and the direct parser handles fine-grained content.

---

## 9. Implications for Grammar Redesign

With the recovery roadmap in section 8, the direct parser can be made resilient enough to support coarse tree-sitter tokens. The previous concern — "one bad word kills the file" — is eliminated by Recovery Fix 1 (~15 lines of code).

The recommended strategy is no longer "selective coarsening" but **full coarsening with recovery prerequisites**:

1. Implement Recovery Fix 1 (fallback word) — unblocks word coarsening
2. Implement Recovery Fix 3 (content-sequence recovery) — unblocks contents simplification
3. Optionally implement Recovery Fix 4 (dependent tier items) — improves quality
4. Proceed with all grammar redesign phases

See [grammar-redesign.md](grammar-redesign.md) for the detailed phased plan.

---

## See Also

- [grammar-redesign.md](grammar-redesign.md) — 6-phase grammar improvement plan
- [grammar-stakeholders.md](grammar-stakeholders.md) — stakeholder analysis for grammar granularity
- [leniency-policy.md](leniency-policy.md) — three-tier leniency classification and permissiveness decisions
- `crates/talkbank-direct-parser/src/file.rs` — direct parser 5-phase pipeline
- `crates/talkbank-direct-parser/src/main_tier.rs` — direct parser content/word parsing
- `crates/talkbank-parser/src/parser/tree_parsing/helpers.rs` — ERROR node analysis
- `crates/talkbank-parser/src/parser/tree_parsing/main_tier/word/` — tree-sitter word recovery
- `crates/talkbank-model/src/model/file/utterance/parse_health.rs` — ParseHealth contract
- `crates/talkbank-model/src/validation/` — post-parse validation layer
