# CLAN Feature Parity

**Status:** Current
**Last updated:** 2026-04-16 22:16 EDT

This chapter summarizes the VS Code extension's feature parity with
the legacy CLAN macOS application. The extension does not just
replicate CLAN — it surpasses it in many areas while faithfully
implementing its core capabilities.

## Summary

The VS Code extension implements all major CLAN features and adds over 30 capabilities that CLAN never had. A small number of CLAN features are not applicable in the VS Code context (they are handled natively by VS Code or are platform-specific).

## Improvements Over CLAN

The following features exist in the VS Code extension but have no equivalent in the CLAN macOS application:

| Feature | Description |
|---------|-------------|
| **Cross-platform** | macOS, Windows, and Linux with identical functionality. CLAN only runs on macOS. |
| **Real-time diagnostics** | Errors appear as you type with inline squiggles. CLAN shows errors only after an explicit check. |
| **Corpus-scale validation** | Validate entire directory trees via the Validation Explorer with results cached in SQLite. CLAN validates one file at a time. |
| **Quick-fix code actions** | Automatic fixes for 21 error codes via `Cmd+.`. CLAN has no equivalent. |
| **Bidirectional cross-tier alignment** | Click any word on any tier to highlight aligned counterparts across all tiers simultaneously. CLAN's alignment display is one-directional and limited. |
| **Alignment mismatch inlay hints** | Inline annotations (`[alignment: 3 main <> 2 mor]`) visible without running any command. CLAN only surfaces these in the error log. |
| **Go to definition** | `F12` jumps from speaker codes to declarations, from dependent tiers to aligned main words. |
| **Find all references** | `Shift+F12` lists every occurrence of a speaker code across declarations, headers, and tiers. |
| **Rename speaker code** | `F2` renames a speaker across the entire file atomically. CLAN requires manual find-and-replace. |
| **Linked editing** | Editing a speaker code simultaneously updates all matching occurrences. |
| **Code lens (utterance counts)** | Per-speaker utterance counts above @Participants without running any analysis. |
| **Smart selection** | `Shift+Ctrl+Right/Left` expands/shrinks by syntactic units (word, utterance, tier block, transcript). |
| **Workspace symbol search** | `Cmd+T` searches across all open CHAT files by speaker and content. |
| **Document links** | `@Media:` values are clickable links. |
| **Speaker filtering** | Side-by-side filtered view of selected speakers only. |
| **Code folding** | Collapse utterance blocks and header blocks. |
| **Breadcrumb navigation** | `Cmd+Shift+O` structured outline of utterances by speaker. |
| **On-type formatting** | Continuation lines auto-indented with tabs. |
| **Diagnostic tags** | Empty utterances shown with fade-out styling. |
| **Snippet templates** | 8 tab-triggered CHAT structure templates. |
| **CSV export** | Analysis results exportable to CSV for spreadsheets and statistical software. |
| **Configurable severity** | Filter diagnostics to errors-only, errors+warnings, or all. |
| **Offline graph rendering** | Bundled Graphviz WASM, no internet needed. |
| **Scoped find** | Search within specific tiers and/or specific speakers, with regex. |
| **Context menu organization** | Analysis, media, and navigation in logical submenus. |
| **Proper data modeling** | All CHAT operations use parsed AST, not string hacking. |
| **Pull diagnostics** | LSP 3.17 pull model alongside push model. |
| **Waveform zoom/scroll** | 100-2000% zoom with pointer-centered mouse wheel. |
| **Playback speed control** | 0.25x to 2x via slider in media panel. |
| **Transcription key configuration** | Remap F4/F5/F8 for foot pedals via standard VS Code keybindings. |

## Features Fully Implemented from CLAN

These CLAN features have been faithfully ported to the VS Code extension:

| CLAN Feature | VS Code Implementation |
|-------------|----------------------|
| Syntax highlighting | TextMate grammar + LSP semantic tokens (11 token types) |
| Validation (CHECK) | Real-time diagnostics + Validation Explorer |
| Hover / Tier Window | `textDocument/hover` for main, %mor, %gra, %pho, %sin tiers |
| Alignment display | `textDocument/documentHighlight` with bidirectional range finders |
| Completion (auto-complete) | `textDocument/completion` for speakers, tiers, postcodes, headers, brackets |
| Dependency graph | `talkbank/showDependencyGraph` + Graphviz WASM rendering |
| 33 CLAN analysis commands | `talkbank/analyze` with generic result rendering |
| KidEval / Eval / Eval-D | Dedicated webview panels with language/activity grid, auto-detection from file headers, cascading database selection, and z-score comparison |
| Participant Editor | `idEditorPanel.ts` with 10-column editable table |
| Media playback (Sonic mode) | `mediaPanel.ts` with single and continuous play |
| Waveform view | `waveformPanel.ts` with Web Audio API rendering |
| Walker mode | `Alt+Down/Up` stepping with auto-play |
| Transcription mode | `F4` stamping with LSP-formatted bullets |
| Coding mode | `.cut` file loading, hierarchical QuickPick, `%cod` insertion |
| Picture display | `picturePanel.ts` with `%pic:` reference scanning |
| F5 loop / F8 rewind | Segment loop toggle and configurable rewind |
| Special character input | Compose-key mode for 30+ CA symbols and 20+ CHAT marks |
| Open in CLAN | Apple Events (macOS) and Windows messaging IPC |
| Format/tidy | `textDocument/formatting` through canonical serializer |
| Document outline | `textDocument/documentSymbol` two-level tree |
| Bullet display | Configurable: dim (35%), hidden, or normal |
| Corpus-level analysis | Directory-level analysis via explorer context menu |

## Features Not Applicable to VS Code

These CLAN features are handled natively by VS Code or are platform-specific and not relevant:

| CLAN Feature | VS Code Equivalent |
|-------------|-------------------|
| Go to line dialog | `Cmd+G` (built-in) |
| Print support | VS Code print extensions |
| Font selection | `editor.fontFamily` setting |
| Window management | VS Code's built-in layout system |
| Options/preferences panel | VS Code Settings UI |
| File input dialog | VS Code's native file picker |
| Commands window | Command Palette (`Cmd+Shift+P`) |
| About dialog | Extension info page |
| Services menu | macOS-specific, not relevant cross-platform |
| Edit mode toggle | VS Code's read-only mode |

## Related Chapters

- [Architecture](architecture.md) — how the extension implements these features
- [LSP Protocol](lsp-protocol.md) — the standard LSP capabilities powering these features
- [Command Catalog](../reference/commands.md) — every VS Code command this extension contributes
- [RPC Contracts](../reference/rpc-contracts.md) — the 12 custom `talkbank/*` endpoints
