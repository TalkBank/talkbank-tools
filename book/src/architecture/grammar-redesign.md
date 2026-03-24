# Grammar Redesign: Full Coarsening with Direct Parser Recovery

**Status:** Historical
**Last updated:** 2026-03-24 01:32 EDT

> **Note:** This proposal assumed the Chumsky direct parser would serve as the
> sub-parser for fine-grained content analysis. The direct parser was removed
> in March 2026. The grammar coarsening ideas remain relevant but the
> direct-parser-as-subparser premise no longer applies.

## Executive Summary

The tree-sitter grammar (`grammar/grammar.js`) has grown to 1,990 lines generating
a 26,687-line `parser.c` with 5 declared conflicts and ~370 node types. Only ~80
of those node types are consumed by any editor feature. The rest exist solely for
the tree-sitter parser's CST-to-model walk — a function the direct parser already
performs with better error messages and no grammar conflicts.

**The plan:** Make tree-sitter tokens radically coarser, add targeted recovery to
the direct parser (~205 lines of Rust), and use the direct parser as a subparser
library for all fine-grained content analysis. This reduces grammar complexity by
~50% while *improving* editor features through richer semantic tokens.

The plan has 7 phases (Phase 0 + Phases 1–6) with explicit dependency ordering.
Phase 0 (direct parser recovery) unblocks everything else and can ship independently.

---

## Current State

### Grammar Metrics

| Metric | Value |
|--------|-------|
| grammar.js lines | 1,990 |
| parser.c lines | 26,687 |
| Declared conflicts | 5 |
| Total node types | ~370 |
| Node types used by editor | ~80 |

### Top State Consumers

| Rule | States | Notes |
|------|--------|-------|
| `contents_repeat1` | 67 | 18-alternative repeat for content sequencing |
| `contents` | 65 | Entry point for main tier content |
| `utterance_end` | 54 | Terminator + postcodes + bullet + newline |
| `tier_body` | 43 | Main tier body structure |
| `types_header` | 32 | @Types header with comma-separated fields |
| `languages_contents_repeat1` | 25 | Language code lists |
| `_id_demographic_fields` | 24 | @ID header demographic section |

### Declared Conflicts

```js
conflicts: $ => [
  [$.contents],
  [$.word_with_optional_annotations],
  [$.nonword_with_optional_annotations],
  [$.base_annotations],
  [$.final_codes],
],
```

---

## What Tree-Sitter Is Actually Used For

### Editor Feature Usage Inventory

Of ~441 node types in the grammar, only ~80 are referenced by any editor feature:

**highlights.scm (~48 node types):** Syntax coloring via tree-sitter queries,
consumed by the `talkbank-highlight` crate and the LSP's semantic tokens provider.

**LSP navigation (~15 node types):**
- Go-to-definition: `speaker`, `participants_header`, `participant`, `mor_dependent_tier`, `gra_dependent_tier`, `mor_contents`, `mor_content`, `gra_contents`, `gra_relation`
- Hover/alignment: `mor_content`, `pho_group`, `sin_group`, `gra_relation`
- Completion: `speaker`, `postcode_prefix`, all 28 `*_tier_prefix` constants

**VS Code extension:** Pure LSP consumer. Does NOT use tree-sitter directly. Has a
TextMate grammar for fallback colorization and regex-based decorations for bullet
markers.

**textobjects.scm, folds.scm, indents.scm, tags.scm:** Declarative queries for
Neovim/Helix. Use ~20 structural node types (utterance, header, dependent_tier,
word_with_optional_annotations, etc.). These benefit from coarse structural nodes
and are unaffected by internal content coarsening.

### The ~290 Unused Node Types

These exist solely for `talkbank-parser`'s CST-to-model walk:

- All `ca_*` nodes (~20 types) — not highlighted, not navigated
- `word_body`, `word_segment`, `word_content`, `word_content_nontext`
- `inline_bullet`, `inline_pic`, `media_url` internals
- All annotation internals (`explanation_annotation`, `para_annotation`, etc.)
- `long_feature_*`, `nonvocal_*`
- Header content nodes (`id_age`, `id_corpus`, `id_sex`, etc.)
- `pause_duration`, `pause_duration_with_decimal`
- `mor_compound_word`, `mor_pre_clitic`, `mor_post_clitic`, `mor_fusional_*`

