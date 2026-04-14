# Spec Implementation Status Audit

**Status:** Reference
**Last updated:** 2026-04-13 21:38 EDT

Audit of all `spec/errors/*.md` files currently marked
`- **Status**: not_implemented`. Each spec is classified against one of
four patterns to guide follow-up work.

## Method

For each spec:

1. Extract the `ErrorCode` variant from `crates/talkbank-model/src/errors/codes/error_code.rs`.
2. Grep all non-test, non-generated Rust sources under `crates/` for
   `ErrorCode::<Variant>` to find emission sites.
3. Extract the spec's first `chat` example and run it through the
   release build of `chatter validate --force --format json` to see what
   error codes actually fire.
4. Cross-reference emission sites, observed codes, and the spec's
   declared "Expected Error Codes" to classify.

"Emission site" counts exclude:

- `crates/talkbank-model/src/errors/codes/error_code.rs` (enum definition)
- `crates/talkbank-clan/src/commands/check/error_map.rs`
  (CLAN-CHECK-number ↔ `ErrorCode` bidirectional map; does not emit)
- `crates/talkbank-cli/src/commands/list_checks.rs` (help text only)
- `crates/talkbank-derive/src/lib.rs` (doc examples)
- `*/tests/**`, `*_test*.rs`, `*/generated/*`, `property_tests_modules/*`,
  `audit_check_parity.rs`, `error_coverage.rs`
- `crates/talkbank-lsp/src/alignment/tests.rs` (test fixtures)

## Patterns

- **A** — Code is emitted by production code; the auto-generated example
  doesn't trigger it. Fix: replace the example with input that actually
  triggers the code (or clarify that a specific narrow construct is needed).
- **B** — Code is emitted somewhere, but tree-sitter's pre-validation or
  error-recovery pre-empts it for this input. Reachable only from the
  re2c parser or from a rare structural edge case. Fix: keep
  `not_implemented`; add a "Status note" explaining the preemption and
  which code fires instead.
- **C** — Variant exists in `ErrorCode` but nothing in production code
  ever constructs it. Candidates for removal as dead code or for future
  implementation.
- **D** — Will be emitted once a feature flag is turned on. Specifically,
  `ValidationContext.enable_quotation_validation` (default `false`)
  gates the cross-utterance validators in
  `crates/talkbank-model/src/validation/cross_utterance/`.

## Summary

The audit covers **48 spec files** (47 unique error codes; `E360` has
two specs — the auto spec and a deprecated-skip-bullet variant). The
task brief said 49; the strict `Status: not_implemented` count is 48.

| Pattern | Count | Meaning |
|---------|------:|---------|
| A: code emits, example wrong | 21 | Fix the spec example |
| B: tree-sitter pre-empts | 16 | Add Status note; keep `not_implemented` |
| C: dead variant           | 9  | Remove, or implement |
| D: flag-gated             | 8  | Turn on once `enable_quotation_validation` is stable |

(Counts may overlap by one: `E360` and `E360_deprecated_skip_bullet`
both cover the same variant `InvalidMediaBullet`; both fall under
Pattern B.)

## Detailed Classification

### Pattern A — Example is wrong (21 codes)

These variants are actively emitted in production; the auto-generated
example simply doesn't exercise that code path. Each row lists what the
spec expects vs. what `chatter validate` actually produces on the
example.

