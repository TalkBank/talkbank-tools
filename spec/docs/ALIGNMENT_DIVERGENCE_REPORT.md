# Alignment Divergence Report: talkbank-tools vs Python batchalign

**Date:** 2026-02-12
**Status:** RESOLVED

---

## Executive Summary

Five behavioral divergences existed between our Rust validator/generator and
Python batchalign's actual %wor generation behavior. All five have been fixed
to match Python batchalign, which is authoritative since it produces the %wor
tiers that we parse and validate.

---

## Divergences Found and Resolved

### Root Cause

Python batchalign's lexer has two behaviors that our Rust code was not matching:

1. **`TokenType.ANNOT` filtering:** The lexer filters out tokens tagged as
   `ANNOT`, which includes nonwords (`&~`), fragments (`&+`), and untranscribed
   (`xxx`/`yyy`/`www`). Fillers (`&-`) are NOT filtered — they appear in %wor.

2. **Replacement substitution:** The lexer completely replaces the original word
   with the replacement text. So `want [: wanted]` becomes just `wanted` in %wor,
   and `&+fr [: friend]` becomes `friend` (the fragment prefix is gone after
   substitution, so it passes the filter).

### Five Divergences

| # | Content | Python batchalign | Our Rust (BEFORE fix) | Fix Applied |
|---|---------|------------------|----------------------|-------------|
| 1 | `&~gaga` nonwords | EXCLUDED from %wor | INCLUDED | Changed `word_is_alignable()` to exclude nonwords for Wor |
| 2 | `&+fr` fragments | EXCLUDED from %wor | INCLUDED | Changed `word_is_alignable()` to exclude fragments for Wor |
| 3 | `xxx/yyy/www` | EXCLUDED from %wor | INCLUDED | Changed `word_is_alignable()` to exclude untranscribed for Wor |
| 4 | `want [: wanted]` | Uses REPLACEMENT `wanted` | Used ORIGINAL `want` | Moved Wor to Mor branch for replacement handling |
| 5 | `&+fr [: friend]` | Uses REPLACEMENT `friend` | EXCLUDED entirely | Consequence of fixes #2 and #4 together |

### Files Changed

| File | Change |
|------|--------|
| `alignment/helpers/rules.rs` | Added `is_wor_excluded_word()`, updated Wor branch in `word_is_alignable()` |
| `alignment/helpers/count.rs` | Moved `Wor` from `Pho\|Sin` to `Mor` branch in both `count_alignable_replaced_word()` and `extract_alignable_from_replaced_word()` |
| `alignment/helpers/tests.rs` | Updated test expectations, added `wor_excludes_nonwords_and_fragments_but_includes_fillers` test |
| `model/file/utterance/metadata/alignment.rs` | Updated `apply_word_timing_replaced()` to distribute timing to replacement words |
| `model/content/main_tier.rs` | Updated `collect_wor_words_content/bracketed()` to use replacement words via new `collect_wor_replaced_word()` |
| `batchalign-core/src/forced_alignment.rs` | Updated `collect_fa_replaced_word()` and `inject_timing_on_replaced_word()` to use replacement words |

---

## Prior Incorrect Fix (Context)

An earlier fix incorrectly moved Wor to the Pho/Sin branch for replacement
handling, based on the batchalign2 spec document (`wor-tier-spec.md`) which
said "The ORIGINAL word appears in %wor." However, the actual Python batchalign
implementation uses replacement words (the lexer completely substitutes them).
The spec document has since been corrected.

---

## Remaining Minor Observations

### 1. Error Code Sharing (E714/E715)

%wor alignment reuses error codes E714 (`PhoCountMismatchTooFew`) and E715
(`PhoCountMismatchTooMany`) from %pho. The error messages include "%wor tier"
in the text, so this is functional. Consider dedicated codes if needed.

### 2. Terminator Validation

%mor validates terminator consistency (E707). %wor does not have an equivalent
check. Consider adding if needed.

---

Last Updated: 2026-02-12
