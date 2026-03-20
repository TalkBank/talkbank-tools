# Further Restructuring Notes

**Status:** Current
**Last updated:** 2026-03-16

This file records the next round of larger changes that would make the current splits more durable. The work in this refactor intentionally stopped at behavior-preserving extraction and did not try to redesign subsystem boundaries.

The canonical durable follow-up documents are now:

- `book/src/contributing/architecture-audit.md`
- `book/src/contributing/rearchitecture-backlog.md`

Keep this file for wave-specific notes only; move standing findings into the
book pages above.

## Completed In This Round

- `validate_parallel` now has an explicit renderer boundary instead of one branch-heavy runtime loop.
- audit-mode output now uses `AuditReporter` instead of a misleading sink abstraction, and worker threads report through an explicit handle.
- `test-dashboard` now uses worker-to-UI message passing instead of shared mutex state.
- the VS Code extension now routes execute-command calls through a typed client facade.
- the VS Code execute-command boundary now separates transport from structured payload contracts, with a dedicated payload module and runtime decoders for the structured responses the extension actually consumes.
- database discovery now decodes through that same VS Code payload boundary, and the KidEval/Eval panel now uses a tighter typed message contract instead of one optional-field message bag.
- the VS Code extension now also centralizes panel/webview message contracts in `vscode/src/webviewMessageContracts.ts`, with runtime decoders and typed outbound builders for the analysis, ID editor, media, waveform, and KidEval/Eval panels.
- the VS Code extension now also centralizes singleton panel lifecycle mechanics in `vscode/src/panelLifecycle.ts`, so its main webview panels reuse one helper path for reuse/create, invalid-message logging, and disposal instead of duplicating that boilerplate per panel.
- the VS Code extension now has an explicit Effect foundation in `vscode/src/effectRuntime.ts` plus the `effect` runtime dependency, giving the current activation-time service bag a typed `Layer` / `Context` bridge and a tagged async-boundary error wrapper before the deeper migration phases land.
- the first Effect-native TypeScript boundary pass is now complete: `vscode/src/effectBoundary.ts` centralizes shared `Schema` decoders and tagged boundary errors, `executeCommandPayloads.ts` and `webviewMessageContracts.ts` now decode through Effect schemas, and `panelLifecycle.ts` now preserves panel-safe logging around tagged malformed-message failures instead of relying on hand-written shape readers.
- the core VS Code command/runtime path is now Effect-native too: `executeCommandClient.ts` returns Effects plus tagged transport/response/server errors, `effectCommandRuntime.ts` owns the shared command runner and VS Code host services, the command registrars now register Effects instead of promise callbacks, and the async panels now run message-side work through that same runner.
- the remaining persistent media-command state now follows that same path: `mediaCommandState.ts` owns the walker and transcription stores, `walker.ts` and `transcription.ts` no longer depend on module-level mutable globals, and media-command cleanup now goes back through the shared Effect runner instead of mutating exported maps directly.
- `talkbank/analyze` now decodes into a typed options struct at the Rust execute-command boundary, and both Rust and TypeScript read the current analysis-option contract through named fields instead of raw JSON maps.
- `talkbank-clan` now owns typed utterance-range parsing, shared CHAT-file discovery, and direct JSON-value serialization for analysis outputs, so the CLI and LSP no longer need to duplicate those seams or round-trip analysis JSON through strings.
- `talkbank-clan` now also owns a higher-level `AnalysisService` / `AnalysisRequest` boundary, so the CLI and LSP adapt their request shapes into one shared library execution surface instead of each importing and executing most CLAN command types directly.
- `talkbank-clan` now owns a library-side `AnalysisOptions` / `AnalysisRequestBuilder` default-and-validation layer for the LSP analysis path, and `SugarConfig` now has an explicit default that matches its documented semantics instead of silently deriving `0`.
- `talkbank-clan` now owns a typed `AnalysisCommandName`, and the CLI analysis path now routes through that typed identifier plus the shared builder/service layer instead of dispatching most work through raw command strings and hand-built requests.
- `talkbank/analyze` now has a schema-owned editor/server contract in `crates/talkbank-lsp/src/backend/contracts.rs`, and the generated schema is checked in at `schema/analyze-command.schema.json`.
- the generated `talkbank/analyze` schema is now consumed by a shared fixture-validation path: `vscode/src/test/fixtures/analyzeCommandPayload.json` is checked by both the TypeScript payload-builder test and a Rust `jsonschema` integration test.
- `talkbank-lsp` now decodes execute-command payloads through one typed protocol module instead of scattered string matching and positional JSON parsing.
- `talkbank-lsp` execute-command routing now flows through feature-oriented service objects for document commands, analysis, participants, and chat operations instead of one match-heavy request handler.
- `talkbank-cli` top-level command routing now flows through feature-oriented family services in `crates/talkbank-cli/src/commands/dispatch.rs`, `cli/run.rs` is now a composition root, and `validate::run_validate_command` owns the typed alignment/cache/interface plumbing that used to be assembled inline.
- `talkbank-cli` validation cache setup now also routes through one shared helper in `crates/talkbank-cli/src/commands/validate/cache.rs`, so single-file validate, directory validate, audit mode, and watch-triggered validation all share the same cache initialization, `--force` clearing, and validation-result read/write conventions.
- `test-dashboard` is now library-backed instead of binary-only: `src/test_dashboard/` owns the dashboard worker/state/ui modules under the existing `talkbank-tools` library target, and `src/bin/test-dashboard.rs` is only the thin binary entrypoint.
- `test-dashboard` now also separates execution from persistence: `src/test_dashboard/runner.rs` owns corpus/file execution, `src/test_dashboard/manifest.rs` owns manifest mutation and checkpoint saves, and `FileTestOutcome` owns recent-failure formatting instead of leaving that summary assembly inline in the worker loop.
- `talkbank-clan` golden coverage now routes most parity/snapshot cases through manifest-style declarations backed by `crates/talkbank-clan/tests/clan_golden/harness.rs`, while bespoke temp-file tests stay explicit and reviewed baseline snapshots remain in `crates/talkbank-clan/tests/clan_golden/snapshots/`.
- `talkbank-clan/tests/common/mod.rs` now owns shared corpus-root, fixture-path, CLAN-binary discovery, and CLAN process-execution helpers reused by `clan_golden`, `converter_golden`, and `transform_golden`, so those suites no longer each reimplement the same test-only path setup and tempdir-backed CLAN command orchestration.
- `crates/talkbank-clan/tests/converter_golden.rs` now routes its repetitive to-CHAT and from-CHAT converter coverage through shared case runners and generated test declarations, so fixture loading, CHAT parsing, and CLAN skip behavior no longer get rebuilt in each individual converter test.
- `crates/talkbank-clan/tests/transform_golden.rs` now routes its repetitive CLAN parity coverage through a generated parity runner with explicit runner modes for stdin-driven vs file-argument CLAN commands, while the more bespoke rust-only/temp-file transform tests stay explicit.
- `talkbank-lsp/tests/position_conversion/mod.rs` now owns shared assertion helpers for offset↔position and roundtrip checks, so the ASCII/Unicode/CHAT/bounds regression cases no longer each rebuild the same conversion-and-assertion boilerplate inline.
- `talkbank-model::MainTier` now owns `find_context_dependent_ca_omission_span()`, so the tree-sitter parser and direct parser no longer duplicate the same recursive CA-omission / shortening traversal when enforcing parser-context gating.
- `crates/talkbank-direct-parser/src/header/mod.rs` is now a thin header entrypoint: `dispatch.rs` owns byte-prefix parser routing, `standalone.rs` owns the API-compat `@ID` / participant-entry adapters, and `helpers.rs` now also owns the shared parse-error reporting and malformed-header recovery helpers.
- `crates/talkbank-lsp/src/alignment/tier_hover/helpers.rs` now owns shared alignment-pair lookup, text-tier offset resolution, and `%mor`/`%gra` hover formatting helpers, so the tier-hover modules no longer each rebuild manual `AlignmentHoverInfo` shells or scan alignment pairs inline.
- `crates/talkbank-lsp/src/backend/features/code_action.rs` is now a thin composition root, with `code_action_fixes.rs` owning diagnostic-code routing and document-aware quick-fix logic and `code_action_builders.rs` owning the shared `WorkspaceEdit` / `CodeAction` construction path.
- `vscode/src/executableService.ts` now owns external executable discovery and invocation for the VS Code runtime (`talkbank`, `chatter` for `chatter lsp`, and `send2clan`), so extension features no longer import `child_process` directly to locate or launch those binaries.
- `vscode/src/validationExplorer.ts` now routes CLI execution and filesystem discovery through `vscode/src/validation/executor.ts` and `vscode/src/validation/fileFinder.ts`, leaving the explorer provider as the tree/UI/state adapter instead of one monolith that also owned process and traversal logic.
- `tests/test_utils/parser_suite.rs` now owns the shared root integration-test `ParserImpl` enum plus the standard two-parser constructor, so the root test suites no longer each redefine the same parser-wrapper and suite-construction boilerplate in their local `helpers.rs`.
- VS Code command registration now lives in per-feature registrar modules under `vscode/src/activation/commands/`, with `registerExtensionCommands()` acting as the activation-layer aggregation point.
- the TypeScript client and Rust execute-command boundary now validate command-name parity against a shared manifest checked by tests on both sides.
- the remaining text/document request routing in `talkbank-lsp` now flows through one explicit request-family module with shared document-resolution helpers and service objects instead of split thin wrapper modules.
- developer-facing boundary vocabulary is now documented in `docs/boundary-vocabulary.md` and the main book.