| Code | Variant | Spec expects | Example produces | Notes |
|------|---------|--------------|------------------|-------|
| E003 | EmptyString | E316, E502-E504 | E316, E502-E504 | Spec example is already annotated "E003 is not reachable from an empty file"; keep as-is or update spec text to say "file-level unreachable; fires only for empty `NonEmptyString` fields". |
| E208 | EmptyReplacement | E376 | E376 | Spec already explains E376 fires instead. Narrow case (parser building a `Replacement` with zero words) is only reachable via internal constructors. Consider reclassifying as Pattern C for the end-to-end pipeline. |
| E212 | InvalidWordFormat | E212 | (none) | Example `hello world .` is valid. Emits for CA-omission outside CA mode, standalone shortenings, etc. Needs a new example. |
| E214 | EmptyAnnotatedContentAnnotations | E214 | (none) | Example `hello [*] .` parses `[*]` as a valid single annotation. Need an example where annotated content ends with zero annotations. |
| E246 | LengtheningNotAfterSpokenMaterial | E246 | (none) | Example `:hello` is likely absorbed into the word token. Code in `validation/word/structure.rs` fires for `Lengthening` preceded by no spoken material. |
| E251 | EmptyWordContentText | E251 | (none) | Example `@s:eng .` parses as a valid language-tagged form. Emits for empty `Text` / `ShorteningText` inner text. |
| E302 | MissingNode | E501-E505 | E501-E505 | Example lacks a speaker line, so header errors preempt. Code fires from speaker parsing and main-tier structure errors. Need an example with valid headers but a missing tree-sitter node. |
| E309 | UnexpectedSyntax | E501-E505 | E501-E505 | Example triggers header-missing errors. `tree_parsing/helpers.rs` emits this; re2c also emits. |
| E325 | UnexpectedUtteranceChild | — | E324, E600 | Fires from `utterance_parser.rs` for unexpected CST children. Example emits E324 instead. |
| E331 | UnexpectedNodeInContext | — | E316, E722, E724 | Emitted from `tree_parsing/helpers.rs`. Needs a narrower example. |
| E342 | MissingRequiredElement | E390 | E316, E390, E501, E702 | Emitted from `tier_parsers/mor/word.rs`, `cst_assertions.rs`, `error_checking.rs`. Example triggers broader set. |
| E364 | MalformedWordContent | E246, E249 | E246, E249 | Spec explicitly notes "requires tree-sitter to insert a MISSING node where a word is expected". Production emission sites exist in long_feature/nonvocal/word. Example fails to trigger; difficult but not impossible. |
| E370 | StructuralOrderError | E316, E600 | E316, E501, E600 | Widely emitted in main-tier structure code. Example does not cleanly isolate the structural-ordering path. |
| E404 | OrphanedDependentTier | E501-E505 | E501-E505 | Emitted in `chat_file/helpers.rs`. Example's header errors dominate. Need an example with a dependent tier before any main tier. |
| E531 | MediaFilenameMismatch | E531 | (none) | Emitted in `model/file/chat_file/validate.rs`. The example's `@Media: different, audio` may not be parsed against a filename in the validation path used here. Confirm filename-vs-media check is wired through `chatter validate`. |
| E702 | InvalidMorphologyFormat | — | E501-E505 | Emitted in `tree_parsing/parser_helpers/error_analysis/dependent_tier.rs`. Example lacks required headers, so those errors preempt. |
| E709 | InvalidGrammarIndex | E600 | E600 | Spec text already documents: production emits E709 only for numeric index `0` (1-based indexing); example `abc\|0\|ROOT` fails at grammar level with E600. Needs example `%gra:\t0\|0\|ROOT`. |
| E711 | MorEmptyContent | — | E316, E501, E702 | Code emits from `dependent_tier/mor/tier.rs` (3 sites). Grammar rejects `v\|` before model validation runs. The existing Status note documents this already; reclassify as Pattern B. |
| W602 | UnknownUserDefinedTier | W602 | (none) | Code emits from `validation/unparsed_tier.rs`. Spec text says `%xpho` is parsed as recognized tier, so it never routes through the user-defined check. Need an example `%xothertiername`. |
| E322 | EmptyColon | — | E316 | Emitted from `main_tier/structure/convert/prefix.rs`. Example's malformed `*CHI :` gets tree-sitter-repaired into generic E316. |
| E319 | UnparsableLine | — | E602 | Emitted by `error_analysis/line.rs` (tree-sitter) and by re2c. Example triggers tier-level E602 because the non-line is a malformed tier header. |

### Pattern B — Tree-sitter pre-empts (16 codes)

Production code emits these, but tree-sitter's strict grammar or
error-recovery routes the spec's example to a different code (usually
E316 or a sibling). Keep `Status: not_implemented` with a "Status note"
explaining what actually fires.

| Code | Variant | Preempted by | Notes |
|------|---------|--------------|-------|
| E101 | InvalidLineFormat | E501-E504 | Only emitted by CLAN CHECK mapping; confirm: grep shows zero production emit sites. **May actually be Pattern C.** Kept here because `error_map.rs` lists it as a known CHECK error — intentional but currently never emitted from our parsers. |
| E303 | SyntaxError | E501-E505 | Emitted in `error_analysis/file.rs` and re2c. Example preempted by header-level errors. |
| E310 | ParseFailed | E310 | Example produces no errors. Emitted only on catastrophic parse failure (empty/broken CST). Tree-sitter error recovery avoids this path for any parseable input. |
| E311 | UnexpectedNode | E316 | Emitted in `chat_file_parser/single_item/helpers.rs` and utterance_parser. Preempted by E316 for malformed utterance content. |
| E312 | UnclosedBracket | E304, E375 | Status note already present. Tree-sitter's recovery routes unclosed brackets through ERROR nodes that emit E375. |
| E319 | UnparsableLine | E602 | Status note already present. |
| E320 | UnparsableHeader | E539 | Status note already present: malformed header values route to header-value validators (E539 for `@Transcription`, etc.). |
| E321 | UnparsableUtterance | E304, E375 | Status note already present. |
| E322 | EmptyColon | E316 | Status note already present. |
| E323 | MissingColonAfterSpeaker | E316, E502-E505 | Status note already present. |
| E344 | InvalidContentAnnotationNesting | E316 | Code lives in `cross_utterance/quotation_precedes.rs` — but that's actually **flag-gated** too. This is Pattern D, not B. (See Pattern D below; including here for visibility.) |
| E360 | InvalidMediaBullet | E316 | Status note already present. Production emits from 7 sites. Example (tab-delimited malformed timestamp) fails at grammar level, producing E316. |
| E360_deprecated | InvalidMediaBullet | E316 | Same as E360. Deprecated skip-bullet dash variant; grammar rejects the trailing `-` before Rust validation runs. |
| E365 | MalformedTierContent | (none emitted) | Emits only from `header_dispatch/parse.rs`. Narrow trigger. Example produces no error. |
| E708 | MalformedGrammarRelation | E316 (or silent) | Status note already present. Grammar's strict `gra_index` regex prevents malformed relations from reaching `gra/relation.rs`. |
| E711 | MorEmptyContent | E316, E702 | Status note already present. |

