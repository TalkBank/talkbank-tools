# Alignment Architecture in talkbank-tools

**Status:** Current
**Last updated:** 2026-03-26 10:33 EDT

This document describes all alignment data structures, algorithms, and
design decisions in the `talkbank-tools` crate workspace.

`batchalign3` is the main downstream consumer for these APIs. Its `morphotag`,
`align`, `compare`, `benchmark`, and forced-alignment flows all depend on the
alignment contracts described here, so the model-layer alignment surfaces are
the stable boundary to preserve.

## Overview

Tier alignment in CHAT validates that dependent tiers (%mor, %pho, %wor, %sin,
%gra) have the correct number and arrangement of items relative to the main
tier. The alignment module in `talkbank-model` provides:

1. **Domain-aware counting** — how many alignable items does a main tier have
   for each dependent tier type?
2. **Positional mapping** — which main-tier position corresponds to which
   dependent-tier position?
3. **Mismatch diagnostics** — human-readable error reports when counts disagree.
4. **Content traversal** — a shared walker primitive that centralizes the 24+22
   variant recursion.
5. **Trait abstractions** — `IndexPair`, `TierAlignmentResult`, `AlignableTier`,
   and `TierCountable` formalize the shared contracts; `positional_align()`
   eliminates duplication across %pho/%sin/%wor alignment.

## Module Map

```
crates/talkbank-model/src/alignment/
  mod.rs              Public API: re-exports all alignment functions and types
  traits.rs           Trait abstractions (IndexPair, TierAlignmentResult,
                        AlignableTier, TierCountable) + positional_align()
  types.rs            AlignmentPair — the universal index-pair primitive
  mor.rs              align_main_to_mor() — Main tier → %mor items
  pho.rs              align_main_to_pho() — Main tier → %pho tokens (via AlignableTier)
  sin.rs              align_main_to_sin() — Main tier → %sin tokens (via AlignableTier)
  wor.rs              align_main_to_wor() — Main tier → %wor tokens (via AlignableTier)
  gra/                align_mor_to_gra() — %mor chunks → %gra relations
    align.rs          Core alignment logic (uses to_chat_display_string for diagnostics)
    types.rs          GraAlignmentPair, GraAlignment (implements IndexPair, TierAlignmentResult)
    tests.rs          Unit tests
  phon.rs             Phon tier-to-tier alignment (%modsyl↔%mod, %phosyl↔%pho, %phoaln↔both)
  format.rs           Diagnostic formatting for alignment mismatches
  helpers/
    mod.rs            to_chat_display_string() — shared WriteChat→String helper
    domain.rs         TierDomain enum (Mor/Pho/Sin/Wor)
    rules.rs          Predicate functions (counts_for_tier, should_skip_group, etc.)
    count.rs          Counting and extraction over content trees
    walk/             Content walker (walk_words / walk_words_mut)
      mod.rs          Walker implementation, WordItem/WordItemMut enums
      tests.rs        Walker unit tests
    overlap.rs        Overlap marker iteration (walk_overlap_points, extract_overlap_info)
    overlap_groups.rs Cross-utterance overlap groups (analyze_file_overlaps, 1:N matching)
    tests.rs          Helper unit tests
  location_tests.rs   Alignment location tests
```

## Shared Helpers

### `to_chat_display_string()` (`helpers/mod.rs`)

Renders any `WriteChat` value into owned text for diagnostic messages.
Used by all 5 alignment modules (mor, pho, sin, wor, gra) to avoid
duplicating the `write_chat` → `String` pattern. Best-effort: write
failures are silently ignored because diagnostic formatting must never
panic the alignment path.

### Downstream Expectations

Batchalign3 should treat these helpers as the stable alignment surface:

- `%mor` and `%gra` alignment for morphotag/compare-style workflows
- `%wor` alignment and timing mapping for forced alignment workflows
- `%pho`, `%mod`, `%modsyl`, `%phosyl`, and `%phoaln` tier-to-tier alignment
  for phonological and derived-tier consumers
