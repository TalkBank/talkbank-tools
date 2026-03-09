# Feasibility Analysis: Replacing CLAN with talkbank-tools + VS Code

**Date:** 2026-02-17
**Context:** CLAN is legacy C/C++ software that is difficult to integrate into modern toolchains (Rust-based tools, GUI frameworks, WebAssembly targets). This document assesses whether talkbank-tools can fully replace the CLAN macOS application by (a) rewriting analysis tools in Rust and (b) incorporating GUI features into the VS Code extension.

---

## 1. Executive Summary

**Yes, it is feasible to replace CLAN entirely**, but it is a large, multi-year effort that should be approached in phases. The good news:

- The core parsing/validation/serialization infrastructure in talkbank-tools is already **more capable and more robust** than CLAN's CHECK command
- The CUTT framework that all 60+ CLAN analysis tools share has a simple, well-defined architecture that maps cleanly to a Rust trait-based plugin system
- Most CLAN analysis tools are straightforward statistical/search operations over parsed CHAT ASTs — exactly the kind of work Rust excels at
- VS Code's extension API provides all the primitives needed for CLAN's GUI features (audio/video playback, synchronized highlighting, coder mode)
- The batchalign-core bridge already handles the hardest NLP tasks (morphosyntax via neural models, forced alignment, disfluency detection)

The risks are primarily around **scope management** and **domain fidelity**: each of CLAN's ~60 unique analysis tools encodes decades of linguistic convention that must be replicated exactly, and the audio/video synchronized transcription workflow is the most complex GUI feature to port.

---

## 2. What CLAN Contains

### 2.1 Scale of the Codebase

| Component | Files | Lines of Code |
|-----------|-------|---------------|
| Core analysis engine (C++) | 112 .cpp files | ~137,000 |
| macOS GUI (Objective-C++) | 31 .mm files | ~27,000 |
| POST tagger subsystem | 57 files | ~19,000 |
| MOR morphological analyzer | 9 files | ~7,000 |
| MEGRASP grammar parser | 9 files | ~5,000 |
| VOCD vocabulary diversity | 16 files | ~5,000 |
| Headers & utilities | 102+ .h files | ~50,000 |
| **Total** | **~265 files** | **~215,000 lines** |

### 2.2 Complete Feature Inventory

#### A. Analysis Commands (Chapter 7 — 18 commands)
These are the core research tools. Each reads CHAT files, applies speaker/tier filtering, and produces statistical output.

| Command | Purpose | CLAN LoC | Complexity |
|---------|---------|----------|------------|
| **FREQ** | Word frequency counts, crosstabs, concordances | ~5,000 | High (many options) |
| **MLU** | Mean Length of Utterance | ~3,500 | Medium |
| **MLT** | Mean Length of Turn | ~3,000 | Medium |
| **KWAL** | Keyword-in-context search | ~3,000 | Medium |
| **COMBO** | Boolean search across tiers | ~2,500 | Medium |
| **DIST** | Distance between words/codes | ~2,500 | Medium |
| **CHAINS** | Interactional code sequences | ~3,000 | Medium |
| **COOCCUR** | Word co-occurrence tables | ~2,000 | Medium |
| **MAXWD** | Maximum word statistics | ~1,500 | Low |
| **WDLEN** | Word length distribution | ~2,000 | Low |
| **TIMEDUR** | Pause/overlap duration | ~3,500 | Medium |
| **MODREP** | Modification/repetition tracking | ~4,000 | Medium |
| **VOCD** | Vocabulary diversity (D statistic) | ~5,000 (subsystem) | High |
| **GEM/GEMFREQ/GEMLIST** | Gem-based analysis | ~3,000 total | Low-Medium |
| **KEYMAP** | Contingency analysis | ~2,000 | Medium |
| **RELY** | Inter-rater reliability | ~5,000 | High |
| **SCRIPT** | Fixed script comparison | ~2,000 | Low |
| **CHECK** | Format validation | ~6,000 | High (already replaced) |

#### B. Profiling Commands (Chapter 8 — 10 commands)
Higher-level measures that combine multiple analyses. These are the most clinically important tools.

