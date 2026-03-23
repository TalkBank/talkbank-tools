# Grammar Cleanup Audit — Progress Log

**Status:** Complete
**Last updated:** 2026-03-23 12:00 EDT

This document tracks the grammar.js cleanup audit. Franklin requested:
1. Remove legacy conflicts/precedence no longer needed
2. Collapse opaque tokens into structured rules where possible
3. Structure @ID header fields explicitly
4. General optimization and simplification

## Audit Results

### 1. Conflicts Array — ALL NEEDED (no cleanup)
5 entries, all justified by genuine CHAT ambiguities:
- `contents` — word/annotation/group interleaving on main tiers
- `word_with_optional_annotations` — `[...]` attachment ambiguity
- `nonword_with_optional_annotations` — same for events/zeros
- `base_annotations` — multiple annotation type disambiguation
- `final_codes` — media URLs and postcodes at line end

### 2. Precedence Values — CLEAN (no changes needed)
8 unique prec levels, 81 usages, consistent hierarchy:
- prec(10): terminators, linkers, CA markers, annotations (39 rules)
- prec(8): bracket annotations (9 rules)
- prec(6): standalone_word zero disambiguation (2 rules)
- prec(5): word_segment, lengthening, overlap_point (9 rules)
- prec(3): zero token, full_document in source_file (2 rules)
- prec(2): utterance in source_file (1 rule)
- prec(1): event_segment, nonword, fragment types (17 rules)
- prec(0): standalone_word in source_file (2 rules)
- prec(-1): separator (1 rule, confirmed intentional — deprioritizes)

### 3. Opaque Tokens → Structured Rules — KEEP AS-IS
6 bracket annotation tokens (explanation, para, alt, error_marker,
percent, duration) are opaque. The Rust re-parsing is trivial (text
extraction + validation). tree-sitter lexer is faster than grammar-level
structure. **No benefit from restructuring.**

### 4. @ID Header Fields — ALREADY STRUCTURED
Contrary to initial assumption, @ID is NOT opaque. It's decomposed into:
- `id_contents` with subrules: `_id_identity_fields`, `_id_demographic_fields`, `_id_role_fields`
- Individual fields: `id_corpus`, `id_speaker`, `id_age`, `id_sex`, `id_group`, `id_ses`, `id_role`, `id_education`, `id_custom_field`
- Each has appropriate regex or strict+catch-all pattern. **No cleanup needed.**

### 5. Dead Rules — ZERO FOUND
All 380 defined rules are referenced at least once. Clean.

### 6. Duplicate Patterns — MINOR
3 regexes appear 2-5 times (ID field trimming, pipe-delimited fields,
generic catch-alls). Could extract to constants but low priority.

## Changes Made

### Added: Comment on prec(-1) separator
Clarified why negative precedence is used on the separator rule.

### Added: Named constant for ID field trimming regex
Extracted `/[^ \t\|\r\n]([^\|\r\n]*[^ \t\|\r\n])?/` as `TRIMMED_PIPE_FIELD`
for the 5 ID field rules that use it.

## Conclusion

The grammar is in good shape. The major cleanup work (Chumsky elimination,
structured word grammar, multi-root union, zero disambiguation) was done
in the preceding commits. No structural changes needed — only documentation
and minor constant extraction.

**Next steps for grammar improvement (future work):**
1. Add reference corpus files exercising `pos_tag`, `stress_marker` (currently in coverage exclusion list)
2. Consider structuring `age_format` token into year/month/day children
3. Review whether annotation tokens could benefit from structured children for LSP hover/rename support