- shared count/walk helpers for any workflow that needs to reason about
  dependent tier cardinality before alignment

That means a downstream workflow should not reimplement word counting or
positional mapping over raw CHAT text when the model layer already provides a
typed helper.

## Core Types

### TierDomain (`helpers/domain.rs`)

```rust
enum TierDomain { Mor, Pho, Sin, Wor }
```

Each dependent tier applies different alignment eligibility rules over the same
main-tier content. The domain enum makes these policy branches explicit.

**Domain-specific behaviors:**

| Rule | Mor | Pho | Sin | Wor |
|------|-----|-----|-----|-----|
| Skip retrace groups ([details](../chat-format/retraces.md#alignment-behavior)) | Yes | No | No | No |
| Count pauses | No | Yes | No | No |
| PhoGroup handling | Recurse | Atomic (1) | Skip (0) | Recurse |
| SinGroup handling | Recurse | Skip (0) | Atomic (1) | Recurse |
| Include fragments | No | Yes | Yes | Partial |
| Include untranscribed | No | Yes | Yes | No |
| Include tag-marker separators | Yes | No | No | No |
| ReplacedWord aligns to | Replacement | Original | Original | Replacement |

### Retrace Filtering

Retraces are the most alignment-critical content type. A `Retrace` node
wraps content the speaker said then corrected. The alignment rule:

- **%mor (Mor domain):** Skip entirely. Returns count `0`. The retrace was
  a false start; only the correction carries morphological analysis.
- **%pho, %sin, %wor:** Recurse into the retrace content. The words were
  physically produced and have phonological/timing/gestural data.

This is implemented in `count_alignable_item()` and `walk_words()`. Both
check `domain == TierDomain::Mor` on the `UtteranceContent::Retrace`
variant and skip or recurse accordingly.

**Critical invariant:** The parser must emit `UtteranceContent::Retrace`
for *all* retrace patterns, including single-word retraces with
replacements (`word [: repl] [* err] [//]`). If a retrace is
accidentally emitted as a bare `ReplacedWord`, it will be counted for
%mor alignment, causing false E705 errors. This invariant is enforced by
`tests/retrace_replaced_word_regression.rs`.

For the full retrace data model, parsing pipeline, and CHAT examples,
see [Retraces and Repetitions](../chat-format/retraces.md).

### AlignmentPair (`types.rs`)

```rust
struct AlignmentPair {
    source_index: Option<usize>,
    target_index: Option<usize>,
}
```

The universal positional mapping entry. `Some`/`Some` = concrete 1:1 match.
One `None` = placeholder preserving mismatch shape for diagnostics.

Methods:
- `is_complete()` — true when both indices are `Some` (eligible for downstream joins)
- `is_placeholder()` — true for unmatched positions (mismatch rows)

### Per-Domain Results

Each alignment function returns a domain-specific result struct containing
`Vec<AlignmentPair>` and error details:

| Type | Source | Target |
|------|--------|--------|
| `MorAlignment` | Main tier words | %mor items |
| `PhoAlignment` | Main tier words | %pho tokens |
| `SinAlignment` | Main tier words | %sin tokens |
| `WorAlignment` | Main tier words | %wor tokens |
| `GraAlignment` | %mor chunks | %gra relations |
| `PhoAlignment` | %modsyl words | %mod words (tier-to-tier) |
| `PhoAlignment` | %phosyl words | %pho words (tier-to-tier) |
| `PhoAlignment` | %phoaln words | %mod + %pho words (tier-to-tier) |

**%gra note:** %gra aligns to %mor *chunks*, not *items*. Clitics create
additional chunks (e.g., `pro|it~v|be&PRES` = 2 chunks: pre-clitic + main).

## Counting Algorithm (`helpers/count.rs`)

Two entry points:
- `count_tier_positions(content, domain)` — total count for preflight checks
- `count_tier_positions_until(content, max_index, domain)` — partial count for LSP hover
- `collect_tier_items(content, domain)` — items with text for diagnostics

The counting algorithm traverses `UtteranceContent` (24 variants) and
`BracketedItem` (22 variants) with exhaustive `match` — no catch-all arms.
Each variant is classified per domain:

**Word filtering** (`rules.rs`):
- `counts_for_tier(word, domain)` — the canonical domain gate
- Mor: excludes fragments (`&-`, `&~`, `&+`), untranscribed (`xxx`/`yyy`/`www`), omissions
- Wor: excludes nonwords (`&~`), fragments (`&+`), untranscribed, timing tokens (`123_456`);
  includes fillers (`&-um`)
- Pho/Sin: include everything (all produced speech/gesture)

**Retrace filtering**: `Retrace` is a first-class `UtteranceContent` / `BracketedItem`
variant (not an annotation on `AnnotatedGroup`). The walker skips `Retrace` content
for Mor domain; other domains recurse into it.

**Exclude filtering** (`rules.rs`):
- `should_skip_group(annotations, domain)` — `AnnotatedGroup`/`AnnotatedWord` with
  `[e]` exclude marker skip for Mor

**Separator counting**:
- Only tag markers (`,` `„` `‡`) count, and only in Mor domain
- These map to %mor items: `cm|cm`, `end|end`, `beg|beg`

## Content Walker (`helpers/walk/`)

Centralizes the recursive traversal of content trees. Callers provide only
leaf-handling logic via closures.

### Immutable API

```rust
fn walk_words(
    content: &[UtteranceContent],
    domain: Option<TierDomain>,
    callback: impl FnMut(WordItem<'_>),
)
```

### Mutable API

```rust
fn walk_words_mut(
    content: &mut [UtteranceContent],
    domain: Option<TierDomain>,
    callback: impl FnMut(WordItemMut<'_>),
)
```

### Leaf Types

```rust
enum WordItem<'a> {
    Word(&'a Word),
    ReplacedWord(&'a ReplacedWord),
    Separator(&'a Separator),
}
```

Groups are descended transparently, and `AnnotatedWord`/`Event`/`Action` are
unwrapped automatically. `WordItem::Word` carries only the `&Word` reference.

### Domain Gating

| Domain | Retrace | PhoGroup | SinGroup |
|--------|---------|----------|----------|
| `Some(Mor)` | **Skip** | Recurse | Recurse |
| `Some(Pho)` | Recurse | **Skip** (atomic) | Recurse |
| `Some(Sin)` | Recurse | Recurse | **Skip** (atomic) |
| `Some(Wor)` | Recurse | Recurse | Recurse |
| `None` | Recurse | Recurse | Recurse |

### Not Suitable For

- `strip_timing_from_content()` — needs container mutation via `retain()`
- `count.rs` — Pho/Sin treat PhoGroup/SinGroup as counted atomic units (1),
  while the walker skips them entirely

### Downstream Users

| Call site | Domain | Purpose |
|-----------|--------|---------|
| `talkbank-model` `main_tier.rs` | Wor | %wor tier generation |
| `batchalign-chat-ops` `extract.rs` | Mor/Wor | NLP word extraction |
| `batchalign-chat-ops` `fa/extraction.rs` | Wor | FA word extraction |
| `batchalign-chat-ops` `fa/injection.rs` | Wor | Timing injection |
| `batchalign-chat-ops` `fa/postprocess.rs` | Wor | Timing cleanup |

## Trait Abstractions (`traits.rs`)

Four traits formalize the shared contracts across all alignment code.

### `IndexPair`

```rust
pub trait IndexPair: Clone {
    fn source(&self) -> Option<usize>;
    fn target(&self) -> Option<usize>;
    fn from_indices(source: Option<usize>, target: Option<usize>) -> Self;
    fn is_complete(&self) -> bool { ... }
    fn is_placeholder(&self) -> bool { ... }
}
```

Implemented by `AlignmentPair` (main↔dependent) and `GraAlignmentPair`
(%mor↔%gra). Enables generic code that operates on any pair type regardless
of field naming conventions.

### `TierAlignmentResult`

```rust
pub trait TierAlignmentResult: Default {
    type Pair: IndexPair;
    fn pairs(&self) -> &[Self::Pair];
    fn errors(&self) -> &[ParseError];
    fn push_pair(&mut self, pair: Self::Pair);
    fn push_error(&mut self, error: ParseError);
    fn is_error_free(&self) -> bool { ... }
}
```

Implemented by all five result types (`MorAlignment`, `PhoAlignment`,
`SinAlignment`, `WorAlignment`, `GraAlignment`). Documents the shared
accumulator contract and enables generic validation/inspection code.

### `AlignableTier`

```rust
pub trait AlignableTier {
    const DOMAIN: TierDomain;
    fn tier_name(&self) -> &str;
    fn target_count(&self) -> usize;
    fn extract_target_items(&self) -> Vec<TierPosition>;
    fn span(&self) -> Span;
    fn error_code_too_few(&self) -> ErrorCode;
    fn error_code_too_many(&self) -> ErrorCode;
    fn suggestion_too_few(&self) -> &str;
    fn suggestion_too_many(&self) -> &str;
    fn mismatch_format(&self) -> MismatchFormat { Positional }
}
```

Implemented by `PhoTier`, `SinTier`, `WorTier`. Provides everything the
generic `positional_align()` function needs to align any dependent tier
against a main tier. Adding a new tier type requires only a trait impl —
no new alignment function.

`WorTier` overrides `mismatch_format()` to `Diff` (LCS-based) since both
sides are word sequences; the other tiers use `Positional` pairing since
their target items are in different domains (phonological tokens, gestures).

### `TierCountable`

```rust
pub trait TierCountable {
    fn count_tier_positions(&self, domain: TierDomain) -> usize;
    fn collect_tier_items(&self, domain: TierDomain) -> Vec<TierPosition>;
}
```

Implemented for `[UtteranceContent]`. Provides method syntax for the free
functions in `helpers/count.rs`:

```rust
// Before: free function
let count = count_tier_positions(&main.content.content, TierDomain::Mor);

// After: trait method (TierCountable in scope)
let count = main.content.content.count_tier_positions(TierDomain::Mor);
```

### Generic `positional_align()`

```rust
pub fn positional_align<T: AlignableTier>(
    main: &MainTier,
    tier: &T,
) -> (Vec<AlignmentPair>, Vec<ParseError>)
```

Single implementation of the 1:1 positional alignment algorithm shared by
`%pho`, `%sin`, and `%wor`. The public `align_main_to_*` functions are thin
wrappers that call this and construct their domain-specific result type.

`%mor` does not use this function because it has additional terminator
validation logic. `%gra` does not use it because its source is `MorTier`
(chunks), not `MainTier`.

## Parse-Health Gating

Alignment diagnostics honor `ParseHealth` metadata on utterances. If a
dependent tier's domain is parse-tainted (the parser encountered malformed
input it could only partially recover from), alignment mismatch errors for
that domain pair are suppressed. This prevents false-positive diagnostics
from cascading parser failures.

Rules:
- Main-tier taint blocks all main→dependent alignments
- Dependent-tier taint blocks only that tier's alignment
- Unrelated dependent-dependent checks (e.g., %mor→%gra) proceed normally
  if their specific tiers are clean

## Design Principles

1. **Exhaustive matching.** Every `match` on `UtteranceContent` or
   `BracketedItem` explicitly lists all variants. Adding a new variant to
   the model without updating alignment code is a compile error (non-exhaustive
   match), not a silent bug.

2. **Domain as first-class parameter.** `TierDomain` flows through every
   counting/extraction/walking function, making policy branches explicit and
   testable rather than scattered across ad-hoc conditionals.

3. **Separation of counting and alignment.** Counting (`count.rs`) and
   positional mapping (`mor.rs`, `pho.rs`, etc.) are separate passes.
   Counting is a fast preflight; alignment builds the full `AlignmentPair`
   mapping only when needed.

4. **Walker as shared primitive.** `walk_words()` removed ~330 lines of
   duplicated traversal boilerplate across 7 files. New traversal needs
   should use the walker rather than re-implementing recursion.

5. **No string hacking.** All alignment operates on typed AST structures.
   Words are `Word` structs with `cleaned_text()` and `category` fields.
   Tiers are typed (`MorTier`, `PhoTier`, etc.). Serialized text is never
   split or regex-matched for alignment purposes.

## Phon Tier-to-Tier Alignment

The [Phon](https://www.phon.ca/phon-manual/getting_started.html) extension tiers
introduce a second class of alignment that operates **between dependent tiers**
rather than between the main tier and a dependent tier:

| Source tier | Target tier | Error code |
|-------------|-------------|------------|
| `%modsyl` | `%mod` | E725 |
| `%phosyl` | `%pho` | E726 |
| `%phoaln` | `%mod` | E727 |
| `%phoaln` | `%pho` | E728 |

These are **derived-view alignments**: `%modsyl` is a syllabified reannotation
of `%mod`, `%phosyl` of `%pho`, and `%phoaln` aligns both. Because they are
derived views of the same phonological data, word counts must always match
between source and target.

### Implementation

Phon tier alignment is computed in `alignment.rs` (`compute_alignments()`)
after the main-tier alignments. The helper `build_tier_to_tier_alignment()`
constructs index pairs and emits a `build_count_mismatch_error()` diagnostic
when counts disagree.

`%phoaln` is special: it checks word count against **both** `%mod` and `%pho`,
potentially emitting E727 and E728 simultaneously.

### Parse-Health Gating

Three new `ParseHealth` fields gate these checks:

| Gate method | Required clean tiers |
|-------------|---------------------|
| `can_align_modsyl_to_mod()` | `modsyl_clean` ∧ `mod_clean` |
| `can_align_phosyl_to_pho()` | `phosyl_clean` ∧ `pho_clean` |
| `can_align_phoaln()` | `phoaln_clean` ∧ `mod_clean` ∧ `pho_clean` |

### LSP Hover

Hover on `%modsyl` shows the aligned `%mod` word, on `%phosyl` the aligned
`%pho` word, and on `%phoaln` both the aligned `%mod` and `%pho` words plus
segment-level alignment details. The hover resolvers use text-offset-based
word index finding since Phon tiers use `text_with_bullets` grammar nodes.

### Known Data Issues

The Phon XML source data contains orthography↔IPA word count discrepancies
in approximately 4% of files (518 of 12,340 files, 6,312 records). This is
expected in child phonology data where children produce extra syllables or
partial words relative to the target. The
[PhonTalk](https://github.com/phon-ca/phontalk) XML→CHAT converter handles
this inconsistently: `%mod`/`%pho` are truncated to match orthography word
count via `OneToOne` alignment, but `%xmodsyl`/`%xphosyl`/`%xphoaln` are
written from the raw `IPATranscript`, exposing the full IPA word count. This
produces the tier-to-tier mismatches that E725–E728 flag.

## Overlap Marker Iteration

CA overlap markers (⌈⌉⌊⌋) appear at three content levels — top-level
`UtteranceContent`, inside groups as `BracketedItem`, and inside words as
`WordContent`. The alignment module provides two APIs for traversing them,
in `alignment/helpers/overlap.rs`:

### `walk_overlap_points` — Low-Level Iterator

Visits every `OverlapPoint` in document order with its word-position context.
Analogous to `walk_words` but for overlap markers instead of words:

```rust
use talkbank_model::alignment::helpers::{
    walk_overlap_points, OverlapPointVisit,
};

walk_overlap_points(&utterance.main.content.content.0, &mut |visit| {
    // visit.point: &OverlapPoint (kind + optional index)
    // visit.word_position: usize (alignable words seen so far)
});
```

### `extract_overlap_info` — Region-Based Analysis

Collects all markers, then pairs them by (kind, index) into `OverlapRegion`
structs. Each region represents a matched begin-end pair (⌈...⌉ or ⌊...⌋):

```rust
use talkbank_model::alignment::helpers::{
    extract_overlap_info, OverlapRegion, OverlapRegionKind,
};

let info = extract_overlap_info(&utterance.main.content.content.0);

for region in &info.regions {
    // region.kind: Top (⌈⌉) or Bottom (⌊⌋)
    // region.index: Option<OverlapIndex> for disambiguation (2-9)
    // region.begin_at_word / end_at_word: word positions
    // region.is_well_paired(): both markers present and ordered
}

// Proportional onset: fraction of utterance before first ⌈
if let Some(fraction) = info.top_onset_fraction() { /* 0.0-1.0 */ }
```

**Index-aware pairing:** `⌈2...⌉2` forms a separate region from `⌈...⌉`.
Mismatched indices leave markers unpaired.

**Unpaired markers:** Onset-only ⌈ (without ⌉) is a legitimate CA convention.
The region has `end_at_word = None` and `is_well_paired()` returns false,
but `top_onset_fraction()` still works.

### Cross-Utterance Overlap Groups: `analyze_file_overlaps`

For whole-file analysis, `overlap_groups.rs` provides cross-utterance matching:

```rust
use talkbank_model::alignment::helpers::{
    analyze_file_overlaps, FileOverlapAnalysis, OverlapGroup,
};

let analysis = analyze_file_overlaps(&chat_file.lines);

for group in &analysis.groups {
    // group.top: OverlapAnchor (speaker, utterance index, region, bullet)
    // group.bottoms: Vec<OverlapAnchor> — 1:N matching
    println!(
        "Speaker {} top, {} respondents",
        group.top.speaker,
        group.bottoms.len(),
    );
}
// analysis.orphaned_tops: tops with no matching bottom
// analysis.orphaned_bottoms: bottoms with no matching top
```

**1:N matching:** One top region from speaker A can match multiple bottom
regions from speakers B, C, etc. Each bottom is matched to the nearest
preceding top from a different speaker with the same index.

### Overlap Validation

The validator uses `extract_overlap_info` and `analyze_file_overlaps` for
four checks:

| Code | Level | What it checks |
|------|-------|---------------|
| E348 | Utterance | Unpaired markers within a single utterance (warning) |
| E347 | Cross-utterance | Orphaned tops/bottoms with 1:N matching (warning) |
| E373 | Utterance | Invalid overlap index values (must be 2-9) |
| E704 | Cross-utterance | Same speaker encoding both top and bottom overlap (error) |

### Debug Tool

`chatter debug overlap-audit <path>` runs `analyze_file_overlaps` on all
files and reports per-file statistics (groups, bottoms, orphans, temporal
consistency, pairing quality) in TSV format.

Use `--database <path.jsonl>` to write a persistent JSON lines database
for downstream analysis:
```bash
chatter debug overlap-audit data/ca-data/ --database overlap-db.jsonl
```
Each line is a JSON object with file path, counts, and quality classification.

## Downstream Consumers

The alignment module is used by:

| Consumer | Crate/Repo | Usage |
|----------|------------|-------|
| Validation | `talkbank-model` | Cross-tier checks (E714/E715, E725-E728), overlap checks (E347/E348/E373/E704) |
| LSP hover | `talkbank-lsp` | Show aligned tier items for word under cursor |
| Word extraction | `batchalign3` | Pull NLP-ready words from utterances |
| FA injection | `batchalign3` | Insert timing bullets into AST |
| Overlap windowing | `batchalign3` | CA marker-aware UTR pass-2 search windows |
| Overlap audit | `talkbank-cli` | `chatter debug overlap-audit` — per-file overlap statistics |
| %wor generation | `talkbank-model` | Build %wor tier from main tier |
| CLAN commands | `talkbank-clan` | DSS, EVAL, KIDEVAL use typed %mor access |

---
Last Updated: 2026-03-18
