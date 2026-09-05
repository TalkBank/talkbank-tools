# `@Options` and Per-File Command Scoping

**Status:** Current
**Last updated:** 2026-09-05 04:22 EDT

CHAT files can carry an `@Options:` header that scopes which
batchalign3 commands are allowed to run on them. The two values
batchalign3 reads are `CA` and `NoAlign`. They are **independent**
directives, each scoped to a specific command, they are not a
generic "skip everything" flag.

## `@Options: CA`: skip morphotag

A Conversation Analysis transcript. By default, `@Options: CA` means
"the morphotag command is not to be run on this file." CA
transcripts use a separate convention for prosodic and discourse
annotation that does not benefit from automatic POS / dependency
analysis, and any %mor / %gra written by morphotag would be at best
noise.

batchalign3's behavior on a `@Options: CA` file:

- `morphotag`: pass-through. The file's existing %mor / %gra (if
  any) is preserved unchanged. No Stanza inference. No provenance
  comment is added.
- All other commands, run normally.

Corpus reconstruction can make a different, explicit choice with
`--ca-policy analyze`. That policy runs morphotag while preserving the
`@Options: CA` header. It exists for a documented source-to-publication
rebuild where automatic morphology is part of the intended edition, such as
reconstructing MICASE material whose published derivative already carries
`%mor`. The default remains `honor`; a routine invocation never overrides the
transcript declaration silently.

## `@Options: NoAlign`: skip the `align` command

**`@Options: NoAlign` literally means "the `align` command in
batchalign3 is not to run on this file."** NoAlign is scoped to
forced alignment specifically. The directive came from the era when
audio bullets in CHAT could be word-level or utterance-level; a
NoAlign file declares that its audio bullets cover whole utterances
and should not be re-aligned to individual word boundaries by FA.

batchalign3's behavior on a `@Options: NoAlign` file:

- `align`: pass-through. No Whisper, no DP alignment, no rewriting
  of timing markers.
- `morphotag`: runs normally. NoAlign has nothing to do with the
  text-level morphological analysis that morphotag performs. (Prior
  to 2026-05-07, the morphotag pipeline incorrectly conflated
  NoAlign with a global skip; this caused 297 corpus files to
  accumulate stale `%mor` / `%gra` from old buggy runs with no
  rerun path. The fix restores the orthogonal scoping the directive
  was always meant to have.)
- All other commands, run normally.

## Combining `CA` and `NoAlign`

A file may carry both: `@Options: CA, NoAlign`. Under the default CA policy,
the directives remain orthogonal: `CA` skips `morphotag`, `NoAlign` skips
`align`, and any other command runs. `--ca-policy analyze` changes only the
morphotag decision; `NoAlign` continues to govern alignment independently.

## What the directives are NOT

They are not a generic "this file is special, leave it alone" flag.
They are not a substitute for setting `--lang` or `--skip-*` flags.
They scope batchalign3 commands by name, deliberately, per the
CHAT manual's per-file convention. New batchalign3 commands should
not extend or repurpose these directives without an explicit
specification update.

## Related

- CHAT manual: <https://talkbank.org/0info/manuals/CHAT.html>
  (search for "Options" in the headers reference).
- `crates/batchalign/src/pipeline/morphosyntax.rs`, where the submitted
  `CaMorphotagPolicy` and parsed header produce one typed per-file
  disposition consumed by all downstream stages.
- `crates/batchalign/src/fa/mod.rs`: the `align`-side gate
  (`is_no_align`).
