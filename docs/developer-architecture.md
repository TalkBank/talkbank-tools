# Developer Architecture

**Status:** Current
**Last updated:** 2026-03-16

This note records the main architectural seams after the current round of refactors. It is aimed at contributors working across `talkbank-cli`, `talkbank-lsp`, the VS Code extension, and the test/dashboard tooling.

For naming rules across seams like sinks, reporters, coordinators, renderers, and services, see [boundary-vocabulary.md](/Users/chen/talkbank/talkbank-tools/docs/boundary-vocabulary.md).
For current struct-shape guidance, see the book page `book/src/architecture/wide-structs.md` and the audit test `crates/talkbank-cli/tests/wide_struct_audit.rs`.
For the current cross-repo concurrency baseline that originally gated some `talkbank-tools` cleanup follow-ups, see `batchalign3/book/src/developer/architecture-audit.md`. That audit is now captured; the remaining question is sequencing against the active `batchalign3` refactor fronts listed there.

## Extension and LSP command boundary

The VS Code extension now routes editor-facing `workspace/executeCommand` requests through `vscode/src/lsp/executeCommandClient.ts`. That file owns command names and transport-level request dispatch. Structured payload contracts now live beside it in `vscode/src/lsp/executeCommandPayloads.ts`, which owns the TypeScript shapes and Effect `Schema`-based runtime decoders for the structured responses the extension consumes.

Between them, those modules own the editor-side boundary for:

- dependency graphs
- alignment sidecars
- speaker and participant queries
- scoped search
- timing bullet formatting
- analysis and database discovery

On the Rust side, `crates/talkbank-lsp/src/backend/execute_commands.rs` and `crates/talkbank-lsp/src/backend/contracts.rs` are the matching decode boundary. Together they own:

- the advertised execute-command names used in `initialize`
- typed request structs for each command family
- the public transport-shaped JSON contract for `talkbank/analyze`
- the conversion from raw JSON argument vectors into those typed requests

The important rule is: do not add new editor/server commands by hand in scattered feature modules. Add the command name to `executeCommandClient.ts`, add any structured request or response shape to the corresponding boundary module (`executeCommandPayloads.ts` on the TypeScript side and `execute_commands.rs` / `contracts.rs` on the Rust side), then wire feature-specific logic behind them.

Request routing now uses feature-oriented service objects instead of one match-heavy handler. `crates/talkbank-lsp/src/backend/requests/execute_command.rs` is the composition root, and document-local graph/alignment commands, analysis commands, participant commands, and chat-ops commands each route through their own service object. When adding a new execute-command handler, extend the appropriate service instead of growing the central request router.

Command-name parity is also checked mechanically. `vscode/src/test/fixtures/executeCommandContract.json` is the shared manifest for execute-command identifiers, and both the TypeScript tests and the Rust `execute_commands.rs` unit tests assert that their boundary modules match it. When adding or renaming an execute-command, update the TypeScript boundary, the Rust boundary, and the shared manifest together.

Structured-response validation is now explicit too. `executeCommandClient.ts` no longer returns bare promises; it now exposes Effect-returning command operations plus tagged transport/response/server errors, and it decodes known array/object payloads through the helpers in `executeCommandPayloads.ts` before feature modules consume them. The alignment-sidecar playback path reuses that same contract instead of validating a second ad hoc shape. `vscode/src/effectBoundary.ts` now owns the shared Effect `Schema` decode helpers and tagged boundary errors used by that path, so TypeScript-side transport validation no longer depends on one-off hand-written shape readers. If a future command returns a structured payload that the extension depends on, add a decoder and tests there rather than casting it directly in the feature module.

That same rule now applies to database discovery. `discoverDatabases()` returns validated `AvailableDatabase[]` values from the execute-command boundary, and `kidevalPanel.ts` forwards those through a narrower typed panel-message contract instead of one optional-field message bag. If another panel depends on a stable editor/server payload, tighten that boundary in the same place rather than decoding it inside the webview or panel body.