| Command | Purpose | CLAN LoC | Complexity |
|---------|---------|----------|------------|
| **EVAL / EVAL-D** | Comprehensive language profile | ~7,300 | Very High |
| **KIDEVAL** | Child evaluation with normative database | ~4,700 | Very High |
| **IPSYN** | Index of Productive Syntax | ~4,000 | High (rule-based) |
| **DSS** | Developmental Sentence Scoring | ~2,600 | High (rule-based) |
| **C-QPA** | Quantitative Production Analysis | ~4,500 | High |
| **C-NNLA** | Northwestern Narrative Language Analysis | ~3,000 | High |
| **FLUCALC** | Fluency calculation | ~2,500 | Medium |
| **CORELEX** | Core vocabulary analysis | ~2,000 | Medium |
| **MORTABLE** | Morphology tables | ~2,500 | Medium |
| **SUGAR** | Systematic utterance analysis | ~2,500 | Medium |

#### C. Format Conversion Commands (Chapter 10 — 17 commands)

| Direction | Commands |
|-----------|----------|
| **CHAT to X** | CHAT2ANVIL, CHAT2CA, CHAT2ELAN, CHAT2PRAAT, CHAT2SRT, CHAT2TEXT, CHAT2XMAR |
| **X to CHAT** | ANVIL2CHAT, ELAN2CHAT, LAB2CHAT, LENA2CHAT, LIPP2CHAT, PRAAT2CHAT, RTF2CHAT, SALT2CHAT, SRT2CHAT, TEXT2CHAT |

#### D. Reformatting Commands (Chapter 11 — 13 commands)
| Command | Purpose |
|---------|---------|
| CHSTRING | String replacement |
| DATES | Age/date calculation |
| FLO | Simplified main line |
| INDENT | CA overlap realignment |
| LINES | Line operations |
| LONGTIER | Remove line wraps |
| MEDIALINE | Media timing ops |
| REPEAT | Repetition marking |
| RETRACE | Retracing marking |
| ROLES | Role reassignment |
| SEGMENT | Utterance segmentation |
| TIERORDER | Tier reordering |
| TRIM | Trim file sections |

#### E. Format Repair Commands (Chapter 12 — 7 commands)
| Command | Purpose |
|---------|---------|
| COMBTIER | Combine tiers |
| DELIM | Add missing terminators |
| FIXBULLETS | Fix media bullets |
| FIXIT | Split multi-utterance lines |
| LOWCASE | Fix capitalization |
| QUOTES | Extract quoted material |
| CP2UTF | Encoding conversion |

#### F. NLP Subsystems (built into the application)
| Subsystem | Purpose | Notes |
|-----------|---------|-------|
| **MOR** | Morphological parser | Rule-based: arules, crules, drules + lexicon trie |
| **POST** | POS tagger | Brill tagger with training infrastructure |
| **MEGRASP** | Grammar/semantic parser | Maximum entropy model |

#### G. GUI Features (macOS application)
| Feature | Implementation | LoC |
|---------|---------------|-----|
| **Document editor** | Multi-document text editor with CHAT awareness | ~13,000 |
| **Audio/video playback** | AVPlayer/AVAudioPlayer with bullet sync | ~900 |
| **Sonic mode** | Waveform display linked to text | via Praat integration |
| **Transcriber mode** | Keystroke-controlled transcription | ~500 |
| **Walker controller** | Step-through audio with auto-highlighting | ~600 |
| **Coder mode** | Structured code entry with codes file | ~1,000 |
| **KIDEVAL/EVAL dialogs** | Specialized tool UIs with database comparison | ~4,500 |
| **Video linking** | Create/edit media-text associations | ~1,000 |
| **Audio anonymization** | Redact portions of audio | ~300 |
| **Print support** | Multi-page formatted printing | ~1,200 |
| **Command palette** | Run CLAN commands from GUI | ~1,000 |
| **Participant/ID management** | Edit @ID headers | ~1,900 |

---

## 3. What talkbank-tools Already Covers

