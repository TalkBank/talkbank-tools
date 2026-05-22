# Framework-Level Divergences

**Status:** Current
**Last updated:** 2026-05-21 23:38 EDT

These differences apply across all commands, not to specific ones.

## CLI syntax

| Aspect | Legacy CLAN | Rust CLAN |
|--------|-------------|-----------|
| Flag style | `+flag` / `-flag` | `--flag` (both styles accepted) |
| Invocation | `freq file.cha` | `chatter clan freq file.cha` |
| Directory | Manual file listing or `*.cha` | Recursive directory traversal |
| Output format | Text only | Text, JSON, CSV |
| Error output | Simple messages to stderr | Rich diagnostics with file location and context |

Both flag styles are accepted — `+t*CHI` and `--speaker CHI` are equivalent. The automatic rewriting happens in `clan_args::rewrite_clan_args()` before argument parsing.

## Output formatting

- **File headers**: Our headers include the full file path; CLAN uses basename only in some commands
- **Speaker ordering**: CLAN outputs speakers in reverse encounter order (C linked-list prepend pattern). We replicate this behavior for parity.
- **Encoding**: UTF-8 only. CLAN supports legacy encodings via CP2UTF.
- **Line wrapping**: Our text output does not wrap long lines. CLAN wraps at ~80 characters in some commands.

## Counting semantics

These counting rules were discovered during golden test parity verification and apply across multiple commands:

| Rule | Commands affected | Details |
|------|-------------------|---------|
| Population SD | MLU, MLT | Uses `/ n`, not `/ n-1` (sample SD) |
| Brown's morphemes | MLU, WDLEN | Only 7 suffix strings count: `PL`, `PAST`, `Past`, `POSS`, `PASTP`, `Pastp`, `PRESP` |
| Fusional features | VOCD, MLU | `&PRES`, `&INF` etc. are part of the lemma; strip with `split('&')` |
| Apostrophe stripping | WDLEN | Characters counted after removing apostrophes |
| Turn = utterance | DIST | Every utterance is its own turn (no speaker-continuity grouping) |

## AST-based processing

The most fundamental divergence: CLAN processes CHAT files as text using string pattern matching. The Rust reimplementation parses files into a typed AST and operates on structured data. This means:

- Word classification uses typed fields (`word.category`, `word.untranscribed()`) instead of string prefixes (`starts_with('&')`)
- Annotations are structured objects, not regex-extracted substrings
- Transform commands modify AST nodes, then serialize — guaranteeing structural validity

This eliminates classes of bugs where string patterns match unintended content, but occasionally produces different whitespace or formatting in edge cases.

## CLAN-bug divergences (chatter improves on CLAN)

The CLAN parity contract is byte-level output equivalence on file-arg
invocations. Within that contract, the parity work has surfaced
behaviours where CLAN's output is internally inconsistent or
inadvertently doubled — visible plumbing artifacts rather than
intended product. `chatter clan` preserves the *intended* behaviour
and documents the divergence here rather than reproducing the
artifact.

Each entry below names the divergence, CLAN's behaviour, chatter's
behaviour, the source-grounded reason chatter is correct, and the
ledger row where it is tracked.

### Banner duplication

CLAN's `cutt.cpp` mainloop emits the six-line banner block twice in:

* any invocation that reads from `stdin` (`freq < file.cha`),
* any invocation passing more than one positional file argument
  (`freq a.cha b.cha`),
* and arguably others — the `FirstTime` branch fires whenever
  CLAN's input committer reaches the scratch-file boundary, which
  is many call sites.

Single-file invocations (`freq file.cha`) emit the banner once.

- **CLAN behaviour (when duplicating):** banner block printed twice,
  byte-identical except for an extra tab-prefixed echo on the
  second block's line 1.
- **chatter behaviour:** banner block printed once on every
  invocation regardless of source or arity.
- **Reason chatter is correct:** the duplication is internal pipeline
  plumbing exposed by the `FirstTime` + scratch-file-commit ordering
  in `cutt.cpp`. No downstream consumer parses it as semantic content;
  researchers' scripts that anchor on the `****` separator find one
  separator and proceed. Doubling the block would be visual noise
  with no information value.
- **Ledger row:** CLAN-DIV-001 (banner duplication).

If a researcher's script breaks because it relied on seeing exactly
two banner blocks, that script is parsing CLAN's plumbing as content
and needs to be updated; reach out and we will help.

### Source line: always "From pipe input" in CLAN

CLAN's banner emits the literal string `From pipe input` on line 6
in **every** invocation — regardless of whether the input came from
stdin, a single file-arg, or multiple file-args. The label is
misleading: `freq file.cha` (with no `<` redirection) still prints
`From pipe input`.

- **CLAN behaviour:** always `From pipe input` regardless of source.
- **chatter behaviour:** `From file <basename>` when a file is the
  source (aggregated mode uses the first file; per-file mode uses
  each file's basename).
- **Reason chatter is correct:** the file name carries information
  a researcher inspecting CLAN output may want. CLAN's hardcoded
  `From pipe input` was a poor design choice — the only time it is
  technically accurate is for `<` redirection, which is one of
  several supported invocation shapes. chatter's label honestly
  reports the source.
- **Ledger row:** CLAN-DIV-002 (source line).

Migration note: scripts that grep for the literal `From pipe input`
will need to adjust to `From file <…>` when running chatter.
Scripts that anchor on the `****` separator (the more common
pattern) are unaffected.