That same narrowing step now exists for extension-local panel traffic too. `vscode/src/webviewMessageContracts.ts` owns the remaining shared webview message unions, Effect `Schema` decoders, and outbound builder helpers for the analysis, ID editor, media, waveform, and KidEval/Eval panels. The panel adapters (`analysisPanel.ts`, `idEditorPanel.ts`, `mediaPanel.ts`, `waveformPanel.ts`, and `kidevalPanel.ts`) now receive webview payloads as `unknown`, decode them once at the panel boundary, and then work with typed messages internally. The async panels that still do meaningful work (`analysisPanel.ts`, `idEditorPanel.ts`, and `kidevalPanel.ts`) now run those message handlers through the shared Effect runner instead of spawning ad hoc async callbacks. Command callers such as `commands/transcription.ts`, `commands/waveform.ts`, and `activation/commands/media.ts` now build outbound panel messages through the same contract module instead of posting ad hoc `{ command: string }` objects directly. The regression coverage for that seam lives in `vscode/src/test/webviewMessageContracts.test.ts`. If a future panel needs structured bidirectional messages, extend that module rather than reintroducing panel-local message bags.

The singleton webview lifecycle is now explicit as well. `vscode/src/panelLifecycle.ts` owns the shared "reuse existing or create new", invalid-message logging, and disposal-draining mechanics used by the current panel classes. It now treats malformed inbound panel payloads as the tagged `PanelMessageDecodeError` boundary case coming from `effectBoundary.ts`, while still preserving the previous behavior of logging and dropping bad webview messages instead of crashing a panel. `analysisPanel.ts`, `idEditorPanel.ts`, `mediaPanel.ts`, `waveformPanel.ts`, `kidevalPanel.ts`, `graphPanel.ts`, and `picturePanel.ts` still own feature behavior and HTML generation, but they no longer each reimplement the same singleton-panel control flow inline. If a new panel is added, start from those helpers instead of copying another `currentPanel` / `createOrShow()` / `_dispose()` loop.

The analysis request path now follows the same rule on the request side. `talkbank/analyze` no longer relies on a positional JSON tuple or a Rust-side `Map<String, Value>` bag. The logical request payload is now the single-object `AnalyzeCommandPayload` contract in `crates/talkbank-lsp/src/backend/contracts.rs`, reused directly by the backend decoder and exported for schema generation. `analysis.rs` translates that transport-shaped payload into library-owned `talkbank-clan::service::AnalysisOptions`, while the TypeScript side builds the same object shape through `buildAnalyzeCommandRequest(...)`. If a new analysis option is added, update those boundary types first and then thread the new field into the command implementation.

That public analyze contract is now schema-owned. `AnalyzeCommandPayload`, `AnalysisOptionsPayload`, and `AnalysisDatabaseFilterPayload` derive `JsonSchema`, and `schema/analyze-command.schema.json` is generated from those Rust-owned types. This is the current source of truth for the `talkbank/analyze` editor/server wire shape: one object inside the LSP argument vector with `commandName`, `targetUri`, and `options`.

## Validation explorer boundary

`vscode/src/validationExplorer.ts` is now only the tree/UI/state adapter for the bulk validation explorer. It delegates CLI-facing work to `vscode/src/validation/executor.ts` and filesystem discovery to `vscode/src/validation/fileFinder.ts`.

That means:

- `validationExplorer.ts` should own tree item composition, progress UI, refresh signaling, and validation-result state
- `validation/executor.ts` should own `talkbank validate --json` / `talkbank cache clear` command strings, output parsing, and binary discovery
- `validation/fileFinder.ts` should own directory listing policy, recursive `.cha` discovery, hidden-directory skipping, and sort order

If the explorer grows later, extend one of those three seams rather than adding more CLI process logic or filesystem traversal back into the tree provider.

## VS Code external executable boundary

`vscode/src/executableService.ts` now owns the direct external executable boundary for the VS Code extension runtime. That service is responsible for:

- locating `talkbank` in local `target/{debug,release}` builds or on `PATH`
- locating `chatter` from user configuration, `PATH`, or local build outputs for `chatter lsp`
- locating `send2clan` from `PATH` or local build outputs
- running `talkbank` CLI commands and detached `send2clan` launches through the only runtime `child_process` boundary in the extension

That means:

- `validation/executor.ts`, `cacheManager.ts`, `models/cacheStatistics.ts`, `activation/lsp.ts`, and `clanIntegration.ts` should call `ExecutableService` instead of importing `child_process` directly
- `utils/cliLocator.ts` and `utils/lspLocator.ts` may stay as thin compatibility facades, but they should delegate to `ExecutableService` rather than regrowing their own process-spawn logic
- if another extension feature needs an external binary later, add that discovery/invocation path to `ExecutableService` first so runtime dependencies remain documentable in one place

