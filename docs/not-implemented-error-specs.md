# Not-Implemented Error Specifications: Reconciled Inventory

**Status:** Current
**Last updated:** 2026-03-18

## Reconciliation Results

A reconciliation script (`spec/errors/` vs actual code) found that **38 of
the original 94 "not_implemented" specs were actually implemented** — the
validation or parser code existed but the spec status was never updated.

These 38 specs have now been updated to `Status: implemented`.

### Current Counts

| Category | Count | Notes |
|----------|-------|-------|
| **Actually implemented** (just fixed) | 38 | Spec status updated |
| **Genuinely not implemented** | 43 | Enum variant exists, no checking logic |
| **Non-standard specs** | 11 | Auto-generated, non-standard naming |
| **Previously fixed** (E347, E348) | 2 | Fixed during overlap work |
| **Total remaining not-implemented** | 54 | 43 genuine + 11 non-standard |

## Genuinely Not Implemented (43 codes)

These have enum variants in `error_code.rs` but no validation or parser
logic that checks for and reports them.

### Internal / Test (2) — no action needed
| Code | Variant | Notes |
|------|---------|-------|
| E001 | InternalError | Internal sentinel, never reported to users |
| E002 | TestError | Test-only sentinel |

### Parser Layer — Generic Catch-Alls (7)
These are categories where the parser currently reports E316 instead:
| Code | Variant | Notes |
|------|---------|-------|
| E101 | InvalidLineFormat | Line doesn't match any known pattern |
| E303 | SyntaxError | Generic syntax error |
| E317 | UnparsableFileContent | File-level parse failure |
| E318 | UnparsableDependentTier | Dependent tier parse failure |
| E345 | UnmatchedScopedAnnotationBegin | Orphaned `[` opener |
| E350 | GenericAnnotationError | Catch-all for annotation issues |
| E374 | ErrorAnnotationParseError | `[*]` parse failure |

### Parser Layer — Tier Parse Errors (6)
| Code | Variant | Notes |
|------|---------|-------|
| E377 | RetraceParseError | Retrace annotation parse failure |
| E378 | OverlapAnnotationParseError | Overlap annotation parse |
| E380 | UnknownSeparator | Unrecognized separator character |
| E381 | PhoParseError | %pho tier parse failure |
| E384 | SinParseError | %sin tier parse failure |
| E386 | TextTierParseError | Text tier parse failure |

### Validation — Word Level (6)
| Code | Variant | Notes |
|------|---------|-------|
| E209 | EmptySpokenContent | Word has no spoken content |
| E211 | OmissionInReplacement | Deprecated |
| E214 | EmptyAnnotatedScopedAnnotations | Annotated word with empty annotations |
| E251 | EmptyWordContentText | Word text segment is empty |
| E253 | EmptyWordContent | Word has no content items |
| E258 | ConsecutiveCommas | Two commas in a row |

### Validation — Tier Level (13)
| Code | Variant | Notes |
|------|---------|-------|
| E383 | GraParseError | %gra parse failure |
| E385 | WordParseError | Word in tier context |
| E388 | ReplacementOnNonword | `[:` on nonword |
| E700 | UnexpectedTierNode | Unknown tier node type |
| E701 | TierBeginTimeNotMonotonic | Timeline violations |
| E703 | UnexpectedMorphologyNode | Bad %mor node |
| E707 | MorTerminatorPresenceMismatch | %mor has/lacks terminator vs main |
| E711 | MorEmptyContent | %mor item has no content |
| E716 | MorTerminatorValueMismatch | %mor terminator differs from main |
| E717 | PhoTerminatorPresenceMismatch | %pho terminator mismatch |
| E720 | MorGraCountMismatch | %mor and %gra item counts differ |
| E721 | GraNonSequentialIndex | %gra indices not sequential |
| E722 | GraNoRoot | %gra has no ROOT relation |
| E723 | GraMultipleRoots | %gra has >1 ROOT |

### Validation — Header Level (2)
| Code | Variant | Notes |
|------|---------|-------|
| E514 | MissingLanguageCode | @ID line missing language |
| E531 | MediaFilenameMismatch | @Media filename doesn't match |

### Validation — Cross-Utterance (2)
| Code | Variant | Notes |
|------|---------|-------|
| E366 | LongFeatureLabelMismatch | `&{l=X` / `&}l=Y` labels differ |
| E369 | NonvocalLabelMismatch | `&{n=X` / `&}n=Y` labels differ |

### Warnings (3)
| Code | Variant | Notes |
|------|---------|-------|
| W210 | MissingWhitespaceBeforeContent | Formatting warning |
| W211 | MissingWhitespaceAfterOverlap | Formatting warning |
| W724 | GraRootHeadNotSelf | %gra ROOT head index != self |

## Non-Standard Specs (11)

Auto-generated from corpus mining, don't follow E/W numbering:
`Alignment`, `Complex`, `E724_gra_circular_dependency`, `Events`,
`Multiple`, `NONE`, `Omitted`, `Pauses`, `Tag`, `Terminator`, `W603`

These need manual review to determine if they map to existing codes
or should be deleted.

## Prioritization

### Quick wins (validation layer, low effort)
1. **E258** (ConsecutiveCommas) — simple content check
2. **E514** (MissingLanguageCode) — header field check
3. **E707/E716/E717** (terminator mismatches) — tier alignment checks
4. **E722/E723** (GRA root checks) — structural validation
5. **W210/W211** (whitespace warnings) — formatting checks

### Medium effort
6. **E720/E721** (MOR/GRA count/index) — alignment count checks
7. **E366/E369** (label mismatches) — cross-utterance scoped markers
8. **E531** (media filename mismatch) — requires media resolution

### Parser layer (higher effort)
9. **E317-E318** (unparsable content specificity) — parser error recovery
10. **E381/E384/E386** (tier parse errors) — direct parser diagnostics
