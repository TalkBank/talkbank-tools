# CLAN macOS App Features: VS Code Port Assessment

_Last updated: 2026-03-06_

This document inventories the CLAN macOS GUI's user-facing capabilities, maps each
to an idiomatic VS Code equivalent, and records what has already been built.  It is
intended as a living reference for prioritising future extension work.

---

## What Is Already Done

The extension already covers more than it might appear.

### Language support (highlighting, formatting)

TextMate grammar (`syntaxes/chat.tmLanguage.json`) provides basic token
colouring, and the LSP backend adds a full semantic-tokens layer via the
`talkbank-highlight` crate.  The `textDocument/formatting` handler serialises
the open document back through the canonical CHAT serialiser — equivalent to
CLAN's "tidy" formatting pass.

### Hover (main, %mor, %gra, %pho, %sin)

`talkbank-lsp` handles `textDocument/hover` for all five tier types:

- **Main tier** — shows alignable word sequence with formatting.
- **%mor** — morphological breakdown aligned to main-tier words.
- **%gra** — grammatical relations with hierarchy context.
- **%pho** — phonetic transcription with alignment details.
- **%sin** — sign/gesture annotations with context.

CLAN shows similar information in its Tier Window pane.

### Alignment highlighting

Clicking any word on the main tier highlights the corresponding token(s) on
every dependent tier, and vice versa.  This maps to CLAN's interactive
alignment display.  Implemented via `textDocument/documentHighlight` with
bidirectional range finders for %mor, %mod, %sin, %gra, %pho.

### Inlay alignment hints

`textDocument/inlayHint` surfaces `[alignment: N main ↔ M mor]` inline when
counts differ.  There is no direct CLAN GUI equivalent; CLAN surfaces these
only in the error log.

### Quick-fix code actions ✅ DONE

`textDocument/codeAction` offers fixes for 21 error codes: E241 (`xx`→`xxx`),
E242 (trailing-off markers), E244 (consecutive stress markers),
E258/E259 (comma fixes), E301/E305 (missing terminators), E306/E322
(empty utterance/colon deletion), E308 (undeclared speakers), E312/E313
(unclosed bracket/paren), E323 (missing colon after speaker), E362
(timestamp swap), E501/E502/E503 (structural headers), E504 (missing
@Participants), E506/E507 (empty header templates), and E604 (orphaned
%gra).  Diagnostics use fade-out tags (unnecessary/deprecated) for visual
de-emphasis.  Equivalent to CLAN's inline suggestion system.

### Rename speaker codes ✅ DONE

`textDocument/rename` renames a speaker code across `@Participants`, `@ID`
headers, and all main tier lines in a single operation (`F2`).  No CLAN GUI
equivalent — CLAN requires manual find-and-replace.

### Find all references ✅ DONE

`textDocument/references` (`Shift+F12`) finds every occurrence of a speaker
code: `@Participants` declaration, `@ID` header, and all main tier lines.
No CLAN GUI equivalent.

### Code lens (utterance counts) ✅ DONE

`textDocument/codeLens` shows utterance counts per speaker above the
`@Participants` header (e.g., `CHI: 42 utterances`).  Provides an at-a-glance
summary without running an analysis command.  No direct CLAN equivalent.

### CHAT snippets ✅ DONE

8 VS Code snippets (`snippets/chat.json`) for common CHAT structures: file
header block, `@Participants`, `@ID`, main tier, `%mor`, `%gra`, `@Comment`,
and Gem (`@Bg`/`@Eg`).  Equivalent to CLAN's template insertion features.

### Completions (speakers, tier codes, postcodes)

`textDocument/completion` suggests speaker codes drawn from `@Participants`,
valid dependent-tier prefixes (mor, gra, pho, mod, sin, …), and standard
postcode punctuation suffixes.  Matches the auto-complete behaviour CLAN
offers in its editor.

### Go-to definition

`textDocument/definition` navigates from a speaker code to its `@Participants`
declaration, and from a dependent-tier line to the main tier it annotates.
No CLAN GUI equivalent — this is a native VS Code strength.

### %gra dependency graph

