# ADR-002: Effect-based command runtime

**Status:** Accepted
**Last updated:** 2026-04-16 22:01 EDT

## Context

VS Code's command API — `vscode.commands.registerCommand(id, handler)` —
takes any callback. Handlers that fail at runtime either `throw`
(surfacing as an unhandled rejection) or silently return `undefined`.
Features that compose multiple commands (e.g. "filter by speaker, then
run analysis on the filtered virtual document") end up stringing
`try/catch` blocks together.

The LSP client's RPC methods are async and fallible; each
`executeCommand` call can fail three distinct ways (transport, wrong
response shape, server error). Commands that call multiple RPCs need
to classify and recover from each kind without collapsing them into
generic `unknown` catches.

## Decision

**Wire every command through the Effect runtime** via
`registerEffectCommand(context, id, effect)` in
`src/effectCommandRuntime.ts`. Handlers return
`Effect.Effect<void, TypedError, RuntimeServices>` instead of
`Promise<void>`.

Typed error families at the boundaries:

- `ExecuteCommandStructuredError` — LSP RPC calls. Subtypes:
  `ExecuteCommandRequestError` (transport), `ExecuteCommandResponseError`
  (shape), `ExecuteCommandServerError` (server-side), plus
  `StructuredPayloadDecodeError` from `effectBoundary.ts`.
- `StructuredPayloadDecodeError` — any `unknown` payload crossing a
  typed boundary (webview messages, LSP responses).
- Per-feature error types: `CoderCommandError`, `TranscriptionError`, etc.

The runtime catches, classifies, logs, and user-visibly reports every
failure category. No `try/catch` in handlers; no
`unhandledPromiseRejection` surprises.

## Consequences

**Positive.**

- Every handler is a pure value until the runtime runs it. The same
  effect can be run in tests, composed into another effect, or
  retried without re-registering.
- Error classification is compositional. A command that calls
  `analyze` → then `filterDocument` → then opens a panel carries the
  union of all three error families in its type, and the runtime
  branches on `_tag` rather than instanceof chains.
- Webview message decoders share the same boundary: a
  `StructuredPayloadDecodeError` from any panel flows through the
  same reporter as an RPC response-shape mismatch.
- Tests use `Effect.runPromiseExit` to assert on the outcome value
  without catching exceptions.

**Negative.**

- Extra ceremony for trivial commands. "Show an info message" is one
  line with `vscode.window.showInformationMessage(...)` but three
  lines when wrapped as `Effect.sync(() => ...)`.
- Effect is a dependency. Developers unfamiliar with it need to learn
  `Effect.flatMap`, `Effect.catchTag`, and the tagged-error pattern
  from `effect/Data`.
- The runtime file itself (`effectCommandRuntime.ts`) is another
  layer between `registerCommand` and the handler, with its own
  test surface.

## Alternatives considered

**Direct `registerCommand` everywhere, `async`/`await` with
`try/catch`.** Rejected: the 41 command handlers would replicate
boilerplate for error classification, logging, and user notification.
A single bug in the boilerplate (forgotten `catch`, swallowed error
class) would hide across every command.

**Promise-based custom wrapper without Effect.** Rejected: most of
the value here is the typed-error algebra. A Promise wrapper gives
the boilerplate benefit but loses the compositional-error typing.
Effect already has the abstraction; writing a half-version in-house
would be its own maintenance burden.

**Tagged unions + a custom runtime.** Rejected as a smaller version
of "reinvent Effect." The effect library is stable, well-documented,
and tree-shakes cleanly.

## Source anchors

- Runtime: `src/effectCommandRuntime.ts`.
- Typed-error helpers: `src/effectBoundary.ts`.
- LSP error family: `src/lsp/executeCommandErrors.ts`.
- Panel decoders: `src/webviewMessageContracts.ts` +
  `src/webviewContracts/*`.
- Example handler: any file in `src/activation/commands/`.