The current dependency picture is now explicit:

- legacy CLAN CLI binaries are still test-only, owned by `crates/talkbank-clan/tests/common/mod.rs`
- VS Code runtime binaries are `talkbank`, `chatter` (for `chatter lsp`), and optional `send2clan`, all owned by `vscode/src/executableService.ts`
- remaining Rust-side external command execution in this repo is build/test-only (`crates/talkbank-cli/build.rs` for `git describe`, `crates/talkbank-cli/tests/cache_tests.rs` for self-hosted CLI integration)

## Root integration test parser-suite boundary

`tests/test_utils/parser_suite.rs` now owns the shared parser-wrapper boundary for root integration tests. That module provides the common `ParserImpl` enum and the standard tree-sitter + direct parser suite constructor, while individual `tests/*/helpers.rs` files keep only their suite-specific parse/assert helpers and local error-shape mapping.

That means:

- shared parser construction belongs in `tests/test_utils/parser_suite.rs`, not in each test suite's local `helpers.rs`
- per-suite helpers may still add domain-specific `impl ParserImpl` blocks (for example header parsing or error-corpus collection), but they should not reintroduce another local parser enum or another direct/tree-sitter constructor pair
- if another root integration suite needs the standard two-parser coverage, import it from `crate::test_utils::parser_suite` first and add only the domain-specific layer locally

The repo now also keeps one shared fixture for that contract at `vscode/src/test/fixtures/analyzeCommandPayload.json`. The VS Code tests reuse it when checking `buildAnalyzeCommandRequest(...)`, and `tests/validate_analyze_command_fixture.rs` validates the same fixture against the generated schema and deserializes it through the public Rust contract. That keeps one concrete cross-language example under mechanical verification instead of trusting hand-kept mirror examples.

## CLAN library boundary

`talkbank-clan` should be treated as the library-first home for reusable CLAN execution primitives, not just as a bag of command implementations behind CLI wrappers. The current cleanup now keeps three shared analysis seams there:

- `UtteranceRange` in `crates/talkbank-clan/src/framework/filter.rs` is the typed range model for CLAN-style utterance selection
- `DiscoveredChatFiles` in `crates/talkbank-clan/src/framework/input.rs` owns recursive CHAT-file discovery for file- and directory-based analysis targets
- `CommandOutput::to_json_value()` in `crates/talkbank-clan/src/framework/output.rs` gives programmatic consumers structured output without rendering JSON text and reparsing it

On top of those primitives, `crates/talkbank-clan/src/service.rs` now owns the higher-level CLAN analysis execution boundary. `AnalysisCommandName` is the typed identifier for supported CLAN analyses, `AnalysisRequest` is the typed library request surface for named analyses, `AnalysisService` owns JSON and rendered execution over `AnalysisRunner`, and `AnalysisOptions` plus `AnalysisRequestBuilder` now own the raw-option-to-request translation step for outer adapters. That means default values like `corelex` frequency thresholds, `chains`/`keymap` tier fallbacks, `trnfix` tier defaults, and `sugar` minimum-utterance thresholds now come from library-owned configs instead of being open-coded in the LSP adapter. `talkbank-lsp/src/backend/analysis.rs` and the CLI CLAN adapters now translate their outer request shapes into that service boundary instead of importing and executing most command types directly. Keep special cases explicit, but prefer extending the shared service before adding another command-construction hub in an outer crate.

That split is intentional. `talkbank-cli` and `talkbank-lsp` should convert their outer request shapes into these library-owned models, but they should not reintroduce ad hoc range parsing, duplicate directory walking, or JSON string round-trips in their own helper layers. Keep strings at the outer boundary for final text/CSV/CLAN rendering; keep typed models and direct serialization inside the reusable CLAN framework.

Legacy CLAN binary execution remains intentionally outside that runtime library surface. The only remaining CLAN CLI process spawning in this repo lives in `crates/talkbank-clan/tests/common/mod.rs`, which owns `CLAN_BIN_DIR` lookup, per-command availability checks, skip messaging, and tempdir-backed process execution for golden/parity coverage. Production code should continue to route through Rust library APIs, not through CLAN subprocesses.