`talkbank.showDependencyGraph` (keybinding `Cmd+Shift+G`) opens a WebviewPanel
beside the editor showing the dependency tree for the utterance at the cursor,
rendered via Graphviz WASM.  Equivalent to CLAN's Dependency Graph window.

### Validation explorer + diagnostics

The `talkbank-validation-explorer` tree view runs the `chatter validate` CLI,
caches results per file in SQLite, and displays errors as VS Code diagnostics
(squiggles + Problems panel).  Substantially richer than CLAN's single-file
"Check" pass because it operates at corpus scale.

### Media playback (single bullet and continuous)

`talkbank.playBullet` (`Cmd+Shift+Enter`) plays the segment nearest the cursor.
`talkbank.playContinuous` (`Cmd+Shift+/`) plays from the cursor to EOF, moving
the editor selection to track the currently-playing utterance line.  Both
commands resolve `@Media:` locally and prefer language-server alignment sidecar
timings, falling back to direct `•beg_end•` parsing when sidecar data is unavailable. Equivalent to CLAN's Sonic/Waveform viewer play
controls (minus the waveform itself).

### Document symbols (outline view) ✅ DONE

`textDocument/documentSymbol` returns a two-level tree: the transcript as a
`Module` symbol containing per-utterance `String` symbols labelled by speaker
code.  Users navigate via `Cmd+Shift+O`, the breadcrumb bar, and the Outline
view.  Equivalent to CLAN's scrollable utterance index.

### Code folding for dependent tiers ✅ DONE

`textDocument/foldingRange` folds each utterance block (`*SPEAKER:` line +
associated `%xxx:` dependent tiers) as one unit.  The header block
(`@Begin` … first utterance) folds separately.  Annotators can hide annotation
layers to focus on speech text.

### F5/F8 loop and rewind ✅ DONE

`talkbank.rewindMedia` (`F8`) posts `{ command: 'rewind', seconds: N }` to the
active `MediaPanel`; the webview sets `media.currentTime -= N`.
`talkbank.loopSegment` (`F5`) toggles looping of the current segment — the
webview re-seeks to the segment start on each `timeupdate` while the loop flag
is set.  Default rewind amount is configurable via `talkbank.rewind.seconds`.

### Walker mode ✅ DONE

`talkbank.walkerNext` (`Alt+Down`) and `talkbank.walkerPrev` (`Alt+Up`) step
through `•beg_end•` bullets one at a time.  Each step moves the editor cursor to
the corresponding utterance line and, if a `@Media:` header is present, plays
the segment via `MediaPanel`.  Walker resets automatically when the active
document changes.

### CLAN analysis commands ✅ DONE

`talkbank.runAnalysis` opens a QuickPick listing all 33 CLAN commands with
one-line descriptions.  The selected command is executed via an LSP
`workspace/executeCommand` request (`talkbank/analyze`) and the JSON result is
rendered in `AnalysisPanel` — a WebviewPanel that displays section headings,
key-value fields, and tables generically.  No per-command renderer needed.
No external CLI binary is required — analysis runs inside the LSP server.

Commands available (33): freq, mlu, mlt, wdlen, wdsize, maxwd, freqpos,
timedur, kwal, combo, gemlist, cooccur, dist, chip, phonfreq, modrep, vocd,
codes, complexity, corelex, chains, dss, eval, flucalc, ipsyn, keymap,
kideval, mortable, rely, script, sugar, trnfix, uniq.

Commands that require additional input (kwal, combo, keymap, mortable, script,
rely) prompt the user via InputBox or file picker dialogs before execution.

### KidEval / Eval panels ✅ DONE

`talkbank.runKideval` and `talkbank.runEval` open dedicated WebviewPanels for
normative database comparison.  The user selects a language and database from
dropdowns populated by the LSP (`talkbank/kidevalDatabases` and
`talkbank/evalDatabases` commands), optionally filters by age range and gender,
and runs the analysis.  Results display z-scores comparing the child's
performance against normative means, plus detailed metric tables.  These panels
go beyond CLAN's command-line `kideval`/`eval` by providing an interactive GUI
with filtering and visual presentation.

