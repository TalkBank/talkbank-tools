# Summary

[Introduction](introduction.md)

---

# Getting Started

- [Installation](getting-started/installation.md)
- [Quick Start](getting-started/quick-start.md)
- [Migrating from CLAN](getting-started/migration.md)
- [Flag Translation Guide](getting-started/flag-translation.md)

# User Guide

- [Running Commands](user-guide/running-commands.md)
- [Filtering](user-guide/filtering.md)
  - [Speaker Filtering](user-guide/filtering-speakers.md)
  - [Tier Filtering](user-guide/filtering-tiers.md)
  - [Word Filtering](user-guide/filtering-words.md)
  - [Gem Filtering](user-guide/filtering-gems.md)
  - [Utterance Range](user-guide/filtering-range.md)
- [Output Formats](user-guide/output-formats.md)
- [Working with Directories](user-guide/directories.md)
- [Pipelines and Scripting](user-guide/pipelines.md)

# Validation

- [CHECK — CHAT File Validation](commands/check.md)

# Analysis Commands

- [FREQ — Word Frequency](commands/freq.md)
- [MLU — Mean Length of Utterance](commands/mlu.md)
- [MLT — Mean Length of Turn](commands/mlt.md)
- [VOCD — Vocabulary Diversity](commands/vocd.md)
- [DSS — Developmental Sentence Scoring](commands/dss.md)
- [EVAL — Language Sample Evaluation](commands/eval.md)
- [EVAL-D — Evaluation (DementiaBank)](commands/eval-d.md)
- [KIDEVAL — Child Language Evaluation](commands/kideval.md)
- [IPSYN — Index of Productive Syntax](commands/ipsyn.md)
- [FLUCALC — Fluency Calculation](commands/flucalc.md)
- [SUGAR — Grammatical Analysis](commands/sugar.md)
- [MORTABLE — Morphology Tables](commands/mortable.md)
- [KWAL — Keyword and Line](commands/kwal.md)
- [COMBO — Boolean Search](commands/combo.md)
- [CODES — Code Frequency](commands/codes.md)
- [COMPLEXITY — Syntactic Complexity](commands/complexity.md)
- [CORELEX — Core Vocabulary](commands/corelex.md)
- [CHAINS — Clause Chain Analysis](commands/chains.md)
- [CHIP — Interaction Profile](commands/chip.md)
- [COOCCUR — Word Co-occurrence](commands/cooccur.md)
- [DIST — Word Distribution](commands/dist.md)
- [FREQPOS — Positional Frequency](commands/freqpos.md)
- [GEMLIST — Gem Segments](commands/gemlist.md)
- [KEYMAP — Contingency Tables](commands/keymap.md)
- [MAXWD — Longest Words](commands/maxwd.md)
- [MODREP — Model/Replica Comparison](commands/modrep.md)
- [PHONFREQ — Phonological Frequency](commands/phonfreq.md)
- [RELY — Inter-rater Agreement](commands/rely.md)
- [SCRIPT — Template Comparison](commands/script.md)
- [TIMEDUR — Time Duration](commands/timedur.md)
- [TRNFIX — Tier Comparison](commands/trnfix.md)
- [UNIQ — Repeated Utterances](commands/uniq.md)
- [WDLEN — Word Length Distribution](commands/wdlen.md)
- [WDSIZE — Word Size Distribution](commands/wdsize.md)

# Transform Commands

- [CHSTRING — String Replacement](commands/chstring.md)
- [COMBTIER — Combine Tiers](commands/combtier.md)
- [COMPOUND — Compound Normalization](commands/compound.md)
- [DATACLEAN — Format Cleanup](commands/dataclean.md)
- [DATES — Age Computation](commands/dates.md)
- [DELIM — Add Terminators](commands/delim.md)
- [FIXIT — Normalize Formatting](commands/fixit.md)
- [FIXBULLETS — Timing Repair](commands/fixbullets.md)
- [FLO — Fluent Output](commands/flo.md)
- [GEM — Gem Extraction](commands/gem.md)
- [INDENT — Align Overlap Markers](commands/indent.md)
- [LINES — Line Numbers](commands/lines.md)
- [LONGTIER — Remove Line Wrapping](commands/longtier.md)
- [LOWCASE — Lowercase](commands/lowcase.md)
- [MAKEMOD — Model Tier](commands/makemod.md)
- [ORT — Orthographic Conversion](commands/ort.md)
- [POSTMORTEM — Mor Post-processing](commands/postmortem.md)
- [QUOTES — Extract Quotes](commands/quotes.md)
- [REPEAT — Mark Repetitions](commands/repeat.md)
- [RETRACE — Retrace Tier](commands/retrace.md)
- [ROLES — Rename Speakers](commands/roles.md)
- [TIERORDER — Reorder Tiers](commands/tierorder.md)
- [TRIM — Remove Dependent Tiers](commands/trim.md)

# Format Converters

- [CHAT2TEXT — CHAT to Plain Text](commands/chat2text.md)
- [CHAT2ELAN — CHAT to ELAN](commands/chat2elan.md)
- [CHAT2PRAAT — CHAT to Praat TextGrid](commands/chat2praat.md)
- [CHAT2SRT — CHAT to Subtitles](commands/chat2srt.md)
- [ELAN2CHAT — ELAN to CHAT](commands/elan2chat.md)
- [LAB2CHAT — LAB to CHAT](commands/lab2chat.md)
- [LENA2CHAT — LENA to CHAT](commands/lena2chat.md)
- [LIPP2CHAT — LIPP to CHAT](commands/lipp2chat.md)
- [PLAY2CHAT — PLAY to CHAT](commands/play2chat.md)
- [PRAAT2CHAT — Praat TextGrid](commands/praat2chat.md)
- [RTF2CHAT — Rich Text to CHAT](commands/rtf2chat.md)
- [SALT2CHAT — SALT to CHAT](commands/salt2chat.md)
- [SRT2CHAT — Subtitles to CHAT](commands/srt2chat.md)
- [TEXT2CHAT — Plain Text to CHAT](commands/text2chat.md)

# Architecture & Developer Guide

- [Design Philosophy](architecture/philosophy.md)
- [Framework](architecture/framework.md)
- [Adding a Command](developer/adding-command.md)
- [Current Architecture Seams](developer/architecture-seams.md)
- [Testing Strategy](developer/testing.md)
- [Golden Tests](developer/golden-tests.md)
- [Coding Standards](developer/coding-standards.md)

# Divergences & Migration

- [Why We Diverge](divergences/philosophy.md)
- [Framework-Level Divergences](divergences/framework.md)
- [Per-Command Divergences](divergences/per-command.md)
- [CHECK vs chatter validate](divergences/check-vs-validate.md)
- [Commands Not Ported](divergences/not-ported.md)
- [Command Parity Audit](divergences/command-parity-audit.md)
- [Parity Plan](divergences/parity-plan.md)

# Appendices

- [Flag Translation Guide](appendices/flag-mapping.md)
- [CLAN Manual Audit](appendices/clan-manual-audit.md)
- [Command Status Matrix](appendices/status-matrix.md)
- [Dependent Tier Semantics](appendices/dependent-tier-semantics.md)
- [Transform Taxonomy](appendices/transform-taxonomy.md)
- [Glossary](appendices/glossary.md)
- [Improvements Log](appendices/improvements.md)
- [Replacement Analysis](appendices/replacement-analysis.md)
- [Text Hacking Audit](appendices/text-hacking-audit.md)