### Current Editor Gaps

Constructs that **would** benefit from editor highlighting but aren't wired up:

- CA elements and delimiters (`°`, `∆`, `↑`, `↓`, etc.) — not fully in highlights.scm
- Overlap markers (`⌈`, `⌉`, `⌊`, `⌋`) — basic highlighting only
- Linkers (`++`, `+<`, `+^`, etc.) — not highlighted
- Word sub-structure (shortenings, stress markers, prosodic colons) — entire word is one `@string` capture
- Inline bullets — handled by VS Code regex decoration, not tree-sitter

The irony: the grammar produces extremely fine-grained structure that the editor
never uses, while the editor features that would benefit from fine-grained analysis
aren't implemented. The direct parser already provides the analysis — it just needs
a delivery mechanism (semantic tokens).

---

## What Each Parser Does Best

### Tree-Sitter: Structure

1. **Incremental reparsing** — only re-parses changed regions on each keystroke
2. **Error recovery** — produces partial trees even with syntax errors
3. **Query language** — `highlights.scm` for declarative pattern matching
4. **Line-level structure** — document, header, utterance, dependent tier boundaries
5. **Bracket matching** — `<>`, `[]`, `()`, `""` group boundaries

### Direct Parser: Content

1. **Fine-grained content parsing** — full word structure, all CA markers, prosody, shortenings
2. **No grammar conflicts** — chumsky combinators don't have LR ambiguity
3. **Better error messages** — precise locations with span information
4. **Reusable as subparsers** — `(input: &str, offset: usize, errors: &ErrorSink) -> Model`
5. **Already proven** — handles 73 reference corpus files at 100% roundtrip

### Existing Subparser Pattern

The tree-sitter parser already does post-hoc subparsing:
`push_word_segment_with_inline_ca_delimiters()` in `content.rs` takes `word_segment`
text from the CST, scans for CA delimiter characters, and splits into `Text` +
`CADelimiter` segments. This is exactly the pattern we generalize.

---

## Proposed Architecture

### Current

```
keystroke
  --> tree-sitter incremental reparse --> fine-grained CST (~441 node types)
        |
        |--> highlights.scm queries --> semantic tokens (coarse coloring)
        |--> CST node.kind() walks --> go-to-def, hover, completion
        |--> talkbank-parser CST walk --> model --> validation
```

### Proposed

```
keystroke
  --> tree-sitter incremental reparse --> coarse CST (~150 node types)
        |
        |--> highlights.scm on coarse nodes --> tier-level coloring (fast, instant)
        |--> textobjects/folds/indents     --> structural editing (unchanged)
        |--> coarse node text --> direct parser --> fine-grained model
                                                      |
                                      |--> rich semantic tokens (sub-word coloring)
                                      |--> hover with full structure
                                      |--> go-to-def via model alignment
                                      |--> completion with semantic context
                                      |--> inline diagnostics

Tree-sitter owns structure. Direct parser owns content. LSP bridges them.
```

### Semantic Token Overlay

Tree-sitter queries provide fast, approximate coloring on every keystroke. The LSP's
semantic tokens provider overlays finer-grained tokens by running the direct parser
on visible-range tier content. Semantic tokens override query-based tokens:

1. **Instant coarse coloring** from tree-sitter queries (incremental, < 1ms)
2. **Rich fine-grained coloring** from direct parser (slightly delayed, visible range only)

This is how mature LSPs work (e.g., rust-analyzer uses tree-sitter for fast syntax
coloring, then overlays precise semantic tokens from the compiler).

### What This Enables for the Editor

**1. Sub-word Highlighting**

