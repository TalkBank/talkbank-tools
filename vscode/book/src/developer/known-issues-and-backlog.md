# Known Issues & Backlog

**Status:** Current
**Last updated:** 2026-04-16 22:30 EDT

This chapter tracks triaged issues and deferred work across the VS Code
extension and its backing Language Server. Each entry carries a stable
anchor ID so other docs, `CLAUDE.md` files, and commit messages can
reference it. Completed items are **archived**, not deleted, so the
history remains auditable.

## Open

### <a id="kib-001"></a>KIB-001 — Typed `AlignmentPair<S, T>` across real alignments — ✅ complete 2026-04-16

**Status:** closed. The four real alignment tiers now carry
compiler-enforced source/target newtypes. `%wor` is explicitly
excluded; see [KIB-016](#kib-016).

**What landed.**

`AlignmentPair` is parameterized `AlignmentPair<S = usize, T = usize>`
with `usize` defaults so pre-KIB-001 call sites that don't name a
newtype continue to compile. The four alignment tiers that are
semantically 1:1 positional correspondences now pin concrete index
types:

| Tier | Pair alias | Source | Target |
|------|------------|--------|--------|
| `%mor` | `MorAlignmentPair` | `MainWordIndex` | `MorItemIndex` |
| `%gra` | `GraAlignmentPair` | `MorChunkIndex` | `GraIndex` |
| `%pho` / `%mod` | `PhoAlignmentPair` | `MainWordIndex` | `PhoItemIndex` |
| `%sin` | `SinAlignmentPair` | `MainWordIndex` | `SinItemIndex` |

`AlignableTier` grew associated `Source`/`Target` types and
`positional_align` returns `Vec<AlignmentPair<T::Source, T::Target>>`,
so a future impl of `AlignableTier` can't accidentally use the wrong
index space. The compiler now rejects the exact bug class this KIB was
opened for (e.g. feeding a `MorChunkIndex` where a `MorItemIndex` is
expected, or mixing `PhoItemIndex` and `SinItemIndex` at the highlight
boundary).

Consumers that cross the newtype boundary into raw APIs
(`find_*_item_range`, sidecar JSON, `Location` emission) unwrap with
`.as_usize()` at the seam — the only place typed→untyped conversion
should exist. The sidecar's `collect_alignment_pairs` helper is
generic over the `IndexPair` trait so every tier's typed pairs share
one JSON flattener.

**Why `%wor` is excluded.** `%wor` is not actually a positional
alignment — count mismatches are tolerated, no positional indexing is
consumed anywhere, error codes are dead in practice. Typing it would
sharpen the wrong abstraction. See [KIB-016](#kib-016) for the planned
reclassification to a timing sidecar.

**Verification.** 707 `talkbank-model` tests, 180 `talkbank-lsp`
tests, 102 `parser_equivalence` tests, and downstream
`batchalign-chat-ops` all pass.

### <a id="kib-002"></a>KIB-002 — E316 masks older validator errors on 3 fixtures (medium) — ✅ fixed 2026-04-16

**Status:** closed. The fixtures were already correct; only the
filename prefixes were stale.

On investigation the "E316 masks older errors" framing was slightly
wrong. The `E232_auto.md`, `E242_auto.md`, and `E600_auto.md` spec
files had already been updated to declare `Expected Error Codes: E316`
(the parser-level rejection is the authoritative failure mode now),
but the fixture `.cha` filenames still used the legacy error-code
prefixes, and the test uses the filename prefix as the expected
code. Renaming the three fixtures to match the current expectation
makes the test pass with zero change to parser or validator logic.
See [KIB-C022](#kib-c022).

### <a id="kib-004"></a>KIB-004 — Full extension review (code + docs) — ✅ complete 2026-04-16

**Status:** closed. The original plan had two halves — a code review
and a documentation authoring phase. Both are now done.

**Code review (completed earlier 2026-04-16).** Two independent
passes. The first used three parallel agents (TS extension, Rust
LSP, cross-cutting) and produced KIB-005 through KIB-014. The second
was a focused audit of the ~80 feature-handler files that found the
crate in unusually good shape:

- No panics in production code.
- No index-space confusion anywhere — the KIB-C001/C002/C003 bug
  class is fully guarded by `MorTier::item_index_of_chunk` and the
  typed `GraAlignmentPair`.
- No hand-rolled chunk walks — all route through
  `MorTier::chunks()`.
- No regex over CHAT tokens; no string surgery.
- Parse-health observability via `chat_file_cache::load_chat_file`
  ensures every feature path logs stale-baseline reads.

Three legitimate low-severity observability / defensive-coding
findings consolidated into [KIB-015](#kib-015). One false positive
(`format_mor_word_label` already handles `Option<&MorTier>` via its
`"word N"` fallback) and two pure nits dropped on review.

**Documentation authoring (completed late 2026-04-16).** The
original plan's Phase F was nearly unbuilt when the code review half
first closed. A second pass added what was missing:

- New [Reference section](../reference/alignment-indices.md) —
  four pages authored: `alignment-indices.md`,
  `rpc-contracts.md` (12 endpoints), `webview-contracts.md`
  (7 panels), `commands.md` (45 commands).
- New [Design Decisions](../design/adr-001-lsp-over-embedded-parser.md)
  section — four ADRs authored: LSP-over-embedded-parser,
  effect-based command runtime, webview panels over TreeView,
  bundled LSP binary.
- `developer/` pages audited against current code and rewritten
  where drifted: `adding-features.md` (modern error types, test
  fixtures, effect runtime), `custom-commands.md` (shrunk to
  pointer at reference page), `lsp-protocol.md` (error contract
  section added), `testing.md` (new `test_fixtures` module +
  mandatory regression gates).
- `navigation/alignment.md` and `navigation/dependency-graphs.md`
  gained Stale-baseline indicator sections mirroring
  [KIB-013](#kib-013).
- `SUMMARY.md` wired the two new sections into the book TOC.

The docs phase closure is distinct from the code-review phase
closure — conflating them in the earlier close of this KIB was the
honest failure that a direct "are we really done?" question
surfaced. This entry now reflects both halves truthfully.

### <a id="kib-015"></a>KIB-015 — Drive-by observability / defensive-coding improvements (low) — ✅ fixed 2026-04-16

**Status:** all three items closed. See [KIB-C025](#kib-c025).

### <a id="kib-005"></a>KIB-005 — `review.ts` status-bar item is never disposed (high) — ✅ fixed 2026-04-16

See [KIB-C008](#kib-c008) for the closure. Kept in Open for one revision
so references elsewhere still resolve; move to Completed section on the
next backlog pass.

**Area:** `vscode/src/commands/review.ts:43–50`.

A module-level `let statusBarItem: vscode.StatusBarItem | undefined` is
created once on first `startReview` and never disposed. On extension
reload or a second review session the old item leaks (stays in memory;
the `if (!statusBarItem)` guard silently no-ops the would-be new
creation). The rest of the stateful modes (coder, transcription,
walker) correctly scope state inside Effect-injected stores; review is
the outlier.

**Remediation.** Move the status-bar item into `ReviewCommandStateStore`
or dispose/recreate it explicitly in `startReview` / `stopReview`:
```ts
statusBarItem?.dispose();
statusBarItem = undefined;
```

**Effort:** small.

### <a id="kib-006"></a>KIB-006 — Large TypeScript modules split — ✅ complete 2026-04-16

**Status:** closed. Both extractions landed; pattern now established
for any future per-panel splits (waveform, analysis, idEditor,
kideval) as drive-by work.

**Shipped.**

- `vscode/src/lsp/executeCommandErrors.ts` — the three tagged error
  classes (`ExecuteCommandRequestError`, `ExecuteCommandResponseError`,
  `ExecuteCommandServerError`) and the `ExecuteCommandStructuredError`
  union moved out of `executeCommandClient.ts`. The client module
  re-exports them so existing consumers don't change their import
  paths.
- `vscode/src/webviewContracts/mediaPanelContract.ts` — the
  MediaPanel message shapes, schemas, decoder, and message
  constructors (~185 lines) moved out of
  `webviewMessageContracts.ts`. The umbrella contracts file retains
  re-exports for backward compatibility.

155 TypeScript tests pass.

### <a id="kib-007"></a>KIB-007 — LSP crate uses `Result<T, String>` throughout (medium) — ✅ production code migrated 2026-04-16

**Status:** complete for production code. All 4 remaining non-test
`Result<_, String>` sites are intentional:

- `crates/talkbank-lsp/src/highlight.rs` (2 sites) — tree-sitter
  internals. The boundary is at `semantic_tokens.rs` via
  `HighlightFailed`. Migrating the tree-sitter-highlight layer
  itself is a separate concern, not a KIB-007 deliverable.
- `alignment/tests.rs` + `requests/alignment_sidecar.rs` test
  helpers — `Result<_, String>` used for `?` ergonomics in test
  code, explicitly acceptable per workspace CLAUDE.md.

**`LspBackendError` variants (15).** `MissingTier`,
`AlignmentMetadataMissing`, `TierAlignmentMissing`,
`InvalidChunkIndex`, `MissingGraAlignmentPair`, `InvalidGraHeadIndex`,
`HighlightFailed`, `ParseFailure`, `ParseFailureEmpty`,
`LanguageServicesUnavailable`, `DocumentNotFound`, `InvalidRegex`,
`SerializationFailed` (`#[from] serde_json::Error`), `InvalidUri`,
`ExternalServiceFailed`, `UnknownCommand`, `InvalidArgument`.

**Migrated subsystems (7).**

| Subsystem | Reference |
|-----------|-----------|
| `graph/` (mod + builder + edges + tests) | [KIB-C013](#kib-c013) |
| `semantic_tokens.rs` + `language_services` | [KIB-C014](#kib-c014) |
| `requests::context::get_chat_file` + 4 callers | [KIB-C015](#kib-c015) |
| `backend/chat_ops/` (entire module) | [KIB-C016](#kib-c016) |
| `backend/analysis.rs` | [KIB-C017](#kib-c017) |
| `backend/execute_commands.rs` (payload parsing) | [KIB-C018](#kib-c018) |
| `backend/participants.rs` | [KIB-C019](#kib-c019) |

**Followup opportunity (not KIB-007).** Three near-identical copies
of `get_chat_file` / `format_parse_failure` live in
`requests/context.rs`, `chat_ops/shared.rs`, and `participants.rs`.
They should collapse into a single shared module
(`backend::chat_file_cache` or similar) in a focused future
refactor.

### <a id="kib-008"></a>KIB-008 — Stale-tree-on-parse-taint finding was a misread (resolved 2026-04-16)

**Status:** closed / not a bug. Reopened only if a concrete symptom appears.

On review, the behavior in
`backend/diagnostics/validation_orchestrator.rs:325` is **intentional**:
when a transient edit makes the document un-parseable, the orchestrator
deliberately sets `parse_clean[uri] = false` but **keeps** the old
`ChatFile` and parse tree as a fallback baseline. This lets features
like hover continue to show the last-known-good state instead of going
dark on every mid-edit syntax error.

The real issue the agent was gesturing at is not "the cache is stale";
it's "feature handlers do not consult `parse_clean` before using the
baseline." That is [KIB-013](#kib-013). KIB-008 is subsumed by
KIB-013; leaving this entry in place as a breadcrumb for future agents
who run the same audit.

### <a id="kib-009"></a>KIB-009 — `package.json` command drift is hard to diagnose (medium) — ✅ fixed 2026-04-16

See [KIB-C009](#kib-c009).

**Area:** `vscode/package.json` `contributes.commands`.

86 commands declared in `contributes.commands`; only 41 are registered
by `vscode/src/activation/commands/*.ts` via `registerEffectCommand`.
The 45 "missing" handlers fall into three categories:
- **5** validation-explorer commands wired in
  `src/activation/validation.ts` (direct `vscode.commands.registerCommand`).
- **12** server-side RPC commands dispatched via `talkbank-lsp`'s
  `execute_commands.rs`.
- **28** effect-command wrapper commands registered via other
  patterns.

A successor reading `package.json` cannot tell which category a given
command falls into without tracing. Not a bug; a discoverability
hazard.

**Remediation.** Add a short top-of-section comment in
`vscode/package.json` explaining the three wiring paths and linking
each to its registration module. Optionally, add a test that asserts
every `contributes.commands` entry has *some* handler somewhere.

**Effort:** trivial (docs) / small (+test).

### <a id="kib-010"></a>KIB-010 — Missing diagrams in high-traffic book pages (medium) — ✅ resolved 2026-04-16

All actionable items closed. See [KIB-C010](#kib-c010) (sequence
diagram for custom-commands + state diagram for coder workflow) and
[KIB-C012](#kib-c012) (analysis pipeline flowchart +
transcription-workflow flowchart).

The original scout also suggested flowcharts for
`analysis/frequency.md` and `analysis/profiling.md`, but on review
those pages are per-command **reference** pages, not pipelines — the
pipeline diagram on `analysis/running-commands.md` (see KIB-C012)
covers the actual flow that both reference pages inherit.

Per `talkbank-tools/CLAUDE.md` seven diagram rules, pages that describe
a protocol, state machine, or pipeline must include a Mermaid diagram.
Three pages currently describe these in prose only:

| Page | Needed diagram |
|------|----------------|
| `book/src/developer/custom-commands.md` | `sequenceDiagram` of a typical `talkbank/*` RPC round-trip (TS client → stdio → Rust handler → typed response) |
| `book/src/coder/workflow.md` | `stateDiagram-v2` covering idle → active → stepping → complete with decision points for "already coded?" |
| `book/src/analysis/frequency.md`, `profiling.md`, `workflows/transcription.md` | `flowchart LR` per page for the pipeline, with one decision diamond each |

Each diagram must carry a `<!-- Verified against: … -->` footer listing
the source files it was derived from (diagram rule 7).

**Effort:** small per diagram; medium as a batch.

### <a id="kib-011"></a>KIB-011 — LSP crate files over the 400-line soft cap — two splits landed, rest drive-by

**Status:** two extractions shipped; the remaining oversized files
stay as drive-by work because no further clean seams are obvious.

**Shipped.**

- `backend/line_offsets.rs` — line-offset utilities
  (`compute_line_offsets`, `find_line_for_offset`) carved out of
  `backend/incremental.rs`. 81 lines with its own unit tests.
- `backend/execute_command_args.rs` — the five argument parsers
  (`expect_string_argument`, `parse_uri_argument`, `parse_uri_string`,
  `parse_json_argument`, `parse_position_argument`) moved out of
  `backend/execute_commands.rs`. Dropped the top-level file from
  659 → 596 lines.

**Remaining over the soft cap.**

| File | Lines | Why not split yet |
|------|------:|-------------------|
| `backend/execute_commands.rs` | 596 | The 15 request struct impls are cohesive — they all live next to `ExecuteCommandName` and `ExecuteCommandRequest` because their `from_arguments` methods reference the same dispatch tables. Splitting them would fragment the request family. |
| `backend/incremental.rs` | 602 | Tightly coupled to tree-sitter state; no obvious secondary seam after the line-offset extraction. |
| `backend/diagnostics/validation_orchestrator.rs` | 598 | Multiple concerns live here but each one reaches into orchestrator state. Needs design, not mechanical carve-up. |
| `backend/requests/text_document.rs` | 496 | Under 500. Fine. |

None exceed the 800-line hard cap. Further splits remain drive-by:
when a feature change touches one of these files, look for a
freshly-cohesive cluster and extract then.

### <a id="kib-012"></a>KIB-012 — `graph/edges.rs` should use `GrammaticalRelation::head_ref()` (low) — ✅ fixed 2026-04-16

See [KIB-C011](#kib-c011).

**Area:** `crates/talkbank-lsp/src/graph/edges.rs:33,56–66`.

The edge builder reads `rel.head` as raw `usize` and branches on
`to_idx == 0` to distinguish ROOT from a word reference. Phase B
(2026-04-16) added `GrammaticalRelation::head_ref() -> GraHeadRef`
for exactly this case. The current code is correct (the `0` branch is
handled), but explicit is better than implicit when the enum exists.

**Remediation.** Replace the `if to_idx == 0 { 0 } else { to_idx - 1 }`
pattern with `match rel.head_ref() { GraHeadRef::Root => 0,
GraHeadRef::Word(idx) => idx.to_chunk_index().as_usize() }`.

**Effort:** trivial.

### <a id="kib-013"></a>KIB-013 — `parse_clean` had zero readers in feature handlers — ✅ complete 2026-04-16

**Status:** closed. Primitive + observability landed earlier; UX
markers ratified and implemented on 2026-04-16.

**What shipped.**

- `Backend::parse_state(uri) -> ParseState { Clean | StaleBaseline | Absent }`
  + tracing on stale-baseline cache hits (existing, KIB-C020).
- **Hover cards** (alignment-consuming): append a markdown footer
  when `StaleBaseline`. Format:
  ```markdown
  ---

  > ⚠ **Stale baseline** — alignment reflects the last successful parse.
  ```
  Blockquote gives semantic hierarchy in rendered markdown; the
  vocabulary `stale baseline` mirrors the `ParseState::StaleBaseline`
  identifier so the term flows from source → tracing logs →
  user-facing text.
- **Dependency graph (DOT)**: emit a muted top-left label when
  `StaleBaseline`:
  ```
  label="stale baseline"; labelloc="t"; labeljust="l";
  fontcolor="#888888"; fontname="Courier"; fontsize="10";
  ```
  Placement + Courier + gray signal meta-information; subordinate to
  the actual graph content.
- **Other features** stay silent by design. Go to Definition,
  highlights, inlay hints: the result is still typically correct on
  `StaleBaseline`, and a warning there would be noise. Document-level
  diagnostics intentionally not added — would spam the Problems
  panel.
- **Absent**: hover returns `None` (client shows no card); graph
  returns the existing typed `DependencyGraphResponse::Unavailable`.
  No fabricated results.

**Regression coverage.** Two tests:
- `alignment_hover_appends_stale_baseline_footer` in
  `backend/features/hover.rs` — asserts the footer appears under
  `StaleBaseline` and is absent under `Clean`.
- `stale_baseline_emits_muted_label_attrs` in `graph/tests.rs` —
  asserts the DOT label, fontcolor, and Courier font-name attributes
  appear under `StaleBaseline` and are absent under `Clean`.

**Design rationale.** See the frontend-design-skill-informed proposal
captured in chat: commit to "industrial meta-information" as the
aesthetic direction rather than the first generic italic-footer choice.
Both surfaces share the same vocabulary (`stale baseline`) and visual
weight (muted, subordinate, non-competing with primary content).

183 LSP tests pass.

### <a id="kib-014"></a>KIB-014 — Stale `Last updated` timestamps (low) — policy decision 2026-04-16

**Status:** accepted as-is; no bulk action.

The workspace `CLAUDE.md` is explicit: *"Do NOT do a bulk sweep to
stamp dates on docs you haven't verified — that creates false
confidence."* A book page with a 2026-03-30 timestamp is not a bug
per se; it means no-one has yet re-read the page to confirm the
content still matches reality.

The policy: **update a page's timestamp when you actually verify or
edit it, not before.** A successor reading an old timestamp treats it
as a "hasn't been re-reviewed" signal rather than a "wrong" signal.
Individual pages graduate out of "stale" as drive-by maintenance
touches them.

A future `tb` command or pre-push hook could report which pages haven't
been touched in N days as a gentle nudge, but that's a separate
tooling concern, not a backlog item for the book itself.

### <a id="kib-018"></a>KIB-018 — LSP test harness duplicated across 15 modules — ✅ fixed 2026-04-16

**Status:** closed. `crates/talkbank-lsp/src/test_fixtures.rs` now
hosts the shared helpers `parse_chat`, `parse_chat_with_alignments`,
`parse_tree` (direct `tree_sitter::Parser`), and
`parse_tree_incremental` (via `TreeSitterParser`, for tests that
exercise incremental parsing). Ten test modules migrated in one
sweep: `alignment/tier_hover/gra_tier`,
`backend/features/{highlights,code_lens,completion,rename,references,linked_editing,folding_range,document_symbol}`,
`backend/requests/{formatting,alignment_sidecar}`.

The `alignment_sidecar` tests also dropped their
`Result<(), String>` return types — now panic on fixture parse
failure the same as the rest of the crate.

Future test modules must use `crate::test_fixtures::…`; redefining
local helpers is banned. 181 LSP tests all passing.

### <a id="kib-017"></a>KIB-017 — `%gra` hover `aligned_to_mor` / `aligned_to_main` mis-indexed post-clitics — ✅ fixed 2026-04-16

**Status:** closed via TDD (RED → GREEN → regression) on
2026-04-16 20:58 EDT.

**Was.** `find_gra_tier_hover_info` in
`crates/talkbank-lsp/src/alignment/tier_hover/gra_tier.rs:88-104`
treated a `MorChunkIndex` (the source side of `gra_alignment.pairs`)
as a `MorItemIndex` — indexed `mor_tier.items[mor_idx]` directly and
looked up `mor_alignment.pairs` with the same chunk value. For any
`%gra` relation on a post-clitic chunk the hover's `aligned_to_mor`
rendered the item *after* the post-clitic's host, and
`aligned_to_main` rendered the main-tier word for that wrong item.
Primary hover content (Word / Head / Dependents via
`format_mor_word_label`) was unaffected because it already routed
through the chunk primitive.

**Fix.** Collapse chunk → host item via
`MorTier::item_index_of_chunk(chunk_idx)` before indexing `.items`
and before looking up the main↔mor alignment. Pattern now matches
`tier_handlers.rs:346` and `goto_definition.rs:256`. Regression test
`gra_hover_on_post_clitic_aligns_to_host_mor_item` lives in
`gra_tier.rs` — it asserts `aligned_to_mor` renders the host item
(`pron|it~aux|be`'s `POS: pron` + `Post-clitic: aux|be`) and does not
mention the next item's lemma (`cookie`).

**Same bug class as** [KIB-C001](#kib-c001), [KIB-C002](#kib-c002),
[KIB-C003](#kib-c003). This was the fourth and final site that had
escaped the 2026-04-16 migration.

**Verification.** 710 model + 181 LSP (one new) + 102
parser-equivalence + 98 roundtrip tests all passing.

### <a id="kib-016"></a>KIB-016 — Reclassify `%wor` as a timing sidecar, not an alignment — ✅ complete 2026-04-16

**Status:** closed. The `WorAlignment` / `align_main_to_wor` /
`WorTier: AlignableTier` machinery has been removed. `%wor` is now
modeled as [`WorTimingSidecar`](crate::alignment::WorTimingSidecar),
carried as `AlignmentSet.wor_timings: Option<WorTimingSidecar>`.

**What shipped.**

- New enum `WorTimingSidecar { Positional { count } | Drifted { main_count,
  wor_count } }` in `talkbank-model::alignment::wor`. No error stream,
  no pair Vec — count match is the trivial 1:1 correspondence, and drift
  is a data state, not a diagnostic.
- New function `resolve_wor_timing_sidecar(main, wor) -> WorTimingSidecar`
  replaces `align_main_to_wor`. Never returns `ParseError`.
- `AlignmentSet.wor: Option<WorAlignment>` renamed to
  `AlignmentSet.wor_timings: Option<WorTimingSidecar>`. On parse-taint
  (`!can_align_main_to_wor()`), the slot stays `None` — tainted input
  has no sidecar to report; consult `ParseHealth` directly for taint
  context.
- `WorTier: AlignableTier` impl removed. `AlignableTier`'s trait doc
  now names only the four structural alignments (%mor, %gra, %pho, %sin).
- Previously-hacked workarounds retired: `AlignmentSet::is_error_free()`
  and `collect_errors()` no longer need comments-and-skips for `%wor`,
  because there are no `%wor` errors in existence. Both tests that
  guarded the workaround are gone; the invariant is structural.
- `AlignmentSet.collect_errors()` comment and `chat_file::validate`
  rewritten to reflect the new model.
- `batchalign-chat-ops::fa::orchestrate::collect_wor_backed_timings`
  consumes `WorTimingSidecar::positional_count()` directly — no more
  `is_error_free()` gating.
- LSP `alignment_sidecar.rs` synthesizes trivial positional pairs for
  the TS wire format when `Positional`, emits empty when `Drifted`.
- LSP `CLAUDE.md` three-index-space section now explicitly calls out
  that `%wor` is not an alignment; do not reach for
  `%mor`/`%pho`-style helpers.
- Four alignment location-tests and three metadata tests migrated to
  assert against the new sidecar shape.

**Correction to original KIB-016 writeup.** The original entry claimed
E718/E719 were the dead `%wor` error codes. That was wrong —
`E718`/`E719` are `SinCountMismatch{TooFew,TooMany}` and are live for
`%sin`. The `%wor` `AlignableTier` impl piggybacked on
`ErrorCode::PhoCountMismatch{TooFew,TooMany}` (E715/E716), which are
legitimately live for `%pho`. No error codes were retired — the
dead-code element was only the `WorAlignment.errors` field, which
disappeared with the whole type.

**Verification.** 845 talkbank-model + LSP + CLI tests, 102
parser-equivalence, 98 roundtrip reference corpus, 11 wor-terminator
tests, and 763 batchalign-chat-ops tests all pass.


## Completed

These items are archived as evidence of closed work; the commits that
fixed each are in git history.

### <a id="kib-c001"></a>KIB-C001 — Post-clitic `%gra` hover label (Bug #3) — 2026-04-16

`format_mor_word_label` indexed `mor.items` directly with
`word_index - 1`, so hovering a `%gra` relation whose dependent is a
post-clitic silently showed the wrong lemma (masked by a `word N`
fallback). Fixed by routing through `MorTier::chunk_at`. See
[Cross-Tier Alignment § Clitics and the `%mor` Chunk Sequence](../navigation/alignment.md#clitics-and-the-mor-chunk-sequence).

### <a id="kib-c002"></a>KIB-C002 — `%gra` click highlight on post-clitic (Bug #1) — 2026-04-16

`highlights_from_gra_tier` took a `MorChunkIndex` from
`gra_alignment.pairs` and looked it up in `mor_alignment.pairs.target_index`
(an item index). On any post-clitic, the main-tier TEXT highlight
landed on the next item instead of the host word. Fixed by routing the
chunk through `MorTier::item_index_of_chunk` before indexing the
main↔mor alignment. An earlier audit had incorrectly reported this as
already fixed; a RED regression test was what caught it.

### <a id="kib-c003"></a>KIB-C003 — `%gra` Go to Definition on post-clitic (Bug #1b) — 2026-04-16

`goto_definition.rs` had the same bug class as KIB-C002 and was only
noticed during Phase B plumbing. Fix is symmetric: project chunk →
item via `MorTier::item_index_of_chunk` before indexing the main↔mor
alignment.

### <a id="kib-c004"></a>KIB-C004 — Dependency graph edges on post-clitic (Bug #2) — fixed earlier, pinned down 2026-04-16

Dependency-graph DOT edges connected the wrong nodes when an utterance
contained a post-clitic. Commit `525273c2` had fixed the underlying
code; a regression test was added on 2026-04-16 to pin down the
behavior.

### <a id="kib-c005"></a>KIB-C005 — Chunk walker duplication — 2026-04-16

Five different places in the workspace walked the `%mor` chunk
expansion by hand (`MorTier::count_chunks`, `align_mor_to_gra`'s
`extract_mor_chunk_items`, the LSP hover helper, the graph DOT label
builder, and the LSP's new `%gra` hover fix). All now delegate to the
single `MorTier::chunks()` iterator.

### <a id="kib-c008"></a>KIB-C008 — Review mode status-bar resource leak — 2026-04-16

`vscode/src/commands/review.ts` created a module-level
`vscode.StatusBarItem` lazily and called `.hide()` on `stopReview`
instead of `.dispose()`. Each extension reload / repeat review
session leaked one handle. Fixed by introducing
`disposeReviewStatusBar()` (called from `stopReview` and wired into
`context.subscriptions` via `registerReviewStatusBarCleanup()` in
`extension.ts` activation). The new function disposes the item and
clears the module-level variable so the next `startReview` creates a
fresh one.

### <a id="kib-c009"></a>KIB-C009 — Command-handler registration map documented — 2026-04-16

Added a "Where command handlers live" reference table to
`developer/adding-features.md` explaining the three wiring paths:
`registerEffectCommand` (most commands), direct
`vscode.commands.registerCommand` (validation explorer only), and
`backend/features/execute_commands.rs` (the 12 server-side RPC
commands). A successor tracing a `contributes.commands` entry back
to its handler now has a single-page reference for which file to
read.

### <a id="kib-c010"></a>KIB-C010 — Custom-commands sequence diagram + coder state machine diagram — 2026-04-16

`developer/custom-commands.md` gained a 10-participant
`sequenceDiagram` showing a `talkbank/*` RPC round-trip: caller →
`TalkbankExecuteCommandClient` → LSP stdio → dispatch → handler →
model → response, with the typed-boundary rule enumerated below it.
`coder/workflow.md` gained a `stateDiagram-v2` documenting the
five-state coder lifecycle (Idle → LoadingCutFile →
ScanningUtterances → AtUncoded ↔ PickingCode ↔ Inserting →
Complete), verified against `vscode/src/coderPanel.ts`. Both
carry `<!-- Verified against: … -->` footers per the seven diagram
rules.

### <a id="kib-c011"></a>KIB-C011 — `graph/edges.rs` adopts `GrammaticalRelation::head_ref()` — 2026-04-16

The dependency-edge builder previously branched on
`to_idx == 0` to handle the ROOT sentinel. Refactored to
`match rel.head_ref() { GraHeadRef::Root => 0, GraHeadRef::Word(idx)
=> idx.to_chunk_index().as_usize() }`, so the ROOT case is now a
named enum variant rather than an inline magic-number comparison.
All 11 graph tests still pass.

### <a id="kib-c025"></a>KIB-C025 — Drive-by observability / defensive-coding improvements landed — 2026-04-16

All three items from KIB-015 fixed:

1. **`goto_definition.rs`** — replaced the silent
   `mor_idx.min(mor_tier.items.len().saturating_sub(1))` clamp with
   an explicit bounds check that logs `tracing::debug!` and
   returns `None`. An out-of-range cursor now declines the jump
   instead of silently landing on the last `%mor` item.
2. **`linked_editing.rs`** — the `.ok()?` on
   `speaker_node.utf8_text(doc.as_bytes())` now matches explicitly
   and emits `tracing::warn!(start, end, error)` on decode
   failure before returning `None`. Operators can correlate the
   event with other incremental-parse symptoms instead of
   assuming linked editing is broken.
3. **`highlights/tier_handlers.rs`** — added a shared generic
   helper `find_alignment_index_for_target<P: IndexPair>(…)` that
   performs the reverse lookup once, logs `tracing::debug!` on
   no-match, and returns `Option<usize>`. Applied to all five
   call sites (%mor, %pho, %mod, %sin, %mor-via-%gra). Replaces
   five copies of the `.enumerate().find(|(_, p)| p.target_index
   == Some(idx))?` pattern with a single call and gives operators
   a uniform debug signal when the alignment cache is out of sync
   with the CST lookup.

180/180 talkbank-lsp tests pass post-change.

### <a id="kib-c024"></a>KIB-C024 — `get_chat_file` + `format_parse_failure` duplication collapsed — 2026-04-16

Three near-identical copies of `get_chat_file` / `format_parse_failure`
— in `backend/requests/context.rs`, `backend/chat_ops/shared.rs`, and
`backend/participants.rs` — collapsed into a single shared module
`backend/chat_file_cache.rs` with two public functions
(`load_chat_file`, `load_document_and_chat_file`) and one private
helper (`parse_failure_from`). The three former modules now delegate
through thin aliases that preserve their historical call-site names
so feature handlers don't need edits.

Bonus simplifications:
- The stale-baseline `tracing::debug!` logging from KIB-C020 moved
  from `context.rs` into the shared module, so every handler path
  gets the KIB-013 observability without duplication.
- The private legacy stringly `format_parse_failure` helper in
  `participants.rs` (marked `#[allow(dead_code)]`) was deleted
  along with the now-unused `ParseErrors` import.

Three files shrank: `context.rs` 80→48, `chat_ops/shared.rs` 67→31,
`participants.rs` 299→256. Net +23 lines across the four files, but
one source of truth instead of three drifting ones.

### <a id="kib-c023"></a>KIB-C023 — `PhoItemIndex`, `SinItemIndex`, `WorItemIndex` added — 2026-04-16

Rounded out the tier-specific index-space newtypes in
`talkbank-model::alignment::indices`. With `MainWordIndex`,
`MorItemIndex`, `MorChunkIndex`, `GraIndex`, `PhoItemIndex`,
`SinItemIndex`, and `WorItemIndex` all declared (each
`#[serde(transparent)]`, each with a no-op `SpanShift` impl), the
full generic `AlignmentPair<S, T>` refactor KIB-001 describes has
every type it needs. The refactor itself is still pending and
remains the rest of KIB-001.

### <a id="kib-c022"></a>KIB-C022 — E316 fixture filenames stopped matching their expected codes — 2026-04-16

Three fixtures under `tests/error_corpus/validation_errors/` kept
legacy filename prefixes (`E232_`, `E242_`, `E600_`) after the
parser/validator work that made them produce `E316` instead. The
spec files (`E232_auto.md`, `E242_auto.md`, `E600_auto.md`) had
already been updated to declare `Expected Error Codes: E316`; the
fixture filenames were the last bit to drift. Renamed via
`git mv`:

- `E232_compound_marker_at_start.cha` → `E316_compound_marker_at_word_start.cha`
- `E242_unbalanced_quotation.cha` → `E316_unbalanced_quotation.cha`
- `E600_tier_validation_error.cha` → `E316_invalid_mor_tier.cha`

The `validation_errors_detected` test now passes. Zero parser or
validator logic changed.

### <a id="kib-c021"></a>KIB-C021 — Line-offset utilities extracted into `backend/line_offsets.rs` — 2026-04-16

Extracted the pure text-to-line-offset helpers
(`compute_line_offsets`, `find_line_for_offset`) out of
`backend/incremental.rs` into a new `backend/line_offsets.rs`
module (81 lines), along with their four unit tests. The helpers
are `ChatFile`-agnostic and useful for any future consumer that
needs byte-span-to-line mapping (e.g. a feature handler mapping
tree-sitter node spans to line ranges). `incremental.rs` went from
646 to 602 lines and now imports the helpers from the sibling
module. 180/180 talkbank-lsp tests pass post-split.

### <a id="kib-c020"></a>KIB-C020 — `Backend::parse_state` primitive + stale-baseline logging — 2026-04-16

`Backend::parse_state(uri) -> ParseState` was added to `state.rs`
with the three variants `Clean` / `StaleBaseline` / `Absent` that
KIB-013 asked for. `context::get_chat_file` now consults it on
every cache hit and emits `tracing::debug!(uri = %uri, "serving
feature request from stale %mor baseline (KIB-013)")` when the
baseline is stale, so operators can grep LSP logs for the
pattern and judge how often this is hit in real use. Feature
handlers still render against the stale baseline as before
(deliberate graceful degradation per
`validation_orchestrator.rs:325`); the remaining work — per-feature
UX markers to *tell the user* the content is stale — is tracked
as the "what's left" section of KIB-013 and deliberately deferred
so observability comes before UX investment.

### <a id="kib-c019"></a>KIB-C019 — `backend/participants.rs` migrated to `LspBackendError` — 2026-04-16

`handle_get_participants`, `handle_format_id_line`, the local
`command_response` wire adapter, and a third-copy `get_chat_file` +
`parse_failure_from` helper migrated. The `SerializationFailed`
`#[from]` conversion means the body of `handle_get_participants`
uses plain `?` on `serde_json::to_value`. Third-copy
`get_chat_file` is retained locally and flagged in the module
docs for the future shared-module consolidation.

### <a id="kib-c018"></a>KIB-C018 — `backend/execute_commands.rs` payload parsing migrated — 2026-04-16

Top-level `ExecuteCommandName::parse` plus 8 `from_arguments`
decoders plus the four `parse_*_argument` helpers migrated to
`Result<_, LspBackendError>`. Added two new typed variants —
`UnknownCommand { name }` for bad command IDs and
`InvalidArgument { label: &'static str, reason }` for every
positional argument failure (missing, wrong type, malformed JSON).
Helper label parameters switched from `&str` to `&'static str`
since all call sites pass string literals, letting the enum hold
the label without cloning. Tests' `.contains()` string assertions
were replaced with `matches!(err, LspBackendError::Variant { .. })`
patterns.

### <a id="kib-c017"></a>KIB-C017 — `backend/analysis.rs` migrated to `LspBackendError` — 2026-04-16

`handle_analyze`, `build_analysis_options`, `handle_discover_databases`,
and the local `command_response` wire adapter all use typed errors.
Added `InvalidUri { label, reason }` for URL-parse and non-file-URI
failures and `ExternalServiceFailed { service, reason }` to wrap
`talkbank_clan::AnalysisServiceError` and database-discovery errors.
The one test that asserted on the error message switched to a
`matches!(err, LspBackendError::InvalidUri { label: "Invalid second file URI", .. })`
pattern.

### <a id="kib-c016"></a>KIB-C016 — `backend/chat_ops/` subsystem migrated to `LspBackendError` — 2026-04-16

Added three typed variants — `DocumentNotFound`, `InvalidRegex`,
`SerializationFailed` (with `#[from] serde_json::Error`) — and
migrated the entire `chat_ops/` subsystem: `shared.rs` helpers,
`format_bullet.rs`, `scoped_find.rs` (including the `build_matcher`
and `test_build_matcher` helpers), `filter_document.rs`,
`speakers.rs`, `utterances.rs`, and `mod.rs`'s `command_response`
wire-boundary adapter. Dropped `Clone, Eq, PartialEq` derives on
`LspBackendError` because `serde_json::Error` doesn't implement
them — callers now match on variants and rely on `Display` for
snapshots. 180/180 talkbank-lsp tests pass; batchalign3 clean.

Also flagged a separate refactor opportunity: `chat_ops/shared.rs`
and `requests/context.rs` both define a near-identical
`get_chat_file` / `format_parse_failure` pair. They should collapse
into a single shared helper module in a dedicated future refactor.

### <a id="kib-c015"></a>KIB-C015 — `context::get_chat_file` migrated to `LspBackendError` — 2026-04-16

Added three typed variants — `ParseFailure { count, plural,
first_message }`, `ParseFailureEmpty`, `LanguageServicesUnavailable`
— and rewrote `backend/requests/context::get_chat_file` to return
`Result<Arc<ChatFile>, LspBackendError>`. Callers in
`formatting.rs`, `symbols.rs`, `text_document.rs`, and
`execute_command.rs` stringify at the LSP wire boundary
(`invalid_params(err.to_string())` or `Value::String(err.to_string())`).
The private `format_parse_failure` stringifier is now a typed
`parse_failure_from(ParseErrors) -> LspBackendError`. 180/180
talkbank-lsp tests pass; batchalign3 clean.

### <a id="kib-c014"></a>KIB-C014 — `semantic_tokens.rs` migrated to `LspBackendError` — 2026-04-16

Added a `HighlightFailed { reason: String }` variant to
`LspBackendError` that wraps stringly errors from the underlying
tree-sitter-highlight layer. Migrated
`SemanticTokensProvider::{new, semantic_tokens_full,
semantic_tokens_range}` and `backend::language_services::with_semantic_tokens_provider`
(plus its test fixtures) to return / accept `LspBackendError`. The
`BackendInitError::SemanticTokens(String)` variant is preserved
unchanged — the stringify happens at the `initialize_semantic_tokens`
boundary — so callers and the LSP response shape are untouched.
180/180 talkbank-lsp tests pass post-migration. `highlight.rs`
itself remains on `Result<_, String>` for now; it is tree-sitter
internals and better migrated in a focused pass.

### <a id="kib-c013"></a>KIB-C013 — Typed `LspBackendError` introduced + `graph/` subsystem migrated — 2026-04-16

`crates/talkbank-lsp/src/backend/error.rs` was created with a
thiserror-based `LspBackendError` enum carrying six concrete
variants. The `graph/` subsystem (`mod.rs`, `builder.rs`, `edges.rs`,
plus its tests) was migrated end-to-end: every `Result<_, String>`
return became `Result<_, LspBackendError>`, every stringly `Err(...)`
became a typed variant, and tests switched from `err.contains(...)`
assertions to `matches!(err, LspBackendError::Variant { … })`. The
`build_dependency_graph_response` adapter at the LSP wire boundary
maps the typed error to a `Display`-stringified `reason`, so the
JSON-RPC response shape is unchanged for VS Code extension consumers.
180/180 `talkbank-lsp` tests pass post-migration; `batchalign3`
compiles clean. See KIB-007 for the remaining migration scope.

### <a id="kib-c012"></a>KIB-C012 — Analysis pipeline + transcription workflow diagrams — 2026-04-16

`analysis/running-commands.md` gained a `flowchart LR` showing the
generic CLAN-command pipeline: user → QuickPick → optional input
dialog → single-file-vs-directory branch → LSP `talkbank/analyze` →
`AnalysisRunner` → typed JSON response → `AnalysisPanel` → optional
CSV export. `workflows/transcription.md` gained a `flowchart LR`
documenting the F4 cycle: type → F4 → request playback position from
webview → LSP `formatBulletLine` → insert bullet + new speaker line,
with dashed edges for the sideways playback controls (`F8`,
`Shift+F5`). Both carry `<!-- Verified against: … -->` footers.

### <a id="kib-c007"></a>KIB-C007 — SCREENSHOT placeholders and stale cross-link broke `mdbook build` — 2026-04-16

`mdbook-linkcheck2` was failing the book build on 34 pre-existing
`> **[SCREENSHOT: …]**` placeholders (treated as unresolved
reference-style Markdown links) and on one stale cross-reference
(`coder/workflow.md` pointed at `../editing/scoped-find.md` instead of
the real path `../navigation/scoped-find.md`). Fixed by rewriting the
placeholder syntax to `> **(SCREENSHOT: …)**` across the book and
correcting the one cross-reference. The book now builds with zero
linkcheck warnings.

### <a id="kib-c006"></a>KIB-C006 — Loose `GUIDE.md` / `DEVELOPER.md` / `CLAN-FEATURES.md` in `vscode/` — 2026-04-16

Three ~500–1100-line Markdown files lived alongside the populated
book and duplicated its content. All three were absorbed into the
book (with `Performance Notes` and `Releasing` migrated to
`developer/architecture.md` and a new `developer/releasing.md`
respectively) and the loose files deleted. `CLAUDE.md` files and
`docs/inventory.md` now prohibit re-creating them.

## Related Chapters

- [Architecture](architecture.md) — the `%mor` chunk primitive and three index spaces
- [Cross-Tier Alignment](../navigation/alignment.md) — user-facing behavior
- [Testing](testing.md) — regression tests that pin down the fixed bugs