## VS Code activation boundary

The VS Code extension now keeps command registration under `vscode/src/activation/commands/`. `index.ts` is the aggregation point for command registration, while feature registrars such as `utility.ts`, `media.ts`, `analysis.ts`, and `editor.ts` own the actual `vscode.commands.registerCommand(...)` calls. Shared runtime dependencies flow through `ExtensionServices` in `types.ts`.

That aggregate service bag is now also the bridge into the new Effect foundation. `vscode/src/effectRuntime.ts` defines the initial `Context` tags, one shared activation `Layer`, reusable layer runners, and a tagged `AsyncOperationError` wrapper for promise-returning boundary work. `vscode/src/effectCommandRuntime.ts` builds the command/runtime layer on top of that foundation by adding VS Code host services plus the shared `ExtensionCommandRunner` / `registerEffectCommand(...)` path, `vscode/src/mediaCommandState.ts` now holds the persistent walker/transcription state stores that used to live as module-level mutable globals, `vscode/src/coderState.ts` now does the same for coder-mode session state, and `vscode/src/textFileService.ts` is the shared async text-file boundary for command-side file reads such as `.cut` loading.

That means:

- `extension.ts` should stay a composition root that creates services and calls one activation entrypoint
- new commands should land in the registrar module that owns the feature rather than growing one shared activation file
- feature command handlers should return `Effect`s and be registered through `registerEffectCommand(...)` instead of wiring promise-returning callbacks directly
- shared command dependencies should flow through `ExtensionServices` plus the Effect-provided VS Code host services rather than ad hoc imports into `extension.ts`
- future Effect-native commands and panels should receive their runtime dependencies through `effectRuntime.ts` / `effectCommandRuntime.ts` rather than importing ambient singletons directly
- persistent command state should live in Effect-provided store modules such as `mediaCommandState.ts` and `coderState.ts`, not in module-level `let` variables or exported mutable maps
- command-side file reads should go through `textFileService.ts` rather than synchronous `fs.readFileSync(...)` calls embedded in command handlers

## LSP language services

`talkbank-lsp` no longer stores a single parser or semantic-tokens provider behind backend-wide mutexes. `crates/talkbank-lsp/src/backend/language_services.rs` now owns thread-local access to those resources, and `Backend` treats them as a service boundary rather than as shared mutable fields.

That means:

- request helpers reparse on cache miss via `LanguageServices`
- semantic-token requests go through the same service boundary
- diagnostics orchestration no longer needs to lock global parser state before doing incremental work

If a future LSP feature needs parser access, route it through the language-services layer instead of adding another shared lock to `Backend`.

## LSP text/document request boundary

`crates/talkbank-lsp/src/backend/requests/text_document.rs` now owns the broader text/document request family for hover, completion, code actions, inlay hints, references, rename, goto definition, document highlights, selection ranges, linked editing, on-type formatting, and document links. That module uses a small service composition root plus shared document-resolution helper structs so the request layer no longer open-codes the same `document_text` / `get_parse_tree` / `get_chat_file` sequences across several thin wrapper modules.

When adding another request in that family, extend `text_document.rs` or the service objects it contains instead of reintroducing separate wrapper modules for each subset of handlers.

## LSP code action boundary

`crates/talkbank-lsp/src/backend/features/code_action.rs` is now the composition root for diagnostic-driven quick fixes. It delegates per-diagnostic selection and document-aware fix derivation to `code_action_fixes.rs`, while `code_action_builders.rs` owns the shared `WorkspaceEdit` / `CodeAction` construction path used by those fixes.

That means:

- future quick fixes should be routed from `code_action_fixes.rs` by diagnostic code instead of extending one large `match` plus inlined edit-building in the feature entrypoint
- repeated `WorkspaceEdit` / single-edit `CodeAction` assembly should go through `code_action_builders.rs` rather than rebuilding the same struct shells in each fix helper
- `code_action.rs` should stay focused on iterating diagnostics and collecting actions, not on parsing diagnostic messages or constructing edit payloads inline

## LSP alignment hover boundary

`crates/talkbank-lsp/src/alignment/tier_hover/helpers.rs` now owns the shared lookup boundary for alignment hover. That module resolves byte offsets for structured and text-only tiers, joins source/target indices across alignment pair lists, and formats the shared `%mor`/`%gra` dependency summary used by multiple hover cards.

