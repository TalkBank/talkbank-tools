# Not-Implemented Error Specifications: Inventory and Prioritization

**Status:** Current
**Last updated:** 2026-03-17

## Summary

There are 94 error spec files in `spec/errors/` marked with `Status: not_implemented`. These represent error conditions that have been identified and specified but do not yet have working detection in the toolchain.

### Counts by Layer

| Layer      | Count |
|------------|-------|
| parser     | 51    |
| validation | 43    |

### Counts by Level

| Level     | Count |
|-----------|-------|
| file      | 10    |
| header    | 2     |
| utterance | 38    |
| word      | 14    |
| tier      | 20    |

### Counts by Type

| Type          | Count |
|---------------|-------|
| Error (Exxx)  | 83    |
| Warning (Wxxx)| 5     |
| Non-standard  | 6     |

The 6 non-standard entries (Alignment, Complex, Events, Multiple, Omitted, Pauses, Tag, Terminator, NONE) are auto-generated corpus-derived specs that do not follow the standard E/W numbering convention.

---

## Parser Layer (51 specs)

These errors require grammar or parser-level changes to detect. Most are cases where the parser currently reports a generic `E316` (UnparsableContent) rather than a specific error code.

### Parser -- File Level (10)

| Code | Description | Category |
|------|-------------|----------|
| Alignment | Auto-generated from corpus | Alignment count mismatch |
| Complex | Auto-generated from corpus | Alignment count mismatch |
| E001 | InternalError | internal |
| E002 | TestError | internal |
| E003 | Auto-generated from corpus | validation |
| E101 | Auto-generated from corpus | validation |
| Events | Auto-generated from corpus | Alignment count mismatch |
| Multiple | Auto-generated from corpus | Alignment count mismatch |
| NONE | Media filename mismatch | validation |
| Omitted | 0word | Alignment count mismatch |
| Pauses | Auto-generated from corpus | Alignment count mismatch |
| Tag | Auto-generated from corpus | Alignment count mismatch |
| Terminator | Auto-generated from corpus | Alignment count mismatch |

### Parser -- Word Level (3)

| Code | Description | Category |
|------|-------------|----------|
| E208 | Empty replacement | validation |
| E209 | Empty spoken content | validation |
| E251 | Empty word content text | validation |

### Parser -- Utterance Level (24)

| Code | Description | Category |
|------|-------------|----------|
| E302 | Missing required node | validation |
| E303 | Unexpected node - helper function | Parser bugs (experimental) |
| E309 | Unexpected syntax | validation |
| E317 | UnparsableFileContent | parser_recovery |
| E318 | UnparsableDependentTier | parser_recovery |
| E319 | UnparsableLine | parser_recovery |
| E320 | UnparsableHeader | parser_recovery |
| E321 | UnparsableUtterance | parser_recovery |
| E322 | EmptyColon | parser_recovery |
| E323 | Auto-generated from corpus | validation |
| E325 | UnexpectedUtteranceChild | parser_recovery |
| E331 | UnexpectedNodeInContext | parser_recovery |
| E340 | UnknownBaseContent | parser_recovery |
| E344 | Invalid scoped annotation nesting | validation |
| E345 | Unmatched scoped annotation begin | validation |
| E346 | Unmatched scoped annotation end | validation |
| E350 | Replacement text empty | Word validation |
| E369 | Nonvocal label mismatch | validation |
| E374 | ErrorAnnotationParseError | parser_recovery |
| E376 | Replacement parse error | Word validation |
| E377 | RetraceParseError | parser_recovery |
| E378 | OverlapAnnotationParseError | parser_recovery |
| E380 | UnknownSeparator | parser_recovery |
| E383 | Auto-generated from corpus | Dependent tier parsing |

### Parser -- Tier Level (7)

| Code | Description | Category |
|------|-------------|----------|
| E381 | PhoParseError | tier_parse |
| E384 | SinParseError | tier_parse |
| E385 | WordParseError | tier_parse |
| E386 | TextTierParseError | tier_parse |
| E702 | Invalid MOR chunk format - missing \| | Dependent tier parsing |
| E703 | Unexpected morphology node | validation |
| E720 | Mor-Gra count mismatch | Alignment count mismatch |
| E721 | GRA non-sequential index | validation |
| E722 | GRA has no ROOT | validation |
| E723 | GRA has multiple ROOTs | validation |
| W724 | Auto-generated from corpus | validation |

---

## Validation Layer (43 specs)

These are checks that could be implemented without parser or grammar changes. The parser already produces the relevant AST nodes; a validation pass just needs to inspect them.

### Validation -- Word Level (11)

| Code | Description | Category |
|------|-------------|----------|
| E211 | OmissionInReplacement | deprecated |
| E212 | Auto-generated from corpus | Parser error |
| E214 | Auto-generated from corpus | validation |
| E220 | Auto-generated from corpus | Word validation |
| E242 | Auto-generated from corpus | validation |
| E243 | Auto-generated from corpus | validation |
| E245 | Auto-generated from corpus | validation |
| E246 | Auto-generated from corpus | validation |
| E248 | Auto-generated from corpus | validation |
| E253 | Auto-generated from corpus | validation |
| E258 | Consecutive commas | validation |
| W210 | Auto-generated from corpus | validation |
| W211 | Auto-generated from corpus | validation |

### Validation -- Header Level (2)

| Code | Description | Category |
|------|-------------|----------|
| E514 | Missing language code in @ID | validation |
| E531 | Auto-generated from corpus | validation |

### Validation -- Utterance Level (17)