Currently `°wo:rd↑°` gets one `@string` color. With the direct parser:
- Extract `standalone_word` token text from coarse CST node
- Run `parse_word_impl()` → `[CADelimiter(Softer), Text("wo"), Lengthening, Text("rd"), CAElement(PitchUp), CADelimiter(Softer)]`
- Emit separate semantic tokens: CA delimiters, prosodic markers, base text each in their own color

**2. Alignment-Based Navigation**

Currently the LSP counts `mor_content` children in the CST. With the direct parser:
- Parse %mor tier text → `Vec<MorWord>` with byte offsets
- Parse main tier text → aligned words with byte offsets
- Map directly between model items — no fragile child-counting

**3. Hover with Full Structural Context**

Hover on a %mor word gives prefix, POS, subcategories, stem, suffixes, translation
— directly from the chumsky parse, not by walking 15 CST child nodes.

**4. Context-Aware Completion**

The direct parser's recovery (after Fix 2) gives partial parse results: "you're
inside a mor_word, you've typed the POS and pipe, the stem is expected next."

**5. Real-Time Diagnostics on Changed Regions**

Tree-sitter identifies which nodes changed. Extract text, run direct parser, get
precise diagnostics — faster than full-file validation.

---

## Phased Implementation Plan

### Dependency Graph

```
Phase 0: Direct Parser Recovery
  ├── Fix 1 (fallback word)  ──────────► Phase 2 (word coarsening)
  ├── Fix 3 (content recovery) ────────► Phase 3 (contents simplification)
  └── Fix 4 (dep tier items)  ────────► Phase 4 (tier merge, optional)
                                         Phase 7 (%mor coarsening, optional)

Phase 1 (media bullets)    — no prerequisites, can run in parallel with Phase 0
Phase 4 (text tier merge)  — no prerequisites beyond Fix 4 if desired
Phase 5 (bracket annots)   — no prerequisites
Phase 6 (dead rules)       — after all other phases
```

Phases 1, 4, 5 can proceed independently. Phase 2 requires Fix 1. Phase 3
requires Fix 3. Phase 6 is cleanup after everything else. Phase 7 (%mor) is
optional and comes last.

---

### Phase 0: Direct Parser Recovery (Prerequisite)

**Effort:** ~205 lines of Rust across 6 files
**Deliverable:** Direct parser resilient enough to handle malformed input gracefully

This phase has no grammar changes. It's purely Rust work in `talkbank-direct-parser`.
It can ship as a standalone improvement — better error messages, fewer file-level
rejections — independent of any grammar coarsening.

#### Fix 1: Fallback Word at Delegation Point

**File:** `main_tier.rs:485`
**Lines:** ~15
**Impact:** Eliminates "one bad word kills the file"
**Required for:** Phase 2

```rust
match parse_word_impl(word_text, offset + span.start, &errors) {
    ParseOutcome::Parsed(word) => Ok(word),
    ParseOutcome::Rejected => {
        for error in errors.to_vec() { outer_errors.report(error); }
        Ok(Word::new_unchecked(word_text, word_text)
            .with_span(Span::from_usize(offset + span.start, offset + span.end)))
    }
}
```

#### Fix 2: Word-Internal `.or()` Fallback

**File:** `word.rs:84-138`
**Lines:** ~20
**Impact:** Word parser itself never fails (defense in depth)
**Required for:** Nothing (nice-to-have, layered recovery)

```rust
category.then(body).then(form).then(lang).then(pos)
    .map_with(|..., extra| { /* build full Word */ })
    .or(
        any().repeated().at_least(1).to_slice()
            .map_with(move |raw, extra| {
                Word::new_unchecked(raw, raw)
                    .with_span(Span::from_usize(
                        extra.span().start + offset,
                        extra.span().end + offset))
            })
    )
```

#### Fix 3: Content-Sequence `recover_with()`

**File:** `main_tier.rs` content item parser
**Lines:** ~50
**Impact:** Failed content items skipped; parsing continues at next whitespace
**Required for:** Phase 3

```rust
content_item_parser()
    .recover_with(skip_then_retry_until([' ', '\t', '\n']))
```

#### Fix 4: Dependent Tier Item-Level Recovery