### 3.1 Already Replaced (Fully)
| CLAN Feature | talkbank-tools Equivalent | Status |
|-------------|--------------------------|--------|
| CHECK (validation) | `chatter validate` + LSP diagnostics | **Superior** — more errors, structured codes, caching |
| CHAT parsing | tree-sitter parser + talkbank-model | **Superior** — incremental, streaming, error-recovering |
| CHAT serialization | talkbank-model serializer | **Equivalent** — 100% roundtrip on 340 reference files |
| Document editing | VS Code + CHAT language support | **Superior** — syntax highlighting, completions, hover, code actions |
| Format validation in editor | LSP real-time diagnostics | **Superior** — inline errors with quick fixes |
| TIERORDER | `chatter normalize` | **Equivalent** |
| CHAT to JSON | `chatter to-json` / `chatter from-json` | **New capability** (CLAN lacks this) |
| Dependency graph viz | VS Code extension graph panel | **New capability** |
| Corpus-wide validation | `chatter validate --roundtrip` (parallel) | **Superior** — orders of magnitude faster |
| MOR/POST (neural) | batchalign-core → Python models | **Different approach** — neural vs rule-based |
| Forced alignment | batchalign-core | **New capability** |
| Speech-to-text | talkbank-revai | **New capability** |

### 3.2 Partially Covered
| CLAN Feature | Current State | Gap |
|-------------|--------------|-----|
| FLO | Some FLO-like simplification exists in model | Not exposed as CLI command |
| FIXBULLETS | Some bullet repair in validation | Not a standalone tool |
| SEGMENT | batchalign-core has utterance segmentation | Not exposed in CLI |

### 3.3 Not Yet Implemented
| Category | Commands | Count |
|----------|----------|-------|
| Analysis commands | FREQ, MLU, MLT, KWAL, COMBO, DIST, CHAINS, COOCCUR, MAXWD, WDLEN, TIMEDUR, MODREP, VOCD, GEM/GEMFREQ/GEMLIST, KEYMAP, RELY, SCRIPT | 18 |
| Profiling commands | EVAL, EVAL-D, KIDEVAL, IPSYN, DSS, C-QPA, C-NNLA, FLUCALC, CORELEX, MORTABLE, SUGAR | 11 |
| Format converters | All 17 converters except JSON | 17 |
| Reformatting | CHSTRING, DATES, FLO, INDENT, LINES, LONGTIER, MEDIALINE, REPEAT, RETRACE, ROLES, SEGMENT, TRIM | 12 |
| Format repair | COMBTIER, DELIM, FIXIT, LOWCASE, QUOTES, CP2UTF | 6 |
| NLP | Rule-based MOR, Brill POST, MEGRASP | 3 |
| GUI | Audio/video playback, sonic mode, transcriber mode, walker, coder mode, KIDEVAL/EVAL dialogs, video linking, audio anonymization | 8+ |

---

## 4. Architecture Proposal

### 4.1 Analysis Engine: `talkbank-analysis` Crate

CLAN's CUTT framework has a simple plugin architecture: each tool implements `usage()`, `getflag()`, `init()`, and a main processing function that iterates over utterances. This maps perfectly to a Rust trait:

```rust
/// Every analysis command implements this trait
pub trait AnalysisCommand: Send + Sync {
    /// Command name (e.g., "freq", "mlu")
    fn name(&self) -> &str;

    /// Parse command-specific flags
    fn parse_args(&mut self, args: &AnalysisArgs) -> Result<()>;

    /// Process a single utterance (called for each utterance that passes filters)
    fn process_utterance(&mut self, utt: &Utterance, ctx: &AnalysisContext) -> Result<()>;

    /// Called between files
    fn file_boundary(&mut self, path: &Path) -> Result<()> { Ok(()) }

    /// Produce final output
    fn finalize(&mut self, output: &mut dyn Write) -> Result<()>;
}
```

The shared filtering infrastructure (speaker selection, tier selection, word inclusion/exclusion, gem limiting) can be implemented once in a shared `AnalysisRunner` that wraps any `AnalysisCommand`:

```rust
pub struct AnalysisRunner<C: AnalysisCommand> {
    command: C,
    speaker_filter: SpeakerFilter,  // +t, -t options
    word_filter: WordFilter,        // +s, -s options
    gem_filter: Option<GemFilter>,  // +g option
    file_filter: FileFilter,        // +f, -f options
    // ... other shared CUTT options
}
```

This is analogous to CLAN's `cutt.cpp` (17,926 lines) but would be far smaller in Rust thanks to the existing talkbank-model AST.

### 4.2 Priority-Ordered Analysis Commands

**Tier 1 — Core (most-used, clinically important):**
1. **FREQ** — Word frequency. Foundation for many other analyses.
2. **MLU** — Mean Length of Utterance. Most-cited measure in child language research.
3. **MLT** — Mean Length of Turn.
4. **KWAL** — Keyword search with context.
5. **COMBO** — Boolean search across tiers.
6. **KIDEVAL** — Clinical evaluation with normative database.