That means:

- tier-specific hover modules should compose hover cards from helper lookups plus tier-local formatting instead of rescanning alignment pairs inline
- hover modules should build cards through `AlignmentHoverInfo::new(...)` and the existing builder methods instead of reinitializing the full struct shape by hand
- pair lookups should go through the helper functions rather than assuming pair-list row position is the same thing as a source or target index

## CLI command boundary

`talkbank-cli` top-level dispatch now flows through `crates/talkbank-cli/src/commands/dispatch.rs`. `cli/run.rs` is only the composition root: it resolves TUI/logging/theme concerns, then hands the parsed `Commands` enum to a small family router. That router currently owns four feature services:

- validation commands
- utility/format-conversion commands
- cache maintenance commands
- CLAN commands

The important rule is: do not regrow the big top-level `match` in `cli/run.rs`. When adding a new top-level CLI command, extend the owning family service in `commands/dispatch.rs` or add a new family there if the command truly introduces a new cross-cutting boundary.

The validate family now also has an explicit typed command boundary in `crates/talkbank-cli/src/commands/validate/mod.rs`. `ValidateCommandOptions` owns grouped rule/execution/presentation settings before the code branches into either the single-file or directory runtime. That keeps the command-shape translation in one place instead of rebuilding it inline in `run.rs`.

Cache maintenance now follows the same pattern too: `crates/talkbank-cli/src/commands/cache/mod.rs` owns the `run_cache_command(...)` family entrypoint instead of relying on another nested `match` in the CLI runtime.

## CLI validation runtime

`talkbank-cli` validation output is no longer one loop with a large `match` on output mode. `crates/talkbank-cli/src/commands/validate_parallel/renderer.rs` now defines the renderer boundary and `runtime.rs` is responsible for orchestration, cancellation, and stats collection.

That means:

- text output policy belongs in renderer implementations
- JSONL output policy belongs in renderer implementations
- runtime state transitions belong in `runtime.rs`

If a future TUI or progress renderer is added, it should implement the renderer interface rather than extend one shared branch-heavy loop.

Audit mode now follows the same ownership rule. `crates/talkbank-cli/src/commands/validate/audit_reporter.rs` owns a dedicated writer thread and returns a cloneable worker handle. `AuditReporter` is responsible for output lifecycle and final summary assembly, and `finish()` is the only place that joins the writer thread and returns summary stats.

Validation cache setup is now explicit too. `crates/talkbank-cli/src/commands/validate/cache.rs` owns cache initialization, `--force` prefix clearing, cached validation lookups, and cache-write warning behavior. Single-file validate, directory validation, audit mode, and watch-triggered validation now all go through that same helper boundary instead of mixing one-off `UnifiedCache::new()` setup in the file path with a separate directory-only cache helper.

## Model error boundaries

`talkbank-model` no longer keeps every error-reporting concept in one `sink.rs` file. The current split is:

- `crates/talkbank-model/src/errors/error_sink.rs` for the `ErrorSink` trait plus the lightweight forwarding implementations
- `crates/talkbank-model/src/errors/collectors.rs` for in-memory collectors and counters like `ErrorCollector` and `ParseTracker`
- `crates/talkbank-model/src/errors/async_channel_sink.rs` for the async channel-backed sink used by Tokio-facing integrations
- `crates/talkbank-model/src/errors/configurable_sink.rs`, `offset_adjusting_sink.rs`, and `tee_sink.rs` for adapters that transform, filter, or duplicate diagnostics before forwarding
- `crates/talkbank-model/src/validation/async_runtime.rs` for the async validation orchestration entry points that run CPU-bound validation on Tokio blocking threads

That layout is intentional. The repo keeps the `ErrorSink` concept, but it now separates interface, collectors, and adapters so the code structure matches the boundary vocabulary.

## Model content query boundary

`talkbank-model::MainTier` now owns `find_context_dependent_ca_omission_span()`. The tree-sitter parser and the direct parser both call that model-level query instead of each carrying a private recursive walker for CA-omission / shortening detection.

That means:

- parser crates should ask the model content layer semantically meaningful questions instead of cloning another content traversal when the question is really about `MainTier`
- if another parser, validator, or downstream tool needs the same omission/span query, extend the model query surface rather than adding another parser-local helper

## Direct parser header boundary

