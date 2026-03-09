# TalkBank CHAT for VS Code — User Guide

**Status:** Current
**Last updated:** 2026-03-16

A comprehensive language extension for editing and validating CHAT transcription files (`.cha`) inside Visual Studio Code. The extension provides real-time validation, cross-tier alignment visualization, dependency graph rendering, and deep IDE integration with the CHAT format used by TalkBank, CHILDES, and related corpora.

---

## Table of Contents

1. [Why VS Code over CLAN?](#why-vs-code-over-clan)
2. [Installation](#installation)
3. [Syntax Highlighting](#syntax-highlighting)
4. [Real-Time Validation](#real-time-validation)
5. [Hover: Cross-Tier Alignment](#hover-cross-tier-alignment)
6. [Document Highlighting: Visual Alignment](#document-highlighting-visual-alignment)
7. [Dependency Graph Visualization](#dependency-graph-visualization)
8. [Code Completion](#code-completion)
9. [Snippets](#snippets)
10. [Quick Fixes](#quick-fixes)
11. [Rename](#rename)
12. [Linked Editing](#linked-editing)
13. [Find All References](#find-all-references)
14. [Code Lens](#code-lens)
15. [Document Symbols](#document-symbols)
16. [Inlay Hints](#inlay-hints)
17. [Go to Definition](#go-to-definition)
18. [Document Formatting](#document-formatting)
19. [CLAN Analysis Commands](#clan-analysis-commands)
20. [KidEval / Eval Assessment](#kideval--eval-assessment)
21. [Participant Editor](#participant-editor)
22. [Media Playback](#media-playback)
23. [Waveform View](#waveform-view)
24. [Picture Display](#picture-display)
25. [Coder Mode](#coder-mode)
26. [Transcription Mode](#transcription-mode)
27. [Walker Mode](#walker-mode)
28. [Speaker Filtering](#speaker-filtering)
29. [Validation Explorer](#validation-explorer)
30. [Cache Management](#cache-management)
31. [CLAN Integration](#clan-integration)
32. [Special Character Input](#special-character-input)
33. [Settings](#settings)
34. [Keyboard Shortcuts](#keyboard-shortcuts)
35. [Command Reference](#command-reference)
36. [Troubleshooting](#troubleshooting)

---

## Why VS Code over CLAN?

This extension replaces the macOS-only CLAN application with a modern, cross-platform
editing environment. Everything CLAN does — analysis commands, media playback,
transcription, waveform visualization, coding mode — works inside VS Code.

But the extension also adds capabilities that CLAN never had:

- **Real-time validation** — errors appear as you type, not after running a check
- **Quick fixes** — automatic corrections for common errors (`Cmd+.`)
- **Corpus-scale validation** — check entire directory trees, not one file at a time
- **Cross-tier alignment** — click any word to highlight aligned items across all tiers
- **Alignment mismatch hints** — inline warnings when tier counts disagree
- **Go to definition** — jump from speaker codes to declarations, tiers to main words
- **Find all references** — locate every occurrence of a speaker code
- **Rename** — change a speaker code everywhere in one operation
- **Linked editing** — typing a speaker code updates all matching occurrences live
- **Code lens** — utterance counts per speaker shown above `@Participants`
- **Smart selection** — expand/shrink selection by syntactic units
- **Workspace search** — search across all open CHAT files by speaker or content
- **Document links** — `@Media:` values are clickable links
- **Speaker filtering** — view only selected speakers in a side panel
- **CSV export** — export analysis results for spreadsheets and statistical software
- **Code folding** — collapse utterance blocks to focus on specific sections
- **Snippets** — tab-triggered templates for common CHAT structures
- **Cross-platform** — macOS, Windows, and Linux with identical functionality

For the full comparison, see [CLAN Feature Parity](CLAN-FEATURES.md#improvements-over-the-clan-macos-application).

---

## Installation

### Prerequisites

- VS Code 1.85 or later
- The `talkbank-lsp` binary (the language server that powers the extension)

### Building from Source

```bash
# From the repository root (talkbank-tools/):

# 1. Build the language server
cargo build --release -p talkbank-lsp

# 2. Install extension dependencies
cd vscode
npm install
npm run compile

# 3. Launch VS Code with the extension loaded
code --extensionDevelopmentPath=.
```

The extension activates automatically when you open any `.cha` file. It searches for the `chatter` binary on your system PATH first, then falls back to `target/debug/` or `target/release/` relative to the project root and launches the server with `chatter lsp`.

---

## Syntax Highlighting

The extension provides full syntax highlighting for CHAT format via a TextMate grammar. Elements are colored by category:

| Element | Examples | Highlighting |
|---------|----------|--------------|
| **Required headers** | `@UTF8`, `@Begin`, `@End` | Bold keyword |
| **Metadata headers** | `@Participants`, `@ID`, `@Languages` | Keyword |
| **General headers** | `@Date`, `@Location`, custom headers | Keyword |
| **Speaker codes** | `*CHI:`, `*MOT:`, `*INV:` | Variable (distinct color) |
| **Dependent tier prefixes** | `%mor:`, `%gra:`, `%pho:` | Function/tag |
| **Utterance terminators** | `.` `?` `!` `+/.` `+...` | Punctuation |
| **Annotations** | `[= text]`, `[: alt]`, `[*]`, `[+ post]` | String/operator |
| **Scoped groups** | `<word word>` | Bracket highlighting |
| **Retrace markers** | `[/]`, `[//]`, `[//?]` | Operator |
| **Actions & events** | `&=laughs`, `&Claps` | Tag |
| **Pauses** | `(0.5)`, `(.)` | Number |
| **Omissions** | `0word` | Special |
| **Morphology** | `n\|cookie`, `v\|go-PROG` | POS/stem/affix coloring |
| **Grammar relations** | `1\|2\|DET`, `3\|0\|ROOT` | Index/relation coloring |
| **Comments** | `%com:` lines | Comment |

The highlighting also provides semantic tokens from the language server, which offer more precise coloring than the TextMate grammar alone — for example, distinguishing specific POS tags or error-coded tokens.

---

## Real-Time Validation

As you edit a `.cha` file, the language server continuously parses and validates the document. Errors and warnings appear as squiggly underlines in the editor and in the **Problems** panel (`Cmd+Shift+M` / `Ctrl+Shift+M`).

### What Gets Validated

- **Syntax errors**: Malformed headers, missing tabs after speaker codes, unrecognized constructs
- **Required structure**: `@UTF8`, `@Begin`, `@End` headers present and in order
- **Participant consistency**: Speaker codes on main tiers match `@Participants` declarations
- **Tier alignment**: Word counts on `%mor` and `%gra` tiers match the main tier
- **Morphology format**: `%mor` items follow `POS|stem` structure
- **Grammar relations**: `%gra` indices reference valid positions, no circular dependencies
- **Timing bullets**: Timestamp format and ordering
- **Terminators**: Every utterance ends with a valid terminator

Each diagnostic includes:

- **Error code** (e.g., E301, E714) for lookup and filtering
- **Severity** — error (red), warning (yellow), or info (blue)
- **Related information** — pointers to other locations in the file that provide context (e.g., the main tier line when a `%mor` alignment error is reported)

### Diagnostic Tags

Certain diagnostics use VS Code's fade-out styling to indicate unnecessary or removable content. Empty utterances (E306) and empty colons (E322) appear with dimmed text in the editor, visually suggesting that the content can be safely removed.

---

## Hover: Cross-Tier Alignment

One of the most powerful features. Hover over any word or tier item to see how it aligns with elements on other tiers.

### Hovering on a Main Tier Word

Place your cursor over a word on a main tier line (e.g., `*CHI: I want cookie .`). A tooltip appears showing:

```
Main Tier Word: "cookie"

↔ %mor tier
  POS: n (noun)
  Stem: cookie

↔ %gra tier
  3|2|OBJ — OBJ → want (word 2)

↔ %pho tier
  kʊki

---
Alignment computed by talkbank-model
```

### Hovering on a %mor Item

Hover over a morphological analysis item to see its full breakdown:

```
Morphology Element: "pro:sub|I"

POS: pro:sub (subject pronoun)
Stem: I

← Main tier: "I" (word 1)

↔ %gra tier
  1|2|SUBJ — SUBJ → want (word 2)
```

Compound morphology, prefixes, suffixes, clitics, and translations are all shown when present.

### Hovering on a %gra Item

Hover over a grammatical relation to see the dependency it encodes:

```
Grammar Relation: "3|2|OBJ"

Source: cookie (word 3)
Head: want (word 2)
Relation: OBJ

← %mor tier: n|cookie
← Main tier: "cookie"
```

### Other Tiers

Hovering also works on `%pho` (phonological), `%mod` (model phonology), and `%sin` (gesture/sign) tier items, showing their alignment to the main tier and morphology.

### Header Hover

Hovering over `@` headers (`@Languages`, `@Participants`, `@ID`, `@Media`, etc.) shows inline documentation describing the header's purpose and syntax. The `@ID` hover is especially detailed, displaying a field-by-field table explaining each of the 10 pipe-delimited fields.

### Bullet Hover

Hovering over a timing bullet (`•NNN_NNN•`) shows the formatted start time, end time, and duration of the segment. This makes it easy to inspect timing without mentally converting millisecond values.

---

## Document Highlighting: Visual Alignment

Click on any word or tier item and all aligned elements across tiers are highlighted simultaneously. This provides an instant visual map of how a single word flows through the annotation layers.

For example, clicking on the word "cookie" on the main tier highlights:
- **"cookie"** on the main tier (primary highlight)
- **"n|cookie"** on the `%mor` tier (secondary highlight)
- **"3|2|OBJ"** on the `%gra` tier (secondary highlight)
- The corresponding item on `%pho`, `%mod`, or `%sin` if present

This works bidirectionally — clicking a `%gra` item highlights both its source word and head word in `%mor`, plus the corresponding main tier words.

---

## Dependency Graph Visualization

Visualize the grammatical dependency structure of any utterance as an interactive graph.

### How to Use

1. Place your cursor on an utterance that has both `%mor` and `%gra` tiers
2. Press **`Cmd+Shift+G`** (macOS) or **`Ctrl+Shift+G`** (Windows/Linux)
   - Alternatively: right-click → **Show Dependency Graph**
   - Or: Command Palette → **TalkBank: Show Dependency Graph**
3. A side panel opens with the rendered graph

### Graph Layout

- Words are displayed as labeled boxes, arranged left-to-right in utterance order
- Dependency arcs connect each word to its syntactic head
- Each arc is labeled with the grammatical relation (SUBJ, OBJ, DET, MOD, etc.)
- The ROOT relation connects to an invisible root node
- Arcs are **color-coded by relation type**:
  - Blue — SUBJ
  - Red — OBJ, OBJ2
  - Green — ROOT
  - Orange — JCT (adjunct)
  - Purple — MOD (modifier)
  - Light blue — DET (determiner)
  - Teal — QUANT (quantifier)
  - Gray — other relations

### Toolbar Controls

| Button | Action |
|--------|--------|
| **Zoom In** | Increase zoom level by 10% |
| **Zoom Out** | Decrease zoom level by 10% |
| **Slider** | Drag to set zoom (10%–300%) |
| **Fit** | Auto-fit the graph to the panel size |
| **SVG** | Download the graph as an SVG vector file |
| **PNG** | Download the graph as a high-resolution PNG (2x) |

The graph panel reuses a single tab — invoking the command on a different utterance updates the existing panel rather than opening a new one.

### Requirements

The Graphviz WASM renderer is bundled with the extension (`@hpcc-js/wasm`) and works offline. No internet connection is required.

---

## Code Completion

The extension provides context-aware autocompletion triggered by specific characters.

### Speaker Codes (trigger: `*`)

When you type `*` at the start of a line, the extension offers all speakers declared in `@Participants`:

```
*|  ← cursor here

Suggestions:
  CHI   target child     → inserts "*CHI:\t"
  MOT   mother           → inserts "*MOT:\t"
  FAT   father           → inserts "*FAT:\t"
```

Each suggestion includes the speaker's role from the participants header and auto-inserts the colon and tab separator.

### Tier Types (trigger: `%`)

When you type `%` at the start of a line after a main tier, the extension offers all standard dependent tier types:

```
%|  ← cursor here

Suggestions:
  mor   Morphological analysis    → inserts "%mor:\t"
  gra   Grammatical relations     → inserts "%gra:\t"
  pho   Phonological transcription
  mod   Model phonology
  sin   Gesture/sign annotations
  act   Action coding
  cod   General coding
  com   Comments
  ...and more
```

### Headers (trigger: `@`)

When you type `@` at the start of a line, the extension offers all 28 standard CHAT headers (`@Languages`, `@Participants`, `@ID`, `@Media`, `@Date`, `@Location`, etc.) with descriptions. Each suggestion auto-inserts the header name, colon, and tab separator.

### Brackets (trigger: `[`)

When you type `[` inside an utterance, the extension offers 17 standard bracket annotations (`[//]`, `[/]`, `[*]`, `[=]`, `[+ post]`, etc.) with descriptions. This helps you insert correctly-formatted CHAT annotations without memorizing all bracket codes.

### Postcodes (trigger: `+`)

When you type `+` in an utterance context, the extension offers valid CHAT postcodes:

```
+|  ← cursor here

Suggestions:
  +"    Quotation follows
  +,.   Self-completion
  +/.   Interruption
  +/?   Interruption question
  +//.  Self-interruption
  +...  Trailing off
```

---

## Snippets

The extension includes 8 CHAT snippets for common file structures. Type the prefix and press `Tab` to expand. Tab stops let you fill in values sequentially.

| Prefix | Snippet | Description |
|--------|---------|-------------|
| `@UTF8` / `header` / `newfile` | Header block | Complete file skeleton: `@UTF8`, `@Begin`, `@Languages`, `@Participants`, `@ID` lines, first utterance, `@End` |
| `@Participants` / `participant` | Participants | `@Participants` header with two speakers |
| `@ID` / `id` | ID header | Single `@ID` line with all 10 pipe-delimited fields |
| `*` / `utterance` | Main tier | `*CHI:` utterance line with terminator |
| `%mor` | Mor tier | `%mor:` dependent tier line |
| `%gra` | Gra tier | `%gra:` dependent tier line |
| `@Comment` / `comment` | Comment | `@Comment:` header |
| `@Bg` / `@Eg` / `gem` | Gem block | `@Bg`/`@Eg` pair wrapping a cursor position |

Snippets are defined in `snippets/chat.json` and are available in any `.cha` file.

---

## Quick Fixes

When the language server detects certain errors, it offers automatic fixes via the lightbulb icon or `Cmd+.` / `Ctrl+.`:

### E241: Illegal Untranscribed Marker

If you write `xx` (which is not a valid CHAT marker), the fix offers to replace it with `xxx` (the correct untranscribed speech marker).

### E242: Incomplete Word

For words that appear to be cut off, the fix offers to append the trailing-off marker `+...`.

### E301: Missing Terminator

If an utterance is missing its final terminator, three options are offered:
- Add `.` (declarative/default)
- Add `?` (question)
- Add `!` (exclamation)

### E308: Undeclared Speaker

If a speaker code is used on a main tier but not declared in `@Participants`, the fix offers to add the speaker to the `@Participants` header (e.g., "Add 'INV' to @Participants").

### E501: Missing @Begin

If the file is missing `@Begin`, the fix inserts it after the `@UTF8` header line.

### E502: Missing @End

If the file is missing `@End`, the fix inserts it at the end of the file.

### E503: Missing @UTF8

If the file is missing the `@UTF8` declaration, the fix inserts it at the very start of the file.

---

## Rename

Rename a speaker code across the entire file with `F2`. Place your cursor on a speaker code (e.g., `*CHI` on a main tier line, or a code in `@Participants`) and press `F2`. Type the new code and all occurrences are updated simultaneously:

- The speaker entry in `@Participants`
- The corresponding `@ID` header line
- All main tier lines using that speaker code (`*CHI:` becomes `*NEW:`)

This is a standard LSP rename operation — it also works via right-click **Rename Symbol** or the Command Palette.

---

## Linked Editing

When you edit a speaker code (e.g., `CHI`), all other occurrences of that speaker in the document are highlighted for simultaneous editing. This is different from F2 Rename — linked editing provides real-time, in-place editing of all matching speaker codes as you type. Enable it via VS Code's `editor.linkedEditing` setting.

---

## Find All References

Find every occurrence of a speaker code with `Shift+F12`. Place your cursor on a speaker code and press `Shift+F12` (or right-click **Find All References**). The References panel shows all locations where the speaker appears:

- `@Participants` declaration
- `@ID` header
- All main tier lines for that speaker

---

## Code Lens

Above the `@Participants` header, a code lens annotation shows the utterance count for each speaker in the file (e.g., `CHI: 42 utterances | MOT: 38 utterances`). This provides an at-a-glance summary of speaker activity without running an analysis command.

---

## Document Symbols

The extension provides document symbols for navigating within a single `.cha` file via `Cmd+Shift+O` / `Ctrl+Shift+O`.

### Workspace Symbols

Use `Cmd+T` (macOS) / `Ctrl+T` (Windows/Linux) to search across all open CHAT files for headers and speaker lines. This allows quick navigation to any `@` header or `*SPEAKER:` line across the entire workspace without switching files manually.

---

## Inlay Hints

Subtle inline annotations appear at the end of tier lines when alignment counts don't match. These are displayed in a muted color and don't modify the file.

### Main ↔ %mor Mismatch

When the main tier has a different number of words than the `%mor` tier:

```
*CHI:	I want cookie .
%mor:	pro:sub|I v|want   [alignment: 3 main ↔ 2 mor]
```

The hint `[alignment: 3 main ↔ 2 mor]` appears at the end of the `%mor` line, indicating a missing morphological analysis.

### %gra ↔ %mor Mismatch

Similarly, when `%gra` has a different number of relations than `%mor` has items:

```
%mor:	pro:sub|I v|want n|cookie
%gra:	1|2|SUBJ 2|0|ROOT   [alignment: 2 gra ↔ 3 mor]
```

Hovering over the hint shows a tooltip explaining the expected alignment.

---

## Go to Definition

Press `F12` or `Cmd+Click` / `Ctrl+Click` to jump to definitions:

### Smart Selection Expand

Use VS Code's **Expand Selection** (`Cmd+Shift+Right Arrow` on macOS, `Ctrl+Shift+Right Arrow` on Windows/Linux) to expand through CHAT structural levels: word, tier content, full line, utterance block (main tier plus dependent tiers), and finally the entire file. This mirrors VS Code's standard selection expansion but is tuned for the CHAT document structure.

### Speaker Definition

Clicking on a speaker code (e.g., `*CHI` on a main tier line) jumps to its declaration in the `@Participants` header.

### %mor → Main Tier

Clicking on a `%mor` item jumps to the aligned word on the main tier.

### %gra → Main Tier

Clicking on a `%gra` relation jumps through `%mor` to the aligned main tier word.

---

## Document Formatting

Format the entire document to canonical CHAT style:

- **Shortcut**: `Shift+Alt+F` (or your configured format shortcut)
- **Command Palette**: **Format Document**
- **On save**: Configure VS Code's `editor.formatOnSave` setting

The formatter normalizes whitespace, header ordering, speaker code formatting, and tier indentation to produce canonical CHAT output. If the document is already in canonical form, no changes are applied.

### On-Type Formatting

After typing `:` on a `*SPEAKER:` or `%tier:` line, a tab character is automatically inserted. This matches the CHAT format convention where a tab always follows the colon on speaker and tier lines. No manual tab insertion is needed.

---

## CLAN Analysis Commands

Run any of 33 CLAN analysis commands on the current `.cha` file. Results display in a styled webview panel with stat cards, tables, and proportional bar charts.

### How to Use

1. Open a `.cha` file in the editor
2. Right-click and choose **Run CLAN Analysis**, or use the Command Palette (**TalkBank: Run CLAN Analysis**)
3. Select a command from the QuickPick list
4. Some commands prompt for additional input (keywords, file paths)
5. Results appear in a side panel

### Available Commands (33)

| Command | Description |
|---------|-------------|
| **freq** | Word/morpheme frequency counts and type-token ratio |
| **mlu** | Mean length of utterance (morphemes) |
| **mlt** | Mean length of turn (utterances/words per turn) |
| **wdlen** | Word length distribution |
| **wdsize** | Word size (character lengths from %mor stems) |
| **maxwd** | Longest words per speaker |
| **freqpos** | Frequency by part-of-speech from `%mor` tier |
| **timedur** | Time duration from bullet timing marks |
| **kwal** | Keyword-in-context search (prompts for keywords) |
| **combo** | Boolean keyword search with AND/OR (prompts for expression) |
| **gemlist** | List `@Bg`/`@Eg` gem segments |
| **cooccur** | Word co-occurrence counting |
| **dist** | Word distribution/dispersion analysis |
| **chip** | Child/parent interaction profile |
| **phonfreq** | Phonological segment frequency from `%pho` tier |
| **modrep** | Model and replica analysis of imitations |
| **vocd** | Vocabulary diversity (D statistic) |
| **codes** | Frequency of coding tier codes |
| **complexity** | Syntactic complexity ratio from `%gra` tier |
| **corelex** | Core vocabulary analysis (frequent words) |
| **chains** | Code chains and sequences on `%cod` tier |
| **dss** | Developmental Sentence Scoring |
| **eval** | Combined language evaluation measures |
| **flucalc** | Fluency calculation (disfluency measures) |
| **ipsyn** | Index of Productive Syntax |
| **keymap** | Keyword-based contingency mapping (prompts for keyword codes) |
| **kideval** | Child language evaluation (DSS + IPSyn + MLU) |
| **mortable** | Morpheme frequency table (prompts for script file) |
| **rely** | Inter-rater reliability between two files (prompts for second file) |
| **script** | Compare transcript against template (prompts for template file) |
| **sugar** | Sampling Utterances and Grammatical Analysis |
| **trnfix** | Compare two dependent tiers for mismatches |
| **uniq** | Find repeated/unique utterances |

### Commands That Prompt for Input

- **kwal**: Enter one or more keywords (space-separated)
- **combo**: Enter a search expression (e.g., `want+cookie` or `want,milk`)
- **keymap**: Enter keyword codes (space-separated)
- **mortable**: Select a morpheme script file (`.cut`) via file picker
- **script**: Select a template CHAT file (`.cha`) via file picker
- **rely**: Select a second CHAT file (`.cha`) for comparison via file picker

### Exporting Results

Click the **Export CSV** button in the analysis panel toolbar to save all visible tables and statistics as a CSV file. A save dialog lets you choose the output location. The CSV format is compatible with Excel, Google Sheets, and R/Python data tools.

### Running Analysis on a Directory

To analyze all `.cha` files in a directory at once:

1. Right-click a folder in the Explorer sidebar → **Run CLAN Analysis on Directory**
2. Select an analysis command from the picker
3. Results from all files are aggregated in a single analysis panel

The language server's `resolve_files()` walks the directory tree recursively. This is equivalent to CLAN's batch-mode directory analysis.

---

## KidEval / Eval Assessment

Compare a child's language measures against normative databases.

### How to Use

1. Right-click in a `.cha` file → **Run KidEval** or **Run Eval**, or use the Command Palette
2. Select a library directory (default: `/Users/Shared/CLAN/lib/kideval/` or `.../eval/`)
3. Choose a normative database from the dropdown
4. Optionally filter by age range and gender
5. Click **Run** — results show z-scores comparing the child to normative means

### Results Display

- **Stat cards**: Key metrics (MLU, DSS, IPSyn, etc.) with z-score coloring
- **Comparison table**: Side-by-side view of child vs. normative mean and standard deviation
- **Export**: Click **Export CSV** to save results as a spreadsheet-ready file

KidEval focuses on child language development measures; Eval covers general language assessment. Both use the same panel UI with different normative databases.

### Eval-D (Dementia Assessment)

**Command Palette → TalkBank: Run Eval-D (Dementia)** or right-click → **Run Eval-D (Dementia)**

Uses the same panel UI as KidEval/Eval but with DementiaBank normative databases for evaluating language in dementia contexts (MCI, Possible AD, Probable AD, Vascular, Control).

---

## Participant Editor

Edit `@ID` headers in a visual table instead of manipulating pipe-delimited text.

### How to Use

1. Right-click in a `.cha` file → **Edit Participants**, or use the Command Palette
2. A table opens showing all `@ID` lines with columns for each of the 10 fields:
   - Language, Corpus, Speaker Code, Age, Sex, Group, SES, Role, Education, Custom
3. Edit any cell directly in the table
4. Click **Save** to write the canonical `@ID` lines back to the document

All parsing and serialization is handled by the language server (`talkbank/getParticipants` and `talkbank/formatIdLine`). The TypeScript side is a thin UI layer.

---

## Media Playback

Play audio or video files linked via `@Media:` headers directly inside VS Code.
Timing segments are sourced from the language-server alignment sidecar when available, with direct `•beg_end•` parsing as fallback.

### Play at Cursor

**Shortcut**: `Cmd+Shift+Enter` (macOS) / `Ctrl+Shift+Enter` (Windows/Linux)

Plays the single bullet segment (`•beg_end•`) nearest to the cursor position. A media panel opens in the editor showing playback controls.

### Continuous Play

**Shortcut**: `Cmd+Shift+/` (macOS) / `Ctrl+Shift+/` (Windows/Linux)

Plays all segments from the cursor position through to the end of the file. The editor selection tracks the currently-playing utterance line.

Segments play sequentially in **document order** (top to bottom), not sorted
by media time.  When one segment finishes, playback seeks to the next
segment's start time.

### Playback Speed

The media panel toolbar includes a speed slider (0.25x–2x). Drag the slider to adjust playback speed. The current rate is shown next to the slider. Useful for slowing down fast speech during transcription.

### Rewind and Loop

- **Rewind** (`F8`): Rewind by a configurable number of seconds (default: 2). Set via `talkbank.transcription.rewindSeconds` in settings.
- **Loop Segment** (`F5`): Toggle looping of the current segment.
- **Stop Playback**: Available via the Command Palette.

### Overlapping Bullets (Cross-Speaker Overlap)

CHAT allows cross-speaker overlapping bullets — two different speakers can
have utterances whose time ranges overlap, as long as utterances are ordered
by start time in the file (E701).

When the file contains overlapping bullets, playback works as follows:

- **Play at Cursor** plays exactly the clicked utterance's time range.
  No other utterances are affected.

- **Continuous Play** plays each utterance in full, in document order.
  If speaker A's utterance covers 0–3.5s and speaker B's covers 2–2.5s,
  you will hear A's full range, then B's full range.  The overlapping
  audio region (2–2.5s) is heard twice — once as part of each speaker's
  turn.  This matches CLAN behavior and lets you hear each speaker's
  complete utterance without truncation.

- **Waveform overlays** render each segment independently.  Overlapping
  segments appear as stacked colored bars on the waveform.

This behavior is intentional.  For transcripts with frequent backchannels
and short overlapping speech (common in aphasia protocols, conversation
analysis, and multi-party recordings), hearing each speaker's complete turn
in sequence is more useful than trying to play simultaneous audio streams.

### Requirements

The `.cha` file must contain an `@Media:` header, and the referenced media file must be in the same directory (or a sibling `media/` directory).

### Document Links

`@Media` file references are rendered as clickable links in the editor. If the referenced media file exists alongside the `.cha` file (or in a sibling `media/` directory), clicking the link opens it directly.

---

## Waveform View

Visualize the audio waveform of the linked media file with timed utterance overlays.

### How to Use

1. Open a `.cha` file with an `@Media:` header
2. Press `Cmd+Shift+W` (macOS) / `Ctrl+Shift+W` (Windows/Linux)
3. A waveform panel opens showing the audio amplitude

### Interaction

- **Click** anywhere on the waveform to seek to that time. The editor cursor moves to the nearest utterance, and the media panel (if open) seeks as well.
- **Colored overlays** mark each `•beg_end•` bullet segment on the waveform.
- The waveform and media panels are coordinated: playing a segment highlights it in the waveform.

### Zoom and Scroll

- **Zoom in/out**: Use the toolbar buttons, the zoom slider (100%–2000%), or scroll the mouse wheel
- **Fit to window**: Click the fit button to reset zoom to 100%
- **Scroll**: When zoomed in, the waveform scrolls horizontally. During playback, the view auto-scrolls to keep the current segment visible.
- **Pointer-centered zoom**: Mouse wheel zooming is centered on the pointer position for precise navigation.

---

## Picture Display

Show elicitation pictures (Cookie Theft, picture descriptions, etc.) alongside the transcript — CLAN's PictController equivalent.

### How to Use

1. Open a `.cha` file
2. Right-click → **TalkBank: Media** → **Show Elicitation Picture**, or use the Command Palette
3. The extension searches for associated images:
   - `%pic:` references in the document (e.g., `%pic:"image002.jpg"`)
   - Image files with the same base name as the `.cha` file
   - Any image files in the same directory
4. If multiple images are found, a picker appears

The picture opens in a side panel and scales to fit the available space.

---

## Coder Mode

Structured annotation workflow for coding transcripts with a predefined scheme — CLAN's Coder mode equivalent. Used in research workflows where trained coders annotate utterances with codes from a hierarchical coding scheme.

### Workflow

1. Open a `.cha` file
2. Right-click → **TalkBank: Navigation** → **Start Coder Mode**, or use the Command Palette
3. Select a `.cut` codes file (tab-indented hierarchy of codes)
4. The cursor jumps to the first uncoded utterance
5. A QuickPick shows the code hierarchy — select a code to insert it as `%cod:\tcode`
6. The cursor automatically advances to the next uncoded utterance
7. Repeat until all utterances are coded

### Keyboard Shortcuts (active only during coder mode)

| Shortcut | Action |
|----------|--------|
| `Cmd+Enter` | Advance to next uncoded utterance |
| `Cmd+Shift+C` | Insert a code on the current utterance |

### Codes File Format

The `.cut` file uses tab-indented hierarchies:

```
$PRA
	$PRA:request
	$PRA:demand
$ACT
	$ACT:play
	$ACT:read
```

The QuickPick displays codes with indentation to preserve the hierarchy.

---

## Transcription Mode

Create new transcripts by typing while audio plays, then stamping timing bullets at utterance boundaries.

### Workflow

1. Open a `.cha` file with an `@Media:` header
2. **Command Palette** → **TalkBank: Start Transcription Mode**
3. Audio begins playing from the start
4. Type the speaker's utterance on the current line
5. Press `F4` to stamp a timing bullet (`•prevMs_currentMs•`) and advance to a new utterance line
6. Repeat until done
7. **Command Palette** → **TalkBank: Stop Transcription Mode**

### Configuration

| Setting | Default | Description |
|---------|---------|-------------|
| `talkbank.transcription.defaultSpeaker` | `CHI` | Speaker code for new utterance lines |
| `talkbank.transcription.rewindSeconds` | `2` | Seconds to rewind with F8 |

### Foot-Pedal / Custom Keybindings

To remap transcription keys (e.g., for a USB foot pedal):

1. Command Palette → **TalkBank: Configure Transcription Keybindings**
2. The VS Code keybindings editor opens, pre-filtered to TalkBank transcription commands
3. Click the pencil icon next to any command to assign a new keybinding

This is the recommended way to set up foot-pedal controls for F4 (stamp), F8 (rewind), and F5 (loop).

---

## Walker Mode

Step through utterances one segment at a time, playing each segment's audio.

### How to Use

- **Next utterance**: `Alt+Down`
- **Previous utterance**: `Alt+Up`

Each step moves the editor cursor to the corresponding utterance line and plays the segment if media is available. The walker resets when the active document changes.

---

## Speaker Filtering

View a transcript filtered to show only selected speakers.

### How to Use

1. Right-click in a `.cha` file → **Filter by Speaker**, or use the Command Palette
2. Select one or more speaker codes from the multi-select picker
3. A filtered read-only document opens in a side-by-side panel

The filtered view includes all headers plus only the utterance blocks for the selected speakers. The view is still processed by the language server for validation and highlighting.

---

## Validation Explorer

A dedicated tree view in the Explorer sidebar for bulk validation of `.cha` files.

### Opening

The **CHAT Validation** panel appears automatically in the Explorer sidebar when the extension is active. It shows the workspace folder structure filtered to `.cha` files.

### Usage

1. **Validate a single file**: Click the checkmark icon next to any `.cha` file in the tree
2. **Validate a directory**: Click the double-checkmark icon next to a folder, or use the toolbar button to validate the entire workspace
3. **View results**: Files show pass/fail status with icons:
   - Green checkmark — valid
   - Red X with error count — invalid (expand to see individual errors)
   - Spinner — currently validating
4. **Navigate to errors**: Click on an error item to jump to the exact line and column in the editor
5. **Clear cache**: Click the trash icon to clear cached results for a file or directory, forcing revalidation
6. **Refresh**: Use the refresh button in the toolbar to update the tree view

### Toolbar

| Button | Action |
|--------|--------|
| Double-checkmark | Validate all `.cha` files in the workspace |
| Refresh | Refresh the tree view |

### Context Menu

Right-click on files or directories in the tree for:
- **Validate File** / **Validate Directory**
- **Clear Cache**

---

## Cache Management

The extension uses a SQLite cache (at `~/.cache/talkbank-tools/talkbank-cache.db`) to store validation results. This speeds up repeated validation of large corpora.

### Status Bar

A cache indicator appears in the bottom-right of the status bar showing the number of cached files (e.g., `Cache: 95,247 files`). Click it to view detailed statistics.

### View Cache Statistics

**Command Palette → TalkBank: View Cache Statistics**

Displays a popup with:
- Cache database size
- Total cached entries
- Valid / invalid / expired counts
- Cache hit rate
- Last updated timestamp

### Clear All Cache

**Command Palette → TalkBank: Clear All Validation Cache**

Removes the entire cache database. Use this only if the cache becomes corrupted or you want a completely fresh start. Normally, use `--force` or the per-item clear button in the Validation Explorer instead.

---

## CLAN Integration

If you have the CLAN application installed, the extension can open `.cha` files directly in CLAN.

### Usage

- **Right-click in editor** → **Open in CLAN**
- **Right-click on a `.cha` file** in the Explorer → **Open in CLAN**

The file opens in CLAN at your current cursor position (line and column).

### Platform Support

- **macOS**: Uses Apple Events for IPC
- **Windows**: Uses Windows messaging
- **Linux**: Not supported (CLAN is macOS/Windows only)

This feature is optional — all validation and editing features work without CLAN installed.

---

## Special Character Input

Insert Conversation Analysis and CHAT special characters via a compose-key mode.

### How to Use

1. Press **`Cmd+Shift+1`** (macOS) / **`Ctrl+Shift+1`** (Windows/Linux) for **CA mode**, or **`Cmd+Shift+2`** / **`Ctrl+Shift+2`** for **CHAT mode**
2. A status bar indicator shows the active compose mode (e.g., "CA Char…")
3. Press the trigger key to insert the corresponding Unicode symbol
4. Press `Escape` to cancel without inserting

### CA Mode Characters (Cmd+Shift+1)

| Key | Symbol | Meaning |
|-----|--------|---------|
| `1` | ⇗ | Rise to high |
| `2` | ↗ | Rise to mid |
| `3` | → | Level |
| `4` | ↘ | Fall to mid |
| `5` | ⇘ | Fall to low |
| `[` | ⌈ | Overlap start (raised) |
| `]` | ⌉ | Overlap end (raised) |
| `{` | ⌊ | Overlap start (lowered) |
| `}` | ⌋ | Overlap end (lowered) |
| `.` | ∙ | Inhalation |
| `=` | ≈ | Latching |
| `0` | ° | Softer |
| `)` | ◉ | Louder |
| `w` | ∬ | Whisper |
| `s` | ∮ | Singing |
| `b` | ♋ | Breathy voice |
| `*` | ⁎ | Creaky |
| `/` | ⁇ | Unsure |

### CHAT Mode Characters (Cmd+Shift+2)

| Key | Symbol | Meaning |
|-----|--------|---------|
| `q` | ʔ | Glottal stop |
| `Q` | ʕ | Hebrew glottal |
| `:` | ː | Long vowel |
| `H` | ʰ | Raised h |
| `<` | ‹ | Group start |
| `>` | › | Group end |
| `{` | 〔 | Sign group start |
| `}` | 〕 | Sign group end |
| `1` | ˈ | Primary stress |
| `2` | ˌ | Secondary stress |
| `m` | … | Missing word (%pho) |
| `/` | ↫ | Left arrow with circle |
| `=` | ≠ | Crossed equal |

---

## Keyboard Shortcuts

| Shortcut | Action |
|----------|--------|
| `Cmd+Shift+G` / `Ctrl+Shift+G` | Show Dependency Graph |
| `Cmd+Shift+Enter` / `Ctrl+Shift+Enter` | Play Media at Cursor |
| `Cmd+Shift+/` / `Ctrl+Shift+/` | Play Media Continuously |
| `Cmd+Shift+W` / `Ctrl+Shift+W` | Show Waveform View |
| `Alt+Down` | Walker: Next Utterance |
| `Alt+Up` | Walker: Previous Utterance |
| `F8` | Rewind Media |
| `F5` | Toggle Segment Loop |
| `F4` | Stamp Timestamp Bullet (transcription mode only) |
| `Cmd+Shift+1` / `Ctrl+Shift+1` | Insert CA Special Character (compose mode) |
| `Cmd+Shift+2` / `Ctrl+Shift+2` | Insert CHAT Special Character (compose mode) |
| `Cmd+Shift+M` / `Ctrl+Shift+M` | Open Problems panel (all diagnostics) |
| `F2` | Rename Speaker Code |
| `Shift+F12` | Find All References |
| `F12` / `Cmd+Click` | Go to Definition |
| `Cmd+.` / `Ctrl+.` | Quick Fix (when lightbulb appears) |
| `Shift+Alt+F` | Format Document |

---

## Command Reference

All commands are available via the Command Palette (`Cmd+Shift+P` / `Ctrl+Shift+P`) under the **TalkBank** category:

| Command | Description |
|---------|-------------|
| **TalkBank: Show Dependency Graph** | Render `%gra` dependency graph for current utterance |
| **TalkBank: Run CLAN Analysis** | Run one of 30 analysis commands on the current file |
| **TalkBank: Play Media at Cursor** | Play the bullet segment nearest the cursor |
| **TalkBank: Play Media Continuously** | Play all segments from cursor to end of file |
| **TalkBank: Stop Media Playback** | Stop audio/video playback |
| **TalkBank: Rewind Media** | Rewind by configured seconds |
| **TalkBank: Toggle Segment Loop** | Loop the current segment |
| **TalkBank: Show Waveform View** | Open waveform visualization of linked media |
| **TalkBank: Walker: Next Utterance** | Step to the next utterance segment |
| **TalkBank: Walker: Previous Utterance** | Step to the previous utterance segment |
| **TalkBank: Start Transcription Mode** | Begin transcription with media playback |
| **TalkBank: Stamp Timestamp Bullet** | Insert timing bullet at current playback position |
| **TalkBank: Stop Transcription Mode** | End transcription mode |
| **TalkBank: Filter by Speaker** | Show transcript filtered to selected speakers |
| **TalkBank: Validate File** | Validate a single `.cha` file |
| **TalkBank: Validate Directory** | Validate all `.cha` files in a directory |
| **TalkBank: Refresh Validation** | Refresh the Validation Explorer tree |
| **TalkBank: Clear Cache** | Clear cache for a specific file or directory |
| **TalkBank: Clear All Validation Cache** | Clear the entire validation cache |
| **TalkBank: View Cache Statistics** | Show cache size, hit rate, and entry counts |
| **TalkBank: Open in CLAN** | Open current file in the CLAN application |
| **TalkBank: Run KidEval** | Open KidEval normative comparison panel |
| **TalkBank: Run Eval** | Open Eval normative comparison panel |
| **TalkBank: Run Eval-D (Dementia)** | Open Eval-D dementia assessment panel |
| **TalkBank: Edit Participants** | Open @ID line editor |
| **TalkBank: Insert CA Special Character** | Enter CA compose mode (Cmd+Shift+1) |
| **TalkBank: Insert CHAT Special Character** | Enter CHAT compose mode (Cmd+Shift+2) |
| **TalkBank: Cancel Special Character Input** | Cancel active compose mode |
| **TalkBank: Run CLAN Analysis on Directory** | Run analysis on all `.cha` files in a folder |
| **TalkBank: Configure Transcription Keybindings** | Open keybinding editor for transcription keys |

---

## Settings

The extension exposes the following settings (accessible via `Cmd+,` / `Ctrl+,` and searching for "talkbank"):

| Setting | Type | Default | Description |
|---------|------|---------|-------------|
| **Transcription** | | | |
| `talkbank.transcription.defaultSpeaker` | string | `CHI` | Speaker code for new utterance lines in transcription mode |
| `talkbank.transcription.rewindSeconds` | number | `2` | Seconds to rewind with F8 |
| **Walker & Playback** | | | |
| `talkbank.walker.autoPlay` | boolean | `true` | Auto-play media segment when stepping with Alt+Down/Up |
| `talkbank.walker.loopCount` | integer | `1` | Times to loop each segment (0 = loop indefinitely) |
| `talkbank.walker.pauseSeconds` | number | `0` | Seconds to pause between segments during continuous playback |
| `talkbank.walker.walkLength` | integer | `0` | Number of utterances to play continuously (0 = all remaining) |
| `talkbank.media.defaultSpeed` | integer | `100` | Playback speed percentage (25–200) |
| **Display** | | | |
| `talkbank.bullets.display` | string | `"dim"` | How timing bullets appear: `"dim"` (35% opacity), `"hidden"` (invisible), or `"normal"` (full visibility) |
| `talkbank.inlayHints.enabled` | boolean | `true` | Toggle inlay hints (alignment mismatch annotations) on or off |
| **Validation** | | | |
| `talkbank.validation.severity` | string | `"all"` | Filter which diagnostics are shown: `"all"`, `"errorsOnly"`, or `"errorsAndWarnings"` |
| **Advanced** | | | |
| `talkbank.lsp.binaryPath` | string | — | Override the auto-detected `chatter` binary path used for `chatter lsp` |

---

## Troubleshooting

### The language server isn't starting

The extension looks for the `chatter` binary in three places, then launches it with `lsp`:
1. System PATH (via `which chatter`)
2. `target/debug/chatter` relative to the extension directory
3. `target/release/chatter` relative to the extension directory

Make sure you've built it:

```bash
cargo build -p talkbank-lsp
# or for better performance:
cargo build --release -p talkbank-lsp
```

Check the **Output** panel (select "TalkBank Language Server" from the dropdown) for error messages.

### No diagnostics appear

- Ensure the file has a `.cha` extension
- Check that the language mode in the bottom-right status bar shows "CHAT"
- Verify the language server is running (check Output panel)

### Dependency graph shows "Failed to load Graphviz renderer"

The Graphviz renderer is bundled with the extension and works offline. If you see this error, the extension may not be installed correctly. Reinstall or rebuild the extension and ensure `node_modules/@hpcc-js/wasm/` is present.

### Hover/highlighting shows no alignment data

Alignment information requires both a main tier and at least one dependent tier (`%mor`, `%gra`, etc.) to be present. The tiers must also be syntactically valid for alignment to be computed.

### Formatting doesn't change anything

If the document is already in canonical CHAT form, the formatter correctly reports no changes needed.

---
Last Updated: 2026-03-06
