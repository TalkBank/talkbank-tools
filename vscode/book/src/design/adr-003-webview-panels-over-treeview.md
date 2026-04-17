# ADR-003: Webview panels over TreeView

**Status:** Accepted
**Last updated:** 2026-04-16 22:02 EDT

## Context

The extension surfaces five rich auxiliary views — analysis results,
dependency graph, media playback, waveform, KidEval assessment — and
two simpler views (ID editor, picture viewer). VS Code offers three
mechanisms:

1. **TreeView** (`vscode.window.createTreeView`) — native side-bar
   tree with VS Code's built-in styling. Tight integration with
   context-menu contributions; rigid rendering (items with icons and
   labels).
2. **Custom editor** (`vscode.window.registerCustomEditorProvider`) —
   replaces the text editor with a fully custom UI. Suitable for
   non-text formats.
3. **Webview panel** (`vscode.window.createWebviewPanel`) — an
   iframe-like HTML surface inside VS Code, with a PostMessage-based
   JSON bridge to the extension host.

## Decision

**Use webview panels** for all interactive auxiliary views. One
singleton panel per view type (`mediaPanel`, `graphPanel`, etc.),
`createOrShow` pattern for reuse, disposal hooks to tear down
resources when the panel closes.

Native TreeView is used only for the **Validation Explorer** (file-
system-keyed tree of validation results) where the native chrome is
the right affordance.

Webview message contracts are typed per panel — see
[webview-contracts.md](../reference/webview-contracts.md). Each
panel's message union is an Effect `Schema`-decoded discriminated
union; panel controllers translate decode failures into a
`PanelErrorMessage` sent back to the webview.

## Consequences

**Positive.**

- Full control over rendering: Graphviz DOT (rendered with
  `@hpcc-js/wasm`), HTML5 `<audio>`/`<video>`, canvas-based waveforms,
  CSV tables with sortable columns — none of which fit TreeView.
- Panels can carry state the extension host doesn't need to know
  about (e.g. the current waveform scroll position lives in the
  webview's JS).
- The same HTML/JS assets can render in a future non-VS-Code host
  (planned for the Tauri desktop app) with only the PostMessage
  transport swapped.

**Negative.**

- Webview content-security-policy handling, asset-URI resolution, and
  the `vscode-webview-ui-toolkit` dependency all become the
  extension's problem.
- Every panel carries its own test surface and lifecycle bugs (e.g.
  the 2026-04-16 fix for a status-bar item never disposed when the
  review panel closed — see [KIB-005](../developer/known-issues-and-backlog.md#kib-005)).
- Webview → extension messages cross a serialization boundary. The
  `effectBoundary.ts` decoders make the boundary typed, but every
  new message shape needs an Effect `Schema` plus a decoder test.

## Alternatives considered

**TreeView + custom hover card.** Rejected for the dependency graph
and waveform specifically: neither renders in TreeView's item
slots. The media panel *could* live as a TreeView of segments but
would lose inline playback controls.

**Custom editor.** Rejected: a `.cha` file is first and foremost a
text document. Users want to edit it in the normal text editor with
full VS Code affordances (LSP diagnostics, search, git), not in a
replacement editor. The auxiliary views augment the text editor
rather than replace it.

**Multiple windows or an external Electron app.** Rejected: all the
workflows coupled to transcription, walking, coder mode — require
in-editor interaction. Splitting them into a separate app would
break the cursor-position contract (e.g. "play media at cursor").

## Source anchors

- Panel controllers: `src/<panel>Panel.ts` (7 files).
- Shared lifecycle: `src/panelHost.ts`, `src/panelAssets.ts`, `src/panelLifecycle.ts`.
- Webview HTML/JS: `src/webview/<panel>Panel.{html,js}` (5 pairs).
- Message contracts: `src/webviewMessageContracts.ts` (historical umbrella)
  + `src/webviewContracts/<panel>PanelContract.ts` (preferred home for
  new panels, per [KIB-006](../developer/known-issues-and-backlog.md#kib-006)).
- Validation explorer (only TreeView): `src/activation/validation.ts`.
