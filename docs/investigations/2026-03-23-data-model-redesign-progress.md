# Data Model Redesign Progress

**Status:** In Progress
**Last updated:** 2026-03-23

## Phase 1: Retrace as First-Class Variant — COMPLETE ✓

- `Retrace` struct and `RetraceKind` enum added to talkbank-model
- `UtteranceContent::Retrace` and `BracketedItem::Retrace` variants added
- Parser emits `Retrace` for all retrace cases (groups and single words)
- `ContentAnnotation::is_retrace()` public API added
- `Retrace.is_group` flag for lossless roundtrip (angle brackets)
- All 39+ match sites across workspace updated
- All tests pass, make verify green
- Validation regression: 51 → 1 (E246 is grammar-enforced)

## Phase 2: Annotation Rename — PARTIALLY COMPLETE

### Done:
- `ScopedAnnotation` → `ContentAnnotation` (278 references renamed)
- `AnnotatedScopedAnnotations` → `AnnotatedAnnotations`
- Variant renames: `CaContinuationMarker` → `CaContinuation`, `ScopedStressing` → `Stressing`, etc.
- All tests pass, make verify green

### NOT Done (critical):
- **Remove 5 retrace variants from ContentAnnotation**

The retrace variants (`PartialRetracing`, `Retracing`, `MultipleRetracing`, `Reformulation`, `UncertainRetracing`) still exist in `ContentAnnotation` because:

1. The annotation parser (`single.rs`) creates them during CST parsing
2. The content handlers (`word.rs`, `group/parser.rs`) then convert them to `Retrace`
3. CLAN commands (`flo.rs`, `repeat.rs`, `flucalc.rs`) match on them directly
4. Alignment rules and validation retrace detection match on them

**To remove retrace variants, need to:**

1. **Restructure annotation parser** to return `ParsedAnnotation` union type:
   ```rust
   enum ParsedAnnotation {
       Content(ContentAnnotation),
       Retrace(RetraceKind),
   }
   ```
   `parse_scoped_annotations()` returns `Vec<ParsedAnnotation>`. Content handlers split the vec into content annotations + optional retrace.

2. **Update CLAN commands** that match on retrace annotations directly. These need to match on `UtteranceContent::Retrace` instead:
   - `flucalc.rs`: `PartialRetracing → phrase_reps`, `Retracing → revisions`
   - `flo.rs`: retrace detection for FLO transform
   - `repeat.rs`: retrace detection
   - `word_filter.rs`: retrace filtering

3. **Simplify alignment/validation** retrace detection — no longer check annotations, just match `UtteranceContent::Retrace`

4. **Remove `is_retrace()` from ContentAnnotation** — it returns false for all remaining variants

## Phase 3: Structural Marker Collapse — NOT STARTED

Collapse 10 structural UtteranceContent/BracketedItem variants into `Marker(Marker)`.

## Phase 4: Grammar Lint Fixes — NOT STARTED

From `~/tree-sitter-grammar-utils/docs/chat-grammar-lint-report.md`.

## Commits (not yet pushed)

1. `5148362` — Retrace type + all match sites (Phase 1a)
2. `3838847` — Parser emits Retrace (Phase 1b)
3. `fd0d290` — Mark 4 remaining error specs not_implemented
4. `61df72e` — Rename ScopedAnnotation → ContentAnnotation (Phase 2 partial)