### @ID / Participant editor ✅ DONE

`talkbank.editParticipants` opens a WebviewPanel showing all `@ID` lines in
a tabular form with editable fields for the 10 pipe-delimited `@ID` components
(language, corpus, speaker, age, sex, group, SES, role, education, custom).
Parsing and serialization are fully delegated to the LSP via
`talkbank/getParticipants` and `talkbank/formatIdLine` execute commands —
TypeScript is a thin UI layer.  Saving writes canonical `@ID` lines back to
the document.  Equivalent to CLAN's Participant Editor dialog.

### CSV export ✅ DONE

Both `AnalysisPanel` and `KidevalPanel` include an "Export CSV" button that
collects all visible tables and stat cards, formats them as CSV, and opens a
save-file dialog.  This replaces CLAN's "Save Output" text export with a more
structured format suitable for spreadsheet analysis.

### Corpus-level analysis ✅ DONE

`talkbank.runAnalysisOnDirectory` (available via explorer context menu on
folders) runs any CLAN analysis command across all `.cha` files in a directory.
The LSP's `resolve_files()` walks the directory tree; results are aggregated
in a single `AnalysisPanel`.  This matches CLAN's batch-mode directory
analysis capability.

### Transcription mode ✅ DONE

`talkbank.startTranscription` opens `MediaPanel` in transcription mode (no
auto-advance; plays from cursor position).  `talkbank.stampBullet` requests the
current timestamp from the webview via a `requestTimestamp` / `timestamp`
message round-trip, then delegates bullet and utterance line formatting to the
LSP via `talkbank/formatBulletLine`.  The formatted bullet is inserted at the
end of the current line, and a new `*SPEAKER:\t` line opens below.  Speaker code
is taken from `talkbank.transcription.defaultSpeaker` (default: `"CHI"`).
`talkbank.stopTranscription` stops playback and resets state.

### Speaker filtering ✅ DONE

`talkbank.filterBySpeaker` presents a multi-select QuickPick of declared
speakers, then opens a filtered read-only virtual document in a new editor
column.  Both speaker extraction and document filtering are delegated to
the LSP via `talkbank/getSpeakers` and `talkbank/filterDocument` commands —
no CHAT parsing in TypeScript.  The filtered view includes all file-level
headers plus only the utterance blocks whose speaker code is in the selection.
The virtual document is still processed by the LSP (`chat` language ID).

### Waveform / sonic view ✅ DONE

`talkbank.showWaveform` (`Cmd+Shift+W`) opens `WaveformPanel` — a WebviewPanel
that decodes the linked media file via the Web Audio API, renders a canvas
waveform (peak amplitude per pixel column), and overlays coloured rectangles
for each `•beg_end•` bullet segment.  Clicking the waveform seeks both the
media and the editor cursor to the nearest utterance.  The panel is coordinated
with `MediaPanel`: playing a segment highlights it in the waveform via
`highlightSegment` messages, and waveform clicks drive `MediaPanel` via
`seekTo` messages.

### Selection range (smart selection) ✅ DONE

`textDocument/selectionRange` expands the selection by syntactic units:
word → utterance content → tier block → transcript.  Triggered via
`Shift+Ctrl+Right` (expand) / `Shift+Ctrl+Left` (shrink).  No CLAN
equivalent — this is a native VS Code LSP feature.

### Linked editing for speaker codes ✅ DONE

`textDocument/linkedEditingRange` keeps matching speaker codes in sync as
you type.  Editing a speaker code on a main tier line simultaneously updates
the corresponding occurrences.  No CLAN equivalent.

### On-type formatting (auto-tab) ✅ DONE

`textDocument/onTypeFormatting` automatically inserts a leading tab on
continuation lines, keeping dependent tiers properly indented without manual
intervention.

### Workspace symbols ✅ DONE

`workspace/symbol` (`Cmd+T`) enables cross-file search by speaker code and
utterance content across all open CHAT files.  No CLAN equivalent.

### Document links ✅ DONE

`textDocument/documentLink` makes `@Media:` header values clickable links
that open the referenced media file.  No CLAN equivalent.