**Tier 2 — Profiling (clinical/research tools):**
7. **EVAL / EVAL-D** — Comprehensive language profile.
8. **IPSYN** — Index of Productive Syntax.
9. **DSS** — Developmental Sentence Scoring.
10. **VOCD** — Vocabulary diversity.
11. **FLUCALC** — Fluency analysis.
12. **C-QPA** — Quantitative Production Analysis.

**Tier 3 — Useful but less critical:**
13. **TIMEDUR** — Pause/overlap duration.
14. **CHAINS** — Interactional sequences.
15. **DIST** — Distance metrics.
16. **COOCCUR** — Co-occurrence tables.
17. **GEM/GEMFREQ/GEMLIST** — Gem-based analysis.
18. **RELY** — Inter-rater reliability.

**Tier 4 — Format tools (many are already partially covered):**
19. Format converters (CHAT2ELAN, ELAN2CHAT, etc.)
20. Reformatting commands (CHSTRING, FLO, DATES, etc.)
21. Format repair commands (FIXBULLETS, FIXIT, etc.)

### 4.3 VS Code Extension: GUI Features

#### Audio/Video Playback (highest priority GUI feature)
VS Code extensions can play audio/video via webview panels. The approach:

1. **Media Player Webview** — An HTML5 `<audio>`/`<video>` element in a VS Code webview panel. The extension controls playback position via the VS Code extension API <-> webview message passing.

2. **Bullet Synchronization** — When audio plays, the extension:
   - Tracks the current playback time via `timeupdate` events from the webview
   - Matches time to timing bullets in the CHAT file
   - Highlights the corresponding utterance in the text editor using `TextEditorDecorationType`
   - Scrolls the editor to keep the current utterance visible

3. **Click-to-Play** — Clicking a timing bullet in the editor sends a seek command to the media player webview, jumping to that audio position.

4. **Walker Mode** — A simplified playback mode where:
   - Pressing a key plays the next bullet segment
   - The editor highlights the current segment
   - Arrow keys navigate forward/backward through segments

This is fully achievable with VS Code's webview API. The `vscode-media-player` extension demonstrates the pattern. CLAN's AVController.mm is only ~900 lines; the VS Code equivalent would be comparable.

#### Transcriber Mode
A VS Code extension mode where:
- Audio plays continuously
- Pressing a hotkey inserts a timing bullet at the current playback position
- Tab/Enter advances to the next line
- Playback can be paused/rewound with foot pedal or keyboard shortcuts

This maps to VS Code's `TextEditorEdit` API + webview message passing.

#### Coder Mode
CLAN's coder mode lets researchers enter structured codes (e.g., speech acts, gesture types) by:
- Loading a codes definition file
- Presenting a dropdown of valid codes for each utterance
- Validating entered codes against the definition

In VS Code, this maps to:
- A TreeView panel showing available codes
- CompletionProvider that suggests valid codes when typing on coding tiers
- Diagnostic provider that validates codes against the definition file

#### Sonic Mode (Waveform Display)
This is the most technically challenging GUI feature. CLAN integrates with Praat for waveform display. Options:

1. **Praat Integration** — Launch Praat with the audio file and current position (same as CLAN's approach via `sendpraat`). Already feasible via VS Code's `Terminal` API.
2. **Embedded Waveform** — Use a JavaScript waveform library (wavesurfer.js) in a VS Code webview panel. This would provide a more integrated experience than CLAN's.

#### KIDEVAL/EVAL Dialogs
CLAN has specialized dialogs for KIDEVAL and EVAL that:
- Let the user select comparison databases
- Configure analysis parameters
- Display results in a formatted table

In VS Code, these become:
- Quick pick menus for parameter selection
- Webview panels for rich results display
- Or a simpler approach: CLI commands with structured JSON output rendered in a webview

### 4.4 NLP Subsystems

#### MOR (Morphological Analyzer)
CLAN's MOR is a rule-based system using lexicon tries, affix rules (arules.cpp, crules.cpp, drules.cpp), and language-specific grammars. Options:

1. **Keep the neural approach** — batchalign-core already produces MOR output via neural models (Stanza/spaCy). This is more accurate for most languages and requires no manual grammar maintenance.
2. **Port the rule-based engine** — Only needed if exact replication of CLAN MOR output is required for backward compatibility. The rule engine is ~7,000 lines of C++ and would be ~3,000-4,000 lines of Rust.
3. **Hybrid** — Use neural by default, fall back to rule-based for languages without neural models.

**Recommendation:** Keep neural as primary. Port rule-based MOR only if researchers require exact backward compatibility with existing CLAN MOR output for specific studies.

#### POST (POS Tagger)
CLAN's POST is a Brill tagger (~19,000 lines including training infrastructure). The neural models in batchalign-core already outperform it. **No need to port** unless backward compatibility is required.

#### MEGRASP (Grammar Parser)
Maximum entropy grammar/semantic role parser (~5,000 lines). Modern alternatives (spaCy dependency parsing via batchalign-core) are superior. **No need to port.**

---

## 5. Effort Estimates

### 5.1 Analysis Commands

| Category | Commands | Est. Rust LoC | Effort |
|----------|----------|--------------|--------|
| Shared analysis framework (replaces CUTT) | 1 module | ~2,000 | 2-3 weeks |
| FREQ (with all options) | 1 | ~1,500 | 2 weeks |
| MLU + MLT | 2 | ~800 | 1 week |
| KWAL + COMBO | 2 | ~1,200 | 1-2 weeks |
| Profiling commands (EVAL, KIDEVAL, IPSYN, DSS, etc.) | 10 | ~6,000 | 6-8 weeks |
| Other analysis commands (DIST, CHAINS, etc.) | 10 | ~3,000 | 3-4 weeks |
| VOCD | 1 | ~1,500 | 1-2 weeks |
| Format converters | 17 | ~4,000 | 4-5 weeks |
| Reformatting/repair commands | 19 | ~2,000 | 2-3 weeks |
| **Subtotal** | **~60** | **~22,000** | **~25 weeks** |

Note: The Rust implementations will be significantly smaller than CLAN's C++ because:
- The talkbank-model AST eliminates all manual CHAT parsing (~17,000 lines of CUTT)
- Rust iterators/pattern matching replace verbose C string manipulation
- Error handling is already infrastructure (miette, thiserror)
- The existing parser gives us a typed AST to work with, rather than raw text

### 5.2 VS Code Extension Features

| Feature | Est. LoC (TypeScript) | Effort |
|---------|----------------------|--------|
| Audio/video player webview | ~1,500 | 2-3 weeks |
| Bullet synchronization | ~800 | 1-2 weeks |
| Transcriber mode | ~600 | 1 week |
| Walker mode | ~400 | 1 week |
| Coder mode | ~800 | 1-2 weeks |
| Waveform display (wavesurfer.js) | ~1,200 | 2-3 weeks |
| KIDEVAL/EVAL results panels | ~1,000 | 1-2 weeks |
| Analysis command integration (run from VS Code) | ~600 | 1 week |
| Video linking tools | ~500 | 1 week |
| Audio anonymization | ~400 | 1 week |
| **Subtotal** | **~7,800** | **~15 weeks** |

### 5.3 Total Estimated Effort

| Phase | Effort |
|-------|--------|
| Analysis framework + Tier 1 commands | ~8 weeks |
| Tier 2 profiling commands | ~8 weeks |
| VS Code media features | ~10 weeks |
| Format converters | ~5 weeks |
| Remaining analysis + reformatting | ~6 weeks |
| Testing, validation, documentation | ~8 weeks |
| **Total** | **~45 weeks (~11 months)** |

This assumes one experienced Rust developer working full-time, with domain expert guidance on linguistic conventions.

---

## 6. Key Technical Decisions

### 6.1 Where Analysis Tools Run

**Option A: All in Rust CLI, VS Code calls CLI** (Recommended)
- Analysis commands are `chatter freq`, `chatter mlu`, etc.
- VS Code extension invokes CLI and displays results
- Works headless for batch processing, CI, scripting
- Same tools work from terminal and from VS Code

**Option B: Analysis as LSP commands**
- LSP provides `workspace/executeCommand` for running analyses
- More tightly integrated but couples analysis to editor
- Bad for batch/scripting use cases

**Recommendation:** Option A. Analysis commands should be CLI-first, with VS Code as a thin UI layer.

### 6.2 Output Format Compatibility

CLAN commands produce text output with specific formatting conventions that existing scripts and workflows depend on. Options:

1. **Exact CLAN output compatibility** — Match CLAN's text output format character-for-character. Required for backward compatibility with existing user scripts.
2. **Structured output with CLAN compat mode** — Default to JSON/structured output, with `--clan-compat` flag for legacy format.
3. **New format only** — Break compatibility, provide migration guide.

**Recommendation:** Option 2. Default to structured output (JSON, CSV) but provide `--clan-compat` or `--format clan` for exact backward compatibility. This lets existing scripts keep working while new workflows use better formats.

### 6.3 Normative Databases (KIDEVAL, EVAL)

KIDEVAL and EVAL compare a child's language to normative databases. These databases are currently bundled with CLAN in `lib/kideval/` and `lib/eval/`. They need to be:

1. Bundled with the Rust binary (embedded via `include_bytes!` or similar)
2. Or distributed as a separate data package that the CLI locates at runtime
3. Or served from TalkBank servers (requires network access)

**Recommendation:** Embed the databases in the binary for offline use, with optional network update mechanism.

### 6.4 Rule Files (IPSYN, DSS)

IPSYN and DSS use rule files (in `lib/ipsyn/` and `lib/dss/`) that define scoring criteria per language. These need to be preserved and loaded by the Rust implementations. They are small text files that can be embedded or distributed alongside the binary.

---

## 7. Advantages of the New System Over CLAN

1. **Cross-platform from day one** — Rust binary + VS Code works on macOS, Windows, Linux. CLAN GUI is macOS-only; Unix-CLAN is CLI-only with no GUI.

2. **Dramatically faster** — Parallel validation already processes 95,000+ files. Analysis commands can similarly parallelize across files with crossbeam.

3. **Modern error reporting** — miette-based diagnostics with source spans, colors, suggested fixes. CLAN errors are terse text messages.

4. **Structured output** — JSON/CSV output enables programmatic analysis, Jupyter integration, R/Python pipelines. CLAN only produces text.

5. **Incremental parsing** — Tree-sitter enables real-time analysis as the user types, rather than save-then-run.

6. **Maintainable** — Rust's type system, testing infrastructure, and documentation tools make the codebase approachable for new contributors. CLAN's 215K lines of pre-ANSI-era C++ with `#define main(x,y) _main(x,y)` macros are virtually unmaintainable.

7. **Extensible** — The trait-based analysis framework makes it straightforward to add new commands. CLAN requires modifying the CUTT framework and rebuilding the monolithic binary.

8. **VS Code ecosystem** — Users get Git integration, remote development, Copilot, extensions for data visualization, Jupyter notebooks — all for free.

9. **Testable** — Each analysis command can have comprehensive unit tests with known-good CHAT inputs and expected outputs, validated against CLAN's output for backward compatibility.

---

## 8. Risks and Mitigations

### 8.1 Domain Fidelity
**Risk:** Each CLAN command encodes subtle linguistic conventions developed over 40+ years. Reimplementing them incorrectly would produce wrong research results.

**Mitigation:**
- Create a comprehensive test suite by running every CLAN command on reference corpus files and capturing output
- Compare Rust output against CLAN output character-by-character
- Have domain experts (Brian MacWhinney and team) review each command's implementation
- Preserve CLAN as a reference implementation during the transition

### 8.2 Audio/Video in VS Code
**Risk:** VS Code's webview API may have latency or capability limitations compared to native AVPlayer.

**Mitigation:**
- Prototype the media player webview early to validate performance
- For professional transcription workflows requiring <10ms latency, consider a companion Electron app or Tauri app that communicates with the VS Code extension
- wavesurfer.js has been proven in production transcription tools (e.g., Descript)

### 8.3 KIDEVAL/EVAL Normative Databases
**Risk:** The normative databases represent decades of curated clinical data. Incorrect loading or computation would produce wrong clinical scores.

**Mitigation:**
- Extract databases from CLAN's `lib/` directory and convert to a well-documented format (JSON or SQLite)
- Validate every computation against CLAN's output for the same inputs
- Maintain identical scoring algorithms (these are well-documented in published papers)

### 8.4 User Adoption
**Risk:** CLAN users (primarily linguists and speech-language pathologists) are accustomed to CLAN's interface. VS Code may feel unfamiliar.

**Mitigation:**
- Provide guided tutorials and extension pack with recommended settings
- Create a "CLAN mode" that mimics key CLAN workflows
- Maintain CLI compatibility so terminal-oriented users can switch immediately
- Offer a transition period where both CLAN and the new tools are available

### 8.5 Scope Creep
**Risk:** Trying to replicate every CLAN feature at once leads to an unfinishable project.

**Mitigation:**
- Strict phased approach: Tier 1 commands first, then media features, then remaining commands
- Each phase produces a usable, shippable increment
- Commands not yet ported can still be run via the existing CLAN binary (the VS Code extension could even wrap CLAN commands during transition)

---

## 9. Recommended Phased Approach

### Phase 1: Analysis Framework + Core Commands (Months 1-3)
- Build `talkbank-analysis` crate with shared analysis runner
- Implement FREQ, MLU, MLT, KWAL, COMBO
- Add `chatter analyze <command>` CLI interface
- Validate output against CLAN for reference corpus
- **Milestone:** Researchers can run the 5 most common analyses from CLI

### Phase 2: Clinical Profiling (Months 3-5)
- Implement KIDEVAL, EVAL/EVAL-D, IPSYN, DSS
- Port normative databases and rule files
- Add comparison database support
- **Milestone:** Clinicians can generate standardized profiles

### Phase 3: Media Integration (Months 5-8)
- Build audio/video player webview in VS Code extension
- Implement bullet synchronization and click-to-play
- Add transcriber mode and walker mode
- Implement waveform display (wavesurfer.js)
- **Milestone:** Transcription workflow fully operational in VS Code

### Phase 4: Remaining Commands + Format Tools (Months 8-10)
- Implement remaining analysis commands (VOCD, TIMEDUR, CHAINS, etc.)
- Implement format converters (CHAT2ELAN, ELAN2CHAT, etc.)
- Implement reformatting and repair commands
- **Milestone:** Feature parity with CLAN CLI

### Phase 5: Polish + Advanced Features (Months 10-12)
- Coder mode in VS Code
- KIDEVAL/EVAL results panels in webviews
- Audio anonymization
- Sonic mode (Praat integration or embedded waveform)
- Comprehensive documentation and migration guides
- **Milestone:** Full CLAN replacement ready for general use

---

## 10. Transition Strategy

### 10.1 During Development
- Keep CLAN available as the reference implementation
- VS Code extension can offer "Run in CLAN" fallback for unimplemented commands (already have `send2clan-sys` FFI)
- Build comparison test suite that runs both CLAN and Rust on same inputs

### 10.2 At Launch
- Announce new tools as "CLAN Next" or similar
- Provide side-by-side installation
- Publish migration guide mapping CLAN commands to new equivalents
- Offer workshops at TalkBank conferences

### 10.3 Sunsetting CLAN
- Once all commands are validated against CLAN output on reference corpus
- And user community has adopted the new tools
- CLAN can be archived as a historical reference

---

## 11. What We Can Reuse from CLAN

While a clean Rust rewrite is preferred, some CLAN artifacts are valuable:

1. **Rule files** (IPSYN rules, DSS rules, MOR grammars) — These are data files, not code. Copy directly.
2. **Normative databases** (KIDEVAL, EVAL) — Convert from CLAN's format to JSON/SQLite.
3. **Test expectations** — Run CLAN on reference corpus to generate expected output for each command. Use as golden test data.
4. **Algorithm documentation** — The CLAN manual describes each algorithm in detail. Use as specification.
5. **CUTT option conventions** — The shared command-line option syntax (+t, -t, +s, -s, etc.) should be preserved for user familiarity.

---

## 12. Conclusion

Replacing CLAN is feasible, desirable, and timely. The talkbank-tools project has already solved the hardest problems (parsing, validation, AST model, caching, VS Code integration, LSP). The remaining work — analysis commands and media features — is large in scope but straightforward in nature. Each CLAN command is a well-defined statistical or search operation over the CHAT AST that talkbank-tools already provides.

The key success factor is **domain fidelity**: ensuring that every number, every edge case, every linguistic convention is replicated exactly. This requires close collaboration with domain experts and comprehensive comparison testing against CLAN's output.

The result will be a modern, cross-platform, maintainable, and extensible replacement that serves the TalkBank community for decades to come — far outlasting any single developer's career.