### Pattern C — Truly dead (9 codes)

Variant exists in `ErrorCode` but is never emitted from any
production code path. Either delete the variant or implement a producer.

| Code | Variant | Last used where | Recommendation |
|------|---------|-----------------|----------------|
| E101 | InvalidLineFormat | Only in CLAN error_map | Keep variant (CLAN CHECK mapping) but document as "never emitted by our parsers"; or implement as an explicit fallback in `error_analysis/line.rs`. |
| E345 | UnmatchedContentAnnotationBegin | Only in CLAN error_map | No emitter. Paired with E346 (which IS emitted). Either delete or implement a producer in `cross_utterance/`. |
| E348 | MissingOverlapEnd | Only in CLAN error_map | No emitter. Overlap imbalance is currently handled by the grammar as E316. Either delete or add a post-parse check. |
| E353 | MissingOtherCompletionContext | Only in flag-gated `completion.rs` | Actually Pattern D; moved here only for contrast. |
| E381 | PhoParseError | Nowhere | Spec description references Chumsky, which was eliminated. Remove variant. |
| E383 | GraParseError | Only in CLAN error_map | No emitter. `%gra` parse failures surface as E600/E708/E709 today. Remove, or implement a tier-level fallback. |
| E384 | SinParseError | Nowhere | Parallel to E381. Remove variant. |
| E720 | MorGraCountMismatch | Nowhere (only in parity audits) | Known gap: %mor-vs-%gra count comparison not implemented. Implement alongside E705/E706. |
| E365 | MalformedTierContent | Once in `header_dispatch/parse.rs` | Very narrow. Candidate for consolidation with E600/E602. |

### Pattern D — Flag-gated (8 codes)

All in
`crates/talkbank-model/src/validation/cross_utterance/*.rs`, gated by
`ValidationContext.shared.enable_quotation_validation` (default
`false`). Will emit only when the flag is turned on; that is blocked on
the broader quotation/linker audit tracked elsewhere.

| Code | Variant | Source |
|------|---------|--------|
| E341 | UnbalancedQuotationCrossUtterance | `quotation_follows.rs` (3 sites) |
| E344 | InvalidContentAnnotationNesting | `quotation_precedes.rs` |
| E346 | UnmatchedContentAnnotationEnd | `quoted_linker.rs` |
| E351 | MissingQuoteBegin | `completion.rs` |
| E352 | MissingQuoteEnd | `completion.rs` |
| E353 | MissingOtherCompletionContext | `completion.rs` |
| E354 | MissingTrailingOffTerminator | `completion.rs` |
| E355 | InterleavedContentAnnotations | `completion.rs` |

## Follow-up Work

- **Pattern A (21)**: rewrite spec examples from live, failing test
  inputs mined out of `corpus/reference/` or the wild corpus. Use
  `chatter validate` as the oracle. Update `Status: implemented` once
  the new example produces the declared code.
- **Pattern B (16)**: add a `Status note:` line per spec explaining the
  preemption and what code actually fires. Several already have this
  note; finish the rest. Keep `not_implemented` — these are truthful
  markers of what cannot be reproduced through the tree-sitter path.
- **Pattern C (9)**: decide per-code whether to delete the variant or
  implement a producer. Remove `E381`, `E384` outright (reference to
  extinct Chumsky parser). For the CLAN-parity variants (`E101`,
  `E345`, `E348`, `E383`) decide whether to implement or retire.
  `E720` is the only one that definitely needs implementing (clear
  user-visible gap).
- **Pattern D (8)**: these become `Status: implemented` automatically
  when `enable_quotation_validation` defaults to `true`. Track with the
  cross-utterance validator rollout.

## Caveats

This audit is a snapshot against the repository at branch `main`,
2026-04-13. Line numbers in the emission-site table will drift; the
`ErrorCode::Variant` strings are the stable identifiers.

No spec files or source files were modified by this audit.