## LSP and VS Code

- The `batchalign3` architecture audit is now captured in `batchalign3/book/src/developer/architecture-audit.md`. Editor/server contract generation is no longer blocked on a missing audit note, but it should stay sequenced behind the active `batchalign3` control-plane and frontend follow-up refactors listed there.
- Keep the activation boundary under `vscode/src/activation/commands/` feature-oriented as new editor commands are added; prefer adding or refining registrar modules over regrowing one large activation file.
- The `talkbank/analyze` path now proves the schema-driven contract pattern end to end: Rust-owned transport types, generated schema, and matching TypeScript builder logic. If editor/server contract work needs to go further later, extend that pattern to other stable command families before jumping straight to full code generation.
- Keep extension-local panel traffic on the same path: `executeCommandPayloads.ts` owns typed editor/server payloads, `webviewMessageContracts.ts` owns typed extension/webview payloads, and panel classes should decode `unknown` once at those boundaries instead of matching raw command bags inline.
- Keep singleton panel mechanics on the same path too: `panelLifecycle.ts` should stay the shared helper for reuse/create/dispose plumbing so panel files can focus on HTML, state, and feature behavior.
- The planned Effect migration for the core VS Code extension path is now complete: boundary decoding, execute-command transport, command registration, feature command handlers, and the remaining async panel callbacks all run through Effect-native services and runners instead of a parallel promise-based path.
- Strategic TypeScript direction remains the same: keep new extension code on that Effect-native path rather than adding one-off helpers or ambient side effects back into commands and panels.
- Keep persistent extension command state on that same path too: if a command needs cross-invocation state, add a small Effect-provided store module and provide it from the command layer instead of introducing new module-level `let` state or exported mutable collections.
- If future follow-up work is needed, it should be optional second-order adoption (for example deeper Effect treatment of auxiliary services like cache/CLAN integration or other subsystems), not another required core transition pass.
- Keep external executable work on the same path too: runtime `talkbank`, `chatter`/`chatter lsp`, and `send2clan` lookup/launch logic should stay in `vscode/src/executableService.ts`, while legacy CLAN CLI process execution should stay test-only in `crates/talkbank-clan/tests/common/mod.rs`.
- Keep LSP code actions on the same path too: `backend/features/code_action.rs` should stay the composition root, `code_action_fixes.rs` should own per-diagnostic fix routing, and `code_action_builders.rs` should own shared edit/action assembly instead of regrowing one feature-sized quick-fix file.
- Keep LSP hover work on the same path too: `alignment/tier_hover/helpers.rs` should remain the shared lookup boundary for byte-offset resolution, alignment-pair joins, and `%mor`/`%gra` formatting so the tier-specific hover modules stay composition-focused.
- Keep validation-explorer work on the same path too: tree rendering/state should stay in `validationExplorer.ts`, CLI execution should stay in `validation/executor.ts`, and filesystem traversal should stay in `validation/fileFinder.ts` instead of drifting back into one tree-provider class.