`crates/talkbank-direct-parser/src/header/mod.rs` is now the header entrypoint rather than the place that also owns byte-prefix dispatch, shared parse-error reporting, and the API-compat standalone parsers. Those responsibilities now split as:

- `dispatch.rs` for byte-prefix routing into the simple/complex header parsers
- `helpers.rs` for shared malformed-header recovery text plus the reusable chumsky-error reporting helpers
- `standalone.rs` for the public `parse_id_header_standalone(...)` and `parse_participant_entry_standalone(...)` compatibility surface

That means:

- future header-type additions should extend `dispatch.rs` and the relevant simple/complex parser module instead of growing `header/mod.rs`
- shared parse-error reporting should go through the helper functions instead of repeating the same `E501` construction in each entrypoint
- `header/mod.rs` should stay focused on the external `ParseOutcome` boundary and malformed-header recovery policy

## Dashboard threading model

`src/bin/test-dashboard.rs` is now a thin entrypoint over the library-backed `src/test_dashboard/` module set. The UI thread owns `AppState`. The worker owns corpus/file execution and emits `DashboardEvent` values over a channel. `src/test_dashboard/manifest.rs` is the persistence boundary for manifest mutation and checkpoint saves.

That split is intentional:

- UI rendering should not lock shared mutable state
- manifest persistence should stay behind `DashboardManifest` instead of living inline in the worker loop
- reducer logic should stay testable as plain state transitions
- file-level failure summary formatting should stay with the typed `FileTestOutcome` model instead of being rebuilt ad hoc in event emission

When extending the dashboard, prefer adding a new event plus reducer logic over reintroducing shared mutex state.

## CLAN golden test manifests

`crates/talkbank-clan/tests/clan_golden.rs` is now the composition root for the CLAN golden/parity harness. `harness.rs` owns shared CLAN/Rust execution helpers plus the generated test macros. `baseline.rs`, `check.rs`, and `variants_*` now declare manifest-style `ParityCase` values instead of repeating whole test bodies, while `rust_only.rs` keeps the bespoke temp-file scenarios explicit and uses `RustSnapshotCase` only for the plain snapshot cases. `crates/talkbank-clan/tests/common/mod.rs` now owns the shared corpus-root, fixture-path, CLAN-binary discovery, and tempdir-backed CLAN process helpers reused across the golden, converter, and transform suites. `crates/talkbank-clan/tests/converter_golden.rs` now follows the same ownership rule at a smaller scale: shared runner functions own fixture/corpus loading plus CLAN skip behavior, and generated case declarations keep the repetitive converter coverage declarative while preserving the explicit snapshot names. `crates/talkbank-clan/tests/transform_golden.rs` now does the same for its parity-style transform cases: a small runner boundary owns CLAN-vs-Rust setup plus the stdin/file-argument distinction, while the more specialized temp-file and rust-only transform coverage stays explicit.

Snapshot ownership is explicit too:

- committed baselines live in `crates/talkbank-clan/tests/clan_golden/snapshots/*.snap`
- failed local runs may create `*.snap.new` review artifacts; review or delete those locally, but do not commit them as baselines
- when adding a new golden case, add it to the manifest-style case list and commit the reviewed `.snap` file that represents the intentional baseline

## Roundtrip test harness

`tests/roundtrip_corpus/` now follows the same coordinator-owned-state rule. Worker threads only produce per-file `RoundtripEvent::FileComplete` values. `runner.rs` owns the aggregate `RoundtripStats` and updates them as it forwards results to the external event stream.

That means:

- worker threads should stay focused on parsing, cache access, and per-file status creation
- aggregate pass/fail/cache counters belong in `RoundtripStats`, updated by the coordinator
- future roundtrip progress or cancellation reporting should travel as explicit events rather than shared mutable test state

## Current follow-ups

- use `batchalign3/book/src/developer/architecture-audit.md` as the current cross-repo baseline for the remaining mutex/concurrency work
- keep the wide-struct audit current, especially for dashboard state and LSP backend state
- after the remaining concurrency follow-ups called out in that `batchalign3` audit, audit sink-style abstractions and rename or split any that are really collectors, writers, or actors instead of narrow one-way sinks
- extend the current schema-owned editor/server contract approach beyond `talkbank/analyze` only when another command family becomes stable enough that schema drift is a real maintenance cost