### Header and bullet hovers ✅ DONE

`textDocument/hover` now also covers headers (`@Participants`, `@ID`, etc.)
and timing bullets with contextual documentation and alignment details.

### Header and bracket completion ✅ DONE

`textDocument/completion` triggers on `@` for header names and `[` for
bracket annotations, in addition to the existing speaker code and tier
prefix completions.

### Semantically scoped find ✅ DONE

`talkbank.scopedFind` (context menu: "Find in Tier…") searches within
specific CHAT tiers (main, %mor, %gra, %pho, %sin, %act, %cod, %com,
%exp, or all) and optionally filters by speaker.  Supports plain text
and regex (prefix query with `/`).  Uses the parsed `ChatFile` model to
determine tier boundaries, then searches raw text within those spans —
no ad-hoc string parsing.  Results appear in a QuickPick with navigation.
CLAN has no equivalent — users must use generic find-and-replace without
tier awareness.

### Pull diagnostics (LSP 3.17) ✅ DONE

`textDocument/diagnostic` and `workspace/diagnostic` pull model.  The
server caches last-published diagnostics and serves them on demand,
supporting both per-document and workspace-wide queries.  Complements
the existing push model (`publishDiagnostics`).

### Extended quick fixes ✅ DONE

`textDocument/codeAction` now handles 21 error codes (was 7): E241, E242,
E244, E258, E259, E301, E305, E306, E308, E312, E313, E322, E323, E362,
E501, E502, E503, E504, E506, E507, E604.  Fixes include timestamp swap
repair, unclosed bracket/paren insertion, empty header templates, empty
utterance deletion, consecutive comma cleanup, and missing speaker/colon
insertion.

---

## Improvements Over the CLAN macOS Application

The VS Code extension doesn't just replicate CLAN — it surpasses it in many areas.
These features have no equivalent in the original macOS application.

### Cross-Platform

CLAN only runs on macOS. The VS Code extension runs on **macOS, Windows, and Linux**
with identical functionality. Researchers on any platform can use the same tools.

### Better Editor

VS Code is a world-class text editor. CLAN's built-in editor lacks basic features
that VS Code provides out of the box: multi-cursor editing, regex find/replace,
split panes, configurable themes, keyboard shortcut customization, an extension
ecosystem, and integrated terminal. On top of this foundation, the TalkBank
extension adds CHAT-specific intelligence.

### Corpus-Scale Validation

CLAN validates one file at a time. The VS Code extension validates entire directory
trees via the Validation Explorer, with results cached in SQLite for instant
re-display. Researchers working with large corpora (thousands of files) can see
all errors across their dataset at once.

### Real-Time Diagnostics

CLAN shows errors only when you explicitly run a check. The VS Code extension
validates continuously as you type, showing errors and warnings as inline squiggles
and in the Problems panel — the same experience developers expect from modern IDEs.

### Quick-Fix Code Actions

The extension offers automatic fixes for 21 error codes: replacing `xx` with `xxx`,
adding missing terminators, inserting trailing-off markers, adding undeclared
speakers, swapping reversed timestamps, deleting empty utterances, fixing
consecutive commas, and more. Press `Cmd+.` on any diagnostic to see available
fixes. CLAN has no equivalent — users must fix errors manually.

### Bidirectional Cross-Tier Alignment

Click any word on any tier to highlight its aligned counterparts across all other
tiers simultaneously. CLAN's alignment display is one-directional and limited to
specific tier pairs. The extension highlights across main, `%mor`, `%gra`, `%pho`,
and `%sin` tiers in both directions.

### Alignment Mismatch Inlay Hints

Inline annotations (e.g., `[alignment: 3 main ↔ 2 mor]`) appear when tier counts
don't match. These are visible without running any command. CLAN only surfaces
alignment mismatches in the error log after an explicit check.

### Go to Definition

`F12` or `Cmd+Click` jumps from a speaker code to its `@Participants` declaration,
or from a `%mor`/`%gra` item to the aligned main tier word. Standard IDE navigation
that CLAN never offered.

### Find All References