**Files:** `mor_tier.rs`, `gra_tier.rs`, `pho_tier.rs`, `sin_tier.rs`
**Lines:** ~30 per tier (~120 total)
**Impact:** One bad morpheme/relation no longer kills the entire tier
**Required for:** Nothing (desirable quality improvement)

```rust
mor_item_parser()
    .recover_with(skip_then_retry_until([' ', '\t']))
    .separated_by(ws_parser())
    .at_least(1)
```

#### Phase 0 Verification

After all fixes:
- `cargo test -p talkbank-direct-parser` — existing tests pass
- `cargo nextest run -p talkbank-parser-tests` — equivalence suite passes (model output unchanged for well-formed input)
- Reference corpus roundtrip at 100% — recovery only activates on malformed input
- Manually test with deliberately malformed files to verify recovery behavior

---

### Phase 1: Collapse Media Bullets into Single Tokens

**Prerequisites:** None (can run in parallel with Phase 0)
**Estimated savings:** ~30 states, ~8 rules

**Before:**
```js
inline_bullet: $ => seq(
  $.bullet_end, $.natural_number, $.underscore, $.natural_number, $.bullet_end
)
media_url: $ => seq(
  $.bullet_end, field('start_ms', $.natural_number), $.underscore,
  field('end_ms', $.natural_number), choice($.bullet_end, seq(field('skip', $.hyphen), $.bullet_end))
)
```

**After:**
```js
inline_bullet: $ => token(/\u0015\d+_\d+\u0015/)
media_url: $ => token(/\u0015\d+_\d+-?\u0015/)
```

**Rust side:** Extract token text, parse with `bullet_parser()` from the direct
parser or a simple regex to get start_ms, end_ms, and skip flag.

**Risk:** Low. Bullet format is fixed and never varies.

**Query impact:** `highlights.scm` captures `inline_bullet` and `media_url` as
whole nodes already — no change needed.

---

### Phase 2: Collapse Word Body into a Single Coarse Token

**Prerequisites:** Phase 0 Fix 1 (fallback word)
**Estimated savings:** ~50 states, ~15 rules, 2 conflicts eliminated

**Before:** `word_body` is a `choice` of sequences containing `initial_word_segment`,
`word_content` (12-way choice), `shortening`, `stress`, `colon`, `caret`, `tilde`,
`plus`, `overlap_point`, `ca_element`, `ca_delimiter`, `underline_begin/end`.

**After:** `standalone_word` becomes a single coarse token capturing everything from
the first word character to the word boundary (before `@s`, `@z`, `$`, whitespace,
`[`, terminator).

```js
// Word boundary: stop before language marker, form marker, POS tag,
// whitespace, annotation bracket, or terminator
standalone_word: $ => token(
  /[^\s\[\]<>\u0015.!?\u2026]+/  // simplified — actual regex needs careful design
)
```

**Rust side:** Run `parse_word_impl()` from the direct parser on the token text.
With Fix 1, a malformed word produces a raw-text Word instead of a parse failure.

**Risk:** Medium → **Low** (with Fix 1). The previous "high risk" assessment was
based on the direct parser's fail-fast behavior. Fix 1 eliminates this: if the
direct parser can't fully parse the word, it falls back to raw text — exactly what
tree-sitter's ERROR recovery does today.

**Word boundary regex design** is the remaining risk. The direct parser already
handles word boundaries in `main_tier.rs` via chumsky combinators — we need to
express the same boundaries as a tree-sitter token regex. This requires careful
testing against the reference corpus.

**Conflicts eliminated:**
- `[$.word_with_optional_annotations]` — gone (word is a single token)
- `[$.nonword_with_optional_annotations]` — simplified (fewer ambiguities)

---

### Phase 3: Simplify the `contents` Rule

**Prerequisites:** Phase 0 Fix 3 (content-sequence recovery)
**Estimated savings:** ~80 states, ~5 rules, 3–4 conflicts eliminated

**Before:** 18 alternatives in the `repeat(choice(...))` enumerating all permutations
of whitespace, separator, overlap, and content ordering.

