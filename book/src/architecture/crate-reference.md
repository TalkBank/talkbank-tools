# Crate Reference

**Status:** Current
**Last updated:** 2026-03-14

Summary of every crate in the `talkbank-tools` workspace.

## Core Crates

### talkbank-model

The typed data model for CHAT files. Defines `ChatFile`, `Utterance`, `DependentTier`, `MorTier`, `GraTier`, and all other AST types. Includes validation logic, the `WriteChat` trait for CHAT serialization, serde support for JSON, and `JsonSchema` derivations. Also owns error types (`ParseError`, `ErrorSink` trait, `Span`, `SourceLocation`), diagnostic infrastructure, the `ChatParser` trait abstraction, and `ParseValidateOptions`. Provides a closure-based content walker (`for_each_leaf` / `for_each_leaf_mut`) that centralizes recursive traversal of `UtteranceContent` and `BracketedItem` with domain-aware group gating.

### talkbank-derive

Procedural macros for the model crate (`SemanticEq`, `SemanticDiff`, `SpanShift`, `ValidationTagged`, and the `error_code_enum` macro).

### talkbank-parser

The canonical parser. Wraps the tree-sitter C parser and converts the concrete syntax tree (CST) into `ChatFile` model types. Provides error recovery via tree-sitter's GLR algorithm. Used by the LSP and CLI.

### talkbank-direct-parser

The experimental parser using chumsky combinators. Fail-fast design for batch processing of well-formed input. Must produce identical results to the tree-sitter parser on the reference corpus.

### talkbank-transform

High-level pipelines: parse+validate, CHAT-to-JSON, JSON-to-CHAT, normalization. Integrates the validation cache, JSON schema validation, and parallel directory validation.

## Application Crates

### talkbank-clan

CLAN analysis commands (FREQ, MLU, etc.), transforms (FLO, etc.), and format converters. Each command implements the `CommandOutput` trait with typed results. The crate now also owns higher-level library integration seams such as `UtteranceRange`, `DiscoveredChatFiles`, `service::AnalysisCommandName`, `service::AnalysisService` / `service::AnalysisRequest`, and the `service::AnalysisOptions` / `service::AnalysisRequestBuilder` layer so other crates can execute CLAN analyses without reimplementing directory walking, range parsing, raw command-name parsing, JSON output policy, or command-default selection.

### talkbank-cli

The `chatter` CLI binary: validate, normalize, to-json, CLAN command dispatch, and corpus management.

### talkbank-lsp

Language Server Protocol server with tree-sitter incremental parsing, real-time diagnostics, and semantic highlighting. The crate now also exposes `backend::contracts` as the Rust-owned transport contract module for stable editor/server payloads such as `AnalyzeCommandPayload`; that module drives the checked-in `schema/analyze-command.schema.json` artifact. The matching TypeScript-side contract/runtime layers live in `vscode/src/lsp/executeCommandPayloads.ts` for editor/server payload decoding, `vscode/src/webviewMessageContracts.ts` for extension/webview panel messages, `vscode/src/panelLifecycle.ts` for shared singleton panel plumbing, `vscode/src/effectBoundary.ts` for shared Effect `Schema` decoding and tagged boundary errors, `vscode/src/effectRuntime.ts` for the base Effect `Layer` / `Context` foundation, `vscode/src/effectCommandRuntime.ts` for the command/panel runner and VS Code host-service layer, and `vscode/src/mediaCommandState.ts` for persistent Effect-provided media-command state, so the LSP-facing extension seams no longer depend on scattered raw JSON bags, duplicated panel control flow, hand-written boundary decoders, a parallel promise-only command runtime, or module-level mutable command state.

### send2clan-sys

FFI bindings for sending files to the CLAN application (macOS Apple Events, Windows WM_APP).

## Test Crates

### talkbank-parser-tests

Parser equivalence tests. Runs both parsers on each file in the reference corpus and compares results (each `.cha` file is its own `#[test]` via rstest, nextest-compatible). Also runs spec-generated tests, roundtrip tests, and property tests.