`Shift+F12` on a speaker code lists every occurrence across declarations, `@ID`
headers, and all main tier lines. Invaluable for understanding how a participant
is used throughout a transcript.

### Rename Speaker Code

`F2` renames a speaker code across `@Participants`, `@ID` headers, and all main
tier lines in one atomic operation. CLAN requires error-prone manual find-and-replace.

### Linked Editing

Editing a speaker code on a main tier line simultaneously updates all matching
occurrences. No copy-paste errors.

### Code Lens (Utterance Counts)

Utterance counts per speaker appear above the `@Participants` header (e.g.,
`CHI: 42 utterances`) without running any analysis command.

### Smart Selection

`Shift+Ctrl+Right/Left` expands or shrinks selection by syntactic units: word →
utterance → tier block → transcript. Understands CHAT structure, not just lines.

### Workspace Symbol Search

`Cmd+T` searches across all open CHAT files by speaker code and utterance content.
Cross-file navigation that CLAN has no equivalent for.

### Document Links

`@Media:` header values are clickable links that open the referenced media file
directly. No need to navigate to the file manually.

### Speaker Filtering

Filter a transcript to show only selected speakers in a side-by-side panel. Useful
for focusing on a single participant's contributions. CLAN has no equivalent.

### Code Folding

Collapse utterance blocks (main tier + dependent tiers) or the header block to
focus on specific parts of a transcript. Standard editor feature that CLAN lacks.

### Breadcrumb Navigation and Outline View

`Cmd+Shift+O` shows a structured outline of all utterances by speaker. The
breadcrumb bar shows where you are in the document structure. CLAN's scrollable
index is far less capable.

### On-Type Formatting

Continuation lines are automatically indented with tabs as you type, keeping
dependent tiers properly aligned without manual intervention.

### Diagnostic Tags

Empty utterances and empty colons are visually de-emphasized with fade-out
styling, distinguishing minor issues from real errors at a glance.

### Snippet Templates

8 CHAT snippets for rapid annotation entry: header blocks, `@Participants`,
`@ID`, main tier, `%mor`, `%gra`, `@Comment`, and Gem markers. Type a prefix
and press Tab.

### CSV Export

Analysis results can be exported to CSV for use in spreadsheets and statistical
software. CLAN's "Save Output" produces plain text that requires manual parsing.

### Configurable Validation Severity

Filter diagnostics to show only errors, errors and warnings, or everything.
CLAN shows all messages with no filtering.

### Offline Operation

Dependency graph rendering uses bundled Graphviz WASM — no internet connection
required. The extension is fully self-contained.

### Semantically Scoped Find

Search within a specific CHAT tier (main, %mor, %gra, %pho, etc.) and optionally
filter by speaker. Supports plain text and regex. CLAN's find-and-replace operates
on raw text with no awareness of CHAT structure — it cannot limit searches to a
specific tier or speaker.

### Context Menu Organization

Analysis commands, media controls, and navigation features are organized into
logical submenus. CLAN's interface has no comparable organization.

### Proper Data Modeling

All CHAT document operations are delegated to the Rust LSP server, which works
with the parsed AST (`ChatFile` model) rather than ad-hoc regex/string parsing.
Speaker extraction, document filtering, utterance scanning, and bullet formatting
all go through LSP commands (`talkbank/getSpeakers`, `talkbank/filterDocument`,
`talkbank/getUtterances`, `talkbank/formatBulletLine`). The TypeScript extension
is a thin presentation layer with no CHAT parsing logic. CLAN's Objective-C++
code mixes parsing and UI throughout.

---

## Features Assessed but Not Ported

### CHAT-aware find / replace ✅ DONE

