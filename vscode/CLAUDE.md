# VS Code Extension — TalkBank CHAT Editor

**Last modified:** 2026-04-13 11:14 EDT

## Overview

VS Code extension providing real-time validation, CLAN analysis, media playback,
transcription mode, waveform visualization, dependency graphs, and speaker filtering
for CHAT format files. Backed by the `talkbank-lsp` Rust binary over stdio.

## Project Structure

```
vscode/
├── src/
│   ├── extension.ts           # Entry point: LSP client, 20+ command registrations
│   ├── analysisPanel.ts       # Webview: CLAN analysis JSON → styled tables
│   ├── graphPanel.ts          # Webview: Graphviz DOT → SVG
│   ├── mediaPanel.ts          # Webview: audio/video playback with segment tracking
│   ├── waveformPanel.ts       # Webview: Web Audio API waveform visualization
│   ├── validationExplorer.ts  # Tree view: bulk directory validation
│   ├── cacheManager.ts        # Status bar: cache stats (polls chatter CLI)
│   ├── clanIntegration.ts     # Open in CLAN via send2clan FFI
│   └── utils/                 # alignmentSidecar, bulletParser (fallback), mediaResolver, speakerFilter, cliLocator
├── syntaxes/chat.tmLanguage.json  # TextMate grammar (fallback highlighting)
├── package.json               # Extension manifest (commands, keybindings, menus)
├── DEVELOPER.md               # Architecture guide (read this first)
├── GUIDE.md                   # User guide
└── CLAN-FEATURES.md           # Feature parity vs legacy CLAN app
```

## Key Commands

```bash
cd vscode && npm install && npm run compile
cd vscode && npm test          # vitest
cd vscode && npm run lint      # eslint
```

## Features & Shortcuts

| Feature | Shortcut | Description |
|---------|----------|-------------|
| Dependency Graph | Cmd+Shift+G | DOT → SVG via Graphviz WASM |
| Play at Cursor | Cmd+Shift+Enter | Single segment playback |
| Continuous Play | Cmd+Shift+/ | Play all segments from cursor |
| Rewind | F8 | Rewind 2 seconds |
| Loop Segment | F5 | Toggle loop |
| Walker | Alt+Down/Up | Step through utterances with playback |
| CLAN Analysis | Context menu | 33 commands → analysis panel |
| Transcription | Command palette | F4 to stamp timing bullets |
| Speaker Filter | Context menu | Virtual document by speaker |
| Waveform | Cmd+Shift+W | Web Audio waveform visualization |

## Architecture

```
VS Code Extension (TypeScript)   ← commands, webviews, tree views
        │ stdio
talkbank-lsp (Rust binary)       ← LSP protocol, caching, feature dispatch
        │
talkbank-model + parser          ← parsing, validation, alignment
```

## LSP Binary Discovery

`findLspBinary()` searches: system PATH (via `which`) → `target/debug/` → `target/release/`.

## Webview Pattern

All panels use singleton pattern (`createOrShow`). Communication via PostMessage JSON
protocol (rewind, setLoop, seekTo, segmentChanged).

## Analysis Commands (33 total)

All 33 CLAN commands are wired: freq, mlu, mlt, wdlen, wdsize, maxwd, freqpos, timedur,
gemlist, cooccur, dist, chip, phonfreq, modrep, vocd, codes, complexity, corelex, chains,
dss, eval, flucalc, ipsyn, kideval, sugar, trnfix, uniq + 6 requiring user input (kwal,
combo, keymap, mortable, script, rely).

## Dependencies

- `vscode-languageclient@9.0.1` — LSP client
- `@hpcc-js/wasm@2.29.0` — Graphviz WASM
- TypeScript 5.9, ESLint, Vitest

## Detailed Documentation

- `DEVELOPER.md` — architecture deep dive, state machines, adding features
- `GUIDE.md` — user guide with screenshots and workflows
- `CLAN-FEATURES.md` — feature parity assessment vs macOS CLAN app

---
Last Updated: 2026-04-12 06:56 EDT
