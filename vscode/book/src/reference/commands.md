# Command Catalog

**Status:** Current
**Last updated:** 2026-04-16 21:56 EDT

Every user-invocable VS Code command this extension contributes,
grouped by feature area. Commands are declared in `package.json`'s
`contributes.commands`; registrations live in
`src/activation/commands/` (the 41 effect commands) and
`src/activation/validation.ts` (the 4 validation-explorer commands).

Total contributed commands: **45**.

Command IDs all use the `talkbank.` prefix; the corresponding LSP RPC
endpoints (12 of them) use the `talkbank/` prefix instead — they are
not directly user-invocable. See [RPC Contracts](rpc-contracts.md) for
those.

## Editor features

| Command | Title | Handler |
|---------|-------|---------|
| `talkbank.showDependencyGraph` | Show Dependency Graph | `commands/editor.ts` → `graphPanel.ts` |
| `talkbank.showWaveform` | Show Waveform View | `commands/editor.ts` → `waveformPanel.ts` |
| `talkbank.showPicture` | Show Elicitation Picture | `commands/editor.ts` → `picturePanel.ts` |
| `talkbank.editParticipants` | Edit Participants… | `commands/editor.ts` → `idEditorPanel.ts` |
| `talkbank.openInClan` | Open in CLAN | `commands/editor.ts` → `clanIntegration.ts` |

## Media playback

| Command | Title | Keybinding | Handler |
|---------|-------|------------|---------|
| `talkbank.playBullet` | Play Media at Cursor | `Cmd+Shift+Enter` | `commands/media.ts` |
| `talkbank.playContinuous` | Play Media Continuously | `Cmd+Shift+/` | `commands/media.ts` |
| `talkbank.stopPlayback` | Stop Media Playback | | `commands/media.ts` |
| `talkbank.rewindMedia` | Rewind Media (2 s) | `F8` | `commands/media.ts` |
| `talkbank.loopSegment` | Toggle Segment Loop | `F5` | `commands/media.ts` |

## Walker mode

| Command | Title | Keybinding | Handler |
|---------|-------|------------|---------|
| `talkbank.walkerNext` | Walker: Next Utterance | `Alt+Down` | `commands/media.ts` |
| `talkbank.walkerPrev` | Walker: Previous Utterance | `Alt+Up` | `commands/media.ts` |

## Transcription mode

| Command | Title | Keybinding | Handler |
|---------|-------|------------|---------|
| `talkbank.startTranscription` | Start Transcription Mode | | `commands/media.ts` |
| `talkbank.stampBullet` | Stamp Timestamp Bullet | `F4` | `commands/media.ts` |
| `talkbank.stopTranscription` | Stop Transcription Mode | | `commands/media.ts` |
| `talkbank.configureTranscriptionKeys` | Configure Transcription Keybindings… | | `commands/utility.ts` |

## CLAN analysis

| Command | Title | Handler |
|---------|-------|---------|
| `talkbank.runAnalysis` | Run CLAN Analysis… | `commands/analysis.ts` → `analysisPanel.ts` |
| `talkbank.runAnalysisOnDirectory` | Run CLAN Analysis on Directory… | `commands/analysis.ts` |
| `talkbank.runKideval` | Run KidEval… | `commands/analysis.ts` → `kidevalPanel.ts` |
| `talkbank.runEval` | Run Eval… | `commands/analysis.ts` → `kidevalPanel.ts` |
| `talkbank.runEvald` | Run Eval-D (Dementia)… | `commands/analysis.ts` → `kidevalPanel.ts` |

## Navigation

| Command | Title | Handler |
|---------|-------|---------|
| `talkbank.filterBySpeaker` | Filter by Speaker… | `commands/utility.ts` |
| `talkbank.scopedFind` | Find in Tier… | `commands/utility.ts` |

## Coder mode

| Command | Title | Handler |
|---------|-------|---------|
| `talkbank.startCoder` | Start Coder Mode… | `commands/utility.ts` → `coderPanel.ts` |
| `talkbank.stopCoder` | Stop Coder Mode | `commands/utility.ts` |
| `talkbank.coderNext` | Coder: Next Utterance | `commands/utility.ts` |
| `talkbank.coderInsertCode` | Coder: Insert Code… | `commands/utility.ts` |

## Review mode

| Command | Title | Handler |
|---------|-------|---------|
| `talkbank.startReview` | Start Review Mode | `commands/utility.ts` |
| `talkbank.stopReview` | Stop Review Mode | `commands/utility.ts` |
| `talkbank.reviewNext` | Review: Next Flagged | `commands/utility.ts` |
| `talkbank.reviewPrev` | Review: Previous Flagged | `commands/utility.ts` |
| `talkbank.reviewGood` | Review: Rate Good | `commands/utility.ts` |
| `talkbank.reviewEarly` | Review: Rate Early | `commands/utility.ts` |
| `talkbank.reviewLate` | Review: Rate Late | `commands/utility.ts` |
| `talkbank.reviewWrong` | Review: Rate Wrong | `commands/utility.ts` |
| `talkbank.reviewSkip` | Review: Skip | `commands/utility.ts` |

## Special characters

| Command | Title | Handler |
|---------|-------|---------|
| `talkbank.composeCA` | Insert CA Special Character… | `commands/editor.ts` |
| `talkbank.composeChat` | Insert CHAT Special Character… | `commands/editor.ts` |
| `talkbank.cancelCompose` | Cancel Special Character Input | `commands/editor.ts` |

## Validation explorer

Registered directly via `vscode.commands.registerCommand` in
`src/activation/validation.ts` rather than the effect runtime — the
validation explorer tree view owns its own command surface because
the tree view's data model drives context-menu contributions.

| Command | Title | Handler |
|---------|-------|---------|
| `talkbank.validateFile` | Validate File | `validation.ts` |
| `talkbank.validateDirectory` | Validate Directory | `validation.ts` |
| `talkbank.refreshValidation` | Refresh Validation | `validation.ts` |
| `talkbank.clearCache` | Clear Cache | `validation.ts` |

## Cache management

| Command | Title | Handler |
|---------|-------|---------|
| `talkbank.viewCacheStatistics` | View Cache Statistics | `commands/utility.ts` |
| `talkbank.clearAllCache` | Clear All Validation Cache | `commands/utility.ts` |

## Registration patterns

All 41 non-validation commands register through
`registerEffectCommand(context, name, effect)` in
`src/activation/commands/*.ts`. The effect runtime wraps each handler
so failures become typed `Effect` errors rather than unhandled promise
rejections, and the same effect value can compose with other features'
effects without ad-hoc `try/catch` orchestration. See
[ADR-002](../design/adr-002-effect-runtime.md) for the rationale.

A **command-registration sanity test** (`src/activation/commands/__tests__/`)
walks every `contributes.commands` entry and asserts one of two
things:

1. The command name is in the effect-registration table, or
2. The command name is in the validation-explorer allow-list.

Adding a command to `package.json` without registering it is
therefore a compile-or-test failure, not a silent runtime miss — the
outcome [KIB-C009](../developer/known-issues-and-backlog.md#kib-c009)
was filed to create.