CLAN's Find dialog understands tier scoping (search only within %mor, only
within main tiers, etc.).  Implemented as `talkbank.scopedFind` ("Find in
Tier…") — see the "Semantically scoped find" entry above.

---

### Special character input ✅ DONE

`talkbank.composeCA` (`Cmd+Shift+1`) and `talkbank.composeChat` (`Cmd+Shift+2`)
enter a compose-key mode.  The next keystroke is mapped to the corresponding
Unicode codepoint (30+ CA symbols and 20+ non-CA marks).  Ported from
OSX-CLAN's `CharToSpChar()` function.  A status bar indicator shows the active
mode, and pressing `Escape` cancels.  Implemented in `specialChars.ts`.

### Playback speed control ✅ DONE

The media panel toolbar includes a speed slider (0.25x–2x) using the HTML5
`playbackRate` API.  The slider shows the current rate label and defaults to 1x.

### Evald (dementia assessment) ✅ DONE

`talkbank.runEvald` opens the shared KidEval/Eval panel in `evald` mode,
using the `eval-d` LSP command with `EvalVariant::Dialect` for
DementiaBank normative database comparison.  Available via right-click context
menu and Command Palette.

---

## Remaining OSX-CLAN Features Not Yet Ported

Identified by source code audit of `OSX-CLAN/` (2026-03-06).  Grouped by
priority based on user value for the target audience (linguistic researchers).

### Medium value

- **Walker configuration**: ✅ DONE — CLAN's `WalkerController` settings are
  now available as VS Code settings: `walker.autoPlay`, `walker.loopCount`,
  `walker.pauseSeconds`, `walker.walkLength`, `media.defaultSpeed`.  Only
  "backspace amount" is not ported (our rewind uses `transcription.rewindSeconds`).

### Lower priority

- **Picture display**: ✅ DONE — `talkbank.showPicture` finds and displays
  associated images: scans `%pic:` references, same-name image files, and
  directory contents.  Multiple images show a picker.  Available in the
  TalkBank: Media context menu.

- **Coding mode**: ✅ DONE — `talkbank.startCoder` loads a `.cut` codes
  file, then steps through uncoded utterances.  User selects codes from
  a hierarchical QuickPick; selected code is inserted as `%cod:\tcode`.
  `Cmd+Enter` advances to the next uncoded utterance, `Cmd+Shift+C`
  inserts a code without advancing.  Progress shown in the picker
  placeholder.  Utterance detection and coded-status checking are
  delegated to the LSP via `talkbank/getUtterances`.

- **Bullet display**: ✅ DONE — The `talkbank.bullets.display` setting
  controls bullet visibility: `"dim"` (35% opacity, default), `"hidden"`
  (invisible), or `"normal"` (full visibility).  CLAN's "expand bullets"
  mode (inline timestamp display) is not ported; hover over bullets to see
  timing details instead.

### Not applicable to VS Code

These CLAN features are handled natively by VS Code or are platform-specific:

- **Go to line dialog** — `Cmd+G`
- **Print support** — VS Code print extensions
- **Font selection** — VS Code settings `editor.fontFamily`
- **Window management** — VS Code's built-in layout system
- **Options/preferences panel** — VS Code Settings UI
- **File input dialog** — VS Code's native file picker
- **Commands window** — replaced by QuickPick and Command Palette
- **About dialog** — VS Code's extension info page
- **Services menu** — macOS-specific; not relevant cross-platform
- **Edit mode toggle** — VS Code's read-only mode

---

### Waveform zoom / scroll ✅ DONE

The waveform panel now includes a toolbar with zoom in/out buttons, a zoom
slider (100%–2000%), and a fit-to-window button.  Mouse wheel zooms centered
on the pointer position.  When zoomed in, the canvas extends beyond the
viewport and the container scrolls horizontally.  During playback, the view
auto-scrolls to keep the highlighted segment visible.

### Offline Graphviz ✅ DONE

The dependency graph panel now loads `@hpcc-js/wasm` from the bundled
`node_modules` directory via a webview-safe URI instead of fetching from the
jsdelivr CDN.  The extension works fully offline — no internet connection
required for graph rendering.  The Google Fonts CDN link for JetBrains Mono
was also removed (falls back to system monospace).

### Transcription keybinding configuration ✅ DONE

`talkbank.configureTranscriptionKeys` opens the VS Code keybindings editor
pre-filtered to TalkBank transcription commands (stamp, rewind, loop).  Users
can remap F4/F5/F8 to foot-pedal key codes or any other preferred keys using
VS Code's native keybinding system — no custom settings UI needed.

---

*Last Updated: 2026-03-06*