**After:**
```js
contents: $ => repeat1(choice(
  $.whitespaces,
  $.content_item,
  $.separator,
  $.overlap_point,
))
```

**Rust side:** Validate ordering constraints (separator must be whitespace-delimited,
etc.) during model construction. The direct parser's `parse_main_tier()` already
enforces these constraints. With Fix 3, unparseable content items are skipped rather
than killing the file.

**Risk:** Medium. Need to verify that flattening doesn't introduce new ambiguities
with existing token precedences. The `overlap_point` / `separator` / `content_item`
interaction is the most delicate area. However, with a coarser word token (Phase 2),
many of these interactions disappear.

**Conflicts eliminated:**
- `[$.contents]` — gone (no longer 18-way choice)
- `[$.separator, $.contents]` — gone (separator is just one option in flat choice)
- `[$.whitespaces]` — likely gone (whitespace handling simplified)

**Phase 2 + Phase 3 synergy:** These two phases reinforce each other. A coarse word
token (Phase 2) dramatically simplifies `contents` because the grammar no longer
needs to disambiguate word sub-components from other content items. Implementing them
together may be easier than doing them separately.

---

### Phase 4: Merge Text-Based Dependent Tiers

**Prerequisites:** None (Phase 0 Fix 4 is desirable but not required)
**Estimated savings:** ~40 states, ~50 rules

**Before:** 20+ tiers that are all `prefix + tier_sep + text_with_bullets + newline`,
each with a separate prefix token, tier code token, and tier rule.

**After:**
```js
text_dependent_tier: $ => seq(
  alias(
    token(/%(?:act|add|err|exp|gpx|sit|tim|alt|coh|def|fac|flo|par|spa|cod|gls|eng|int|ort)/),
    $.text_tier_prefix
  ),
  $.tier_sep,
  $.text_with_bullets,
  $.newline
)
```

**Rust side:** Dispatch on prefix text to determine tier type. Already works this
way in the direct parser's byte-oriented prefix dispatch.

**Query impact options:**
- (a) Single capture, all text tiers same color — simplest
- (b) Keep individual tier aliases so queries can distinguish — more complex but preserves differentiation
- (c) Use LSP semantic tokens to override with tier-specific colors — aligns with our overall architecture

Recommendation: (c) for the long term, (a) for the initial implementation.

**textobjects.scm impact:** Currently `com_dependent_tier` and `exp_dependent_tier`
have `@comment.outer` captures. With merged tiers, capture the generic
`text_dependent_tier` and let the LSP provide semantic differentiation.

**Risk:** Low for the structural merge.

---

### Phase 5: Collapse Bracket Annotations into Tokens + Subparser

**Prerequisites:** None
**Estimated savings:** ~20 states, ~10 rules, 1 conflict eliminated

**Before:** `explanation_annotation`, `para_annotation`, `alt_annotation`,
`percent_annotation`, `error_marker_annotation` each decompose `[X content]` into
5–6 nodes.

**After:**
```js
bracket_annotation: $ => token(/\[[=!?%*#][^\]]*\]/)
```

**Rust side:** Match on prefix character to determine annotation type, extract
content. The direct parser's annotation parsing in `main_tier.rs` already handles
all types, with the useful fallback that unknown annotations parse as explanation
text (`main_tier.rs:1360-1374`).

**Risk:** Low. Annotation format is rigid: `[PREFIX CONTENT]`. The only subtlety is
`[*]` (no space, no content) vs `[* code]`.

**Conflict eliminated:** `[$.base_annotations]` — gone (single token, no ambiguity).

---

### Phase 6: Delete Dead Rules and Consolidate

**Prerequisites:** After all other phases
**Estimated savings:** ~10 states, ~30 rules

- Delete all 28 `xxx_tier_code` rules (`mor_tier_code: $ => 'mor'`, etc.) — never
  referenced by any rule