| Code | Description | Category |
|------|-------------|----------|
| E310 | Auto-generated from corpus | Main tier validation |
| E341 | UnbalancedQuotationCrossUtterance | cross_utterance |
| E347 | Unbalanced overlap (cross-utterance) | validation |
| E348 | Missing overlap end (intra-utterance) | validation |
| E351 | MissingQuoteBegin | cross_utterance |
| E352 | MissingQuoteEnd | cross_utterance |
| E353 | MissingOtherCompletionContext | cross_utterance |
| E354 | MissingTrailingOffTerminator | cross_utterance |
| E355 | InterleavedScopedAnnotations | cross_utterance |
| E356 | UnmatchedUnderlineBegin | underline_balance |
| E357 | UnmatchedUnderlineEnd | underline_balance |
| E365 | Auto-generated from corpus | validation |
| E366 | Long feature label mismatch | validation |
| E373 | InvalidOverlapIndex | overlap |
| E388 | Auto-generated from corpus | validation |

### Validation -- Tier Level (13)

| Code | Description | Category |
|------|-------------|----------|
| E600 | Tier alignment skipped due to parse errors | validation |
| E700 | Unexpected tier node | validation |
| E701 | Auto-generated from corpus | Dependent tier parsing |
| E704 | Speaker self-overlap -- overlapping overlap markers | validation |
| E707 | Mor terminator presence mismatch | Alignment terminator mismatch |
| E708 | GRA relation missing index | Dependent tier parsing |
| E709 | Invalid grammar index | validation |
| E711 | Mor empty content | Mor content validation |
| E716 | Mor terminator value mismatch | Alignment terminator mismatch |
| E717 | Pho terminator presence mismatch | Alignment terminator mismatch |
| E724 | GRA has circular dependency | validation |
| W602 | Auto-generated from corpus | Warnings |
| W603 | Auto-generated from corpus | Warnings |

---

## Prioritization Notes

### Parser Layer (51 specs) -- Lower Priority

The majority of parser-layer specs are E316 catch-all cases: the parser currently reports a generic `UnparsableContent` error rather than a specific error code. Implementing these individually would require grammar changes (new tree-sitter rules or error recovery nodes) or parser-side logic to distinguish between different kinds of parse failures. This is lower priority until parser error specificity becomes an explicit goal.

Notable subgroups within the parser layer:

- **Parser recovery codes** (E317-E325, E331, E340, E374, E377, E378, E380): These represent different flavors of unparsable content. The parser currently lumps most of these into E316. Splitting them out would improve error messages but requires significant parser work.
- **Tier parse errors** (E381, E384-E386): Errors in parsing %pho, %sin, %wor, and text tiers. These use the direct parser (chumsky) and may already produce partial diagnostics.
- **GRA structural validation at parser layer** (E720-E723, W724): These check %gra tier structure (count mismatches, index sequencing, ROOT presence). Despite being classified as parser-layer, some of these could potentially be reclassified as validation-layer checks.

### Validation Layer (43 specs) -- Higher Priority

Validation-layer specs represent checks that can be implemented without parser changes -- the AST already contains the necessary information, and a validation pass just needs to inspect it. These are the most cost-effective to implement.

Key subgroups:

- **Cross-utterance validation** (E341, E347, E351-E355): Checks that span multiple utterances (quotation balance, overlap pairing, trailing-off context). These require the validator to maintain state across utterances, which is already supported by the cross-utterance validation infrastructure.
- **Overlap validation** (E347, E348, E373, E704): Overlap-related checks. **E347 is now partially implemented** (cross-utterance overlap balancing was added as part of the two-pass UTR overlap work). **E348 is fully implemented** (intra-utterance overlap pairing warnings). E373 (invalid overlap index) and E704 (speaker self-overlap) remain unimplemented.
- **Word-level validation** (E211-E258, W210, W211): Various word content checks. Many are auto-generated from corpus and need spec refinement before implementation. E211 (OmissionInReplacement) is marked deprecated.
- **Underline balance** (E356, E357): Checks for unmatched underline begin/end markers. Similar in structure to overlap balance checks.

### Tier-Level Codes (20 specs) -- Important for Data Quality

Twenty specs target tier-level validation, split between parser (7) and validation (13) layers. Many involve %mor/%gra alignment and structural validation:

- **Alignment terminator mismatches** (E707, E716, E717): Check that %mor and %pho tier terminators match the main tier. These are important for data integrity in corpora that include morphological and phonological annotations.
- **GRA structural checks** (E708, E709, E720-E724, W724): Index validity, ROOT presence/uniqueness, circular dependencies, count mismatches. Critical for any downstream tool that consumes %gra tiers.
- **Mor content validation** (E711): Empty morphology content. Straightforward to implement.

### Temporal / E700-Range Codes (11 specs)

The E700-E724 range contains tier validation codes. Several may be blocked by missing infrastructure:

- E700 (unexpected tier node) and E701 are general tier validation.
- E702-E703 are MOR parsing structural checks.
- E707, E716, E717 are terminator alignment checks that depend on the alignment infrastructure being able to compare main tier and dependent tier terminators.
- E720-E724 are GRA-specific structural validations.

### Recommended Implementation Order

1. **E348** -- Already fully implemented; update spec status.
2. **E347** -- Partially implemented (cross-utterance); update spec to reflect current coverage.
3. **Validation-layer word checks** (E242, E243, E245, E246, E248, E253, E258) -- Straightforward AST inspection.
4. **Cross-utterance checks** (E341, E351-E355) -- Infrastructure exists; similar to overlap balancing.
5. **Underline balance** (E356, E357) -- Pattern matches existing overlap balance code.
6. **Tier alignment terminators** (E707, E716, E717) -- Important for data quality.
7. **GRA structural** (E708, E709, E720-E724) -- Important for %gra consumers.
8. **Header validation** (E514, E531) -- Low-hanging fruit.
9. **Parser recovery specificity** (E317-E325) -- Only when error message quality becomes a priority.
