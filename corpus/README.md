# Reference Corpus

## Overview

The reference corpus at `corpus/reference/` is the 100%-pass quality
gate for all parser/grammar changes. Both parsers — the canonical
tree-sitter parser (`talkbank-parser`) and the alternate re2c parser
(`talkbank-parser-re2c`, used as a specification oracle and
performance parser) — must agree on every file. Every file is
self-describing with `@Comment:` headers explaining what it
demonstrates.

## Structure

`corpus/reference/` is organized into subdirectories by what each
group of files demonstrates:

- `core/` — document structure, headers, metadata
- `content/` — main tier: words, terminators, linkers, pauses
- `annotation/` — brackets, retraces, groups, scoping
- `tiers/` — dependent tiers: `%mor`, `%gra`, `%pho`, `%wor`, etc.
- `ca/` — conversation analysis: overlaps, intonation
- `audio/` — audio-linked files with `%wor` plus the matching `.mp3`s
- `languages/` — one conversation per language, morphotagged with
  `%mor`/`%gra`
- `edge-cases/` — boundary and corner-case constructs
- `word-features/` — feature-focused word-level fixtures

The live file counts and node-coverage status are recomputed on every
`make verify` / `make coverage` run; check those for current numbers.

## Validation

```bash
make verify                    # All pre-merge gates
make coverage                  # Node coverage check
cargo run --release -p talkbank-cli -- validate corpus/reference/ --roundtrip --force
```

## Key Policies

- Every file in `corpus/reference/` MUST pass parser equivalence
  between tree-sitter and re2c, and roundtrip validation.
- If a grammar/parser change breaks even one file, revert
  immediately.
- Every file has `@Comment:` headers explaining its purpose and
  constructs.
- Language files have fresh `%mor`/`%gra` from batchalign3 morphotag.
- Never hand-edit generated artifacts.

## See Also

- The repo-root CLAUDE.md
- `crates/talkbank-parser-tests/` — the equivalence-test harness

---
Last Updated: 2026-05-21