- Consolidate `header_sep` and `tier_sep` — identical definitions
- Remove `content_item: $ => $.core_content` (trivial indirection)
- Remove word sub-structure rules made dead by Phase 2 (`word_body`, `word_segment`,
  `word_content`, `word_content_nontext`, `shortening`, `stress`, etc.)
- Remove annotation sub-structure rules made dead by Phase 5

**Risk:** None. Dead code and trivial aliases.

---

### Phase 7 (Optional): Coarsen %mor Tier

**Prerequisites:** Phase 0 Fix 4 (dependent tier item recovery)
**Estimated savings:** ~50 states, ~15 rules, 1–2 conflicts eliminated

The %mor tier grammar is a special case. The previous plan recommended keeping it
structured ("Option A"). With recovery Fix 4 in place and the semantic token overlay
architecture proven by Phases 1–6, coarsening %mor becomes viable and attractive.

**Before:** ~15 grammar rules decompose MOR items into `mpos`, `stem`,
`mor_prefix`, `mor_suffix`, `mor_fusional`, etc.

**After:**
```js
mor_content: $ => token(/[^\s]+/)  // each whitespace-delimited MOR item
```

**Rust side:** Run `parse_mor_word_content()` on each token. With Fix 4, a malformed
morpheme is skipped rather than killing the tier.

**Why now viable:**
- Semantic tokens overlay can provide sub-morpheme coloring (POS, stem, suffixes)
  more richly than tree-sitter queries ever could
- LSP alignment no longer depends on CST child-counting (uses model alignment)
- Fix 4 ensures one bad morpheme doesn't kill the tier

**Why optional:**
- The %mor grammar is stable and doesn't cause maintenance pain
- `highlights.scm` already colors POS, stems, and suffixes differently
- If the editor experience is good enough without this phase, skip it

**Conflicts eliminated:** `[$.mor_prefixes]`, `[$.mor_category, $.stem]`

---

## Impact Summary

### Grammar Reduction

| Phase | States Saved | Rules Removed | Conflicts Eliminated |
|-------|-------------|---------------|---------------------|
| 0. Direct parser recovery | 0 | 0 | 0 |
| 1. Media bullets | ~30 | ~8 | 0 |
| 2. Word body | ~50 | ~15 | 2 |
| 3. Simplify contents | ~80 | ~5 | 3–4 |
| 4. Merge text tiers | ~40 | ~50 | 0 |
| 5. Bracket annotations | ~20 | ~10 | 1 |
| 6. Dead rules | ~10 | ~30 | 0 |
| 7. %mor (optional) | ~50 | ~15 | 1–2 |
| **Total (with Phase 7)** | **~280** | **~133** | **7–9 of 11** |
| **Total (without Phase 7)** | **~230** | **~118** | **6–7 of 11** |

### Expected Results

| Metric | Before | After (Phases 0–6) | After (Phases 0–7) |
|--------|--------|--------------------|--------------------|
| grammar.js lines | 1,990 | ~1,200 | ~1,050 |
| parser.c lines | 26,687 | ~15,000 | ~12,000 |
| Node types | ~370 | ~150 | ~135 |
| Declared conflicts | 5 | 2–3 | 1–2 |
| Editor node types used | ~80 | ~80 (semantic tokens replace CST nodes) | ~80 |
| `tree-sitter generate` time | ~2s | ~1s | ~0.8s |

### Rust Work

| Component | Effort | Impact |
|-----------|--------|--------|
| Phase 0 (recovery fixes) | ~205 lines, 6 files | Direct parser resilient for any input |
| `talkbank-parser` updates | ~200 lines per phase | CST walk updated to extract token text → direct parser |
| LSP semantic tokens provider | ~300 lines (new) | Sub-word, sub-morpheme, sub-annotation coloring |
| LSP navigation updates | ~100 lines | Model-based instead of CST-child-based |
| `highlights.scm` updates | ~50 lines per phase | Coarser captures, tier-level coloring |
| `textobjects.scm` updates | ~20 lines total | Minimal — already uses structural nodes |

---

## Migration Strategy

### Per-Phase Verification Protocol

Each phase independently verifiable:

1. Implement the grammar change (coarser tokens)
2. `cd grammar && tree-sitter generate`
3. `cd grammar && tree-sitter test`
4. Update `talkbank-parser` to extract token text and call direct parser
5. `cargo test -p talkbank-parser`
6. `cargo nextest run -p talkbank-parser-tests` — **the critical safety net**
7. `./no-cache-ref-test.sh` — reference corpus roundtrip at 100%
8. Update `highlights.scm`, verify coloring manually
9. Update LSP code if navigation/completion affected
10. Verify LSP features in VS Code

The `talkbank-parser-tests` equivalence suite ensures both parsers produce identical
model output regardless of CST structure changes.

### Recommended Execution Order

```
Sprint 1: Phase 0 (recovery) + Phase 1 (bullets)
  ├── Fix 1, Fix 2 in parallel with Phase 1
  ├── Fix 3, Fix 4 in parallel
  └── Ship: improved direct parser + simpler bullets

Sprint 2: Phase 2 + Phase 3 (word + contents)
  ├── Phase 2 first (word coarsening)
  ├── Phase 3 immediately after (contents simplification)
  └── Ship: major grammar reduction, semantic token overlay prototype

Sprint 3: Phase 4 + Phase 5 (tiers + annotations)
  ├── Independent, can be done in either order
  └── Ship: grammar mostly coarsened

Sprint 4: Phase 6 (cleanup) + Phase 7 (optional %mor)
  ├── Phase 6: delete dead rules
  ├── Phase 7: only if semantic tokens are working well
  └── Ship: final state
```

### Rollback Strategy

Each phase is independently revertable because:
- Grammar changes are in `grammar.js` (single file)
- Rust parser updates are in `talkbank-parser` (single crate)
- The direct parser changes (Phase 0) are purely additive recovery paths
- `talkbank-parser-tests` catches any model regression immediately

If a phase causes problems, revert the grammar.js change, re-run
`tree-sitter generate`, revert the Rust parser update, and the system is back to
the previous state.

---

## Direct Parser Capabilities Available as Subparsers

| Construct | Direct Parser Function | Current Grammar Rules | Coarsened In |
|-----------|----------------------|----------------------|-------------|
| Words | `parse_word_impl()` | ~15 rules | Phase 2 |
| Media bullets | `bullet_parser()` | 5-node seq | Phase 1 |
| %mor words | `parse_mor_word_content()` | ~15 rules | Phase 7 |
| %gra relations | `parse_gra_relation_content()` | 5-node seq | Phase 7 |
| %pho words | `parse_pho_tier_content_with_type()` | pho_word regex + group | (stable) |
| %sin words | `parse_sin_tier_content()` | sin_word + group | (stable) |
| %wor timing | `parse_wor_tier_content()` | wor_tier_body multi-node | (stable) |
| Text tiers | `text_tier_content_parser()` | text_with_bullets | Phase 4 |
| Main tier content | `parse_main_tier()` | contents (132 states) | Phase 3 |
| Annotations | various in `main_tier.rs` | 5–6 nodes each | Phase 5 |
| Headers | `parse_header_impl()` | per-header content rules | (stable) |

All these functions are available as methods on `TreeSitterParser`:
`parse_word_fragment()`, `parse_main_tier_fragment()`,
`parse_chat_file()`, etc.

---

## See Also

- [parser-error-recovery.md](parser-error-recovery.md) — full recovery audit and fix designs
- [grammar-stakeholders.md](grammar-stakeholders.md) — stakeholder analysis for grammar granularity
- [leniency-policy.md](leniency-policy.md) — three-tier leniency classification
- `grammar/grammar.js` — current grammar definition
- `grammar/queries/highlights.scm` — syntax highlighting queries
- `crates/talkbank-direct-parser/` — chumsky-based direct parser
- `crates/talkbank-parser/` — tree-sitter CST walker
- `crates/talkbank-lsp/` — LSP server
- `crates/talkbank-highlight/` — syntax highlighting crate

---

*Last updated: 2026-02-18*