## Tests

- Continue reducing test-only helper duplication in `talkbank-lsp` and `talkbank-clan` by exposing small internal test support modules where duplication still remains.
- Keep root integration tests on the same path too: shared parser-suite wiring should live in `tests/test_utils/parser_suite.rs`, with per-suite `helpers.rs` files only layering on domain-specific parse/assert helpers and local error mapping.
- Keep legacy CLAN binary execution test-only: `crates/talkbank-clan/tests/common/mod.rs` should remain the single owner of `CLAN_BIN_DIR` lookup, per-command availability checks, skip messaging, and process spawning for golden coverage.

## Parsers

- Keep direct-parser header work on the same path too: `header/mod.rs` should stay the composition root, `dispatch.rs` should own byte-prefix routing, `standalone.rs` should own API-compat parsing entrypoints, and `helpers.rs` should own shared recovery/reporting helpers instead of drifting back into one oversized module.

## Concurrency Audit

- Keep auditing remaining mutex usage across `talkbank-tools` and `batchalign3`, using `batchalign3/book/src/developer/architecture-audit.md` as the current cross-repo baseline for UI-thread sharing, background cache access, and worker coordination that should move to narrower ownership or message-passing patterns.

## Naming and Responsibility Audit

- After the remaining concurrency follow-ups from the current `batchalign3` audit, audit sink-style abstractions across the repo. Keep narrow one-way reporting interfaces like `ErrorSink`, but rename or split broader stateful objects that are really collectors, writers, or actor boundaries.
