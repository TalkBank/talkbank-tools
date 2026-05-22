# POST -- POS Disambiguation (deliberately not implemented)

**Status:** Reference -- stub command
**Last updated:** 2026-05-21 15:30 EDT

## Purpose

In legacy CLAN, `post` disambiguates the multiple part-of-speech
candidates that `mor` emits on a `%mor` tier, choosing the most
likely category for each token based on a trained model and a
context-sensitive rule database.

This project does **not** implement `post`. It is part of the legacy
CLAN MOR/POST grammar pipeline ([see `mor`](mor.md)) and depends on
the same lexicon and rule artifacts that we deliberately do not port.

## What to use instead

Use the [batchalign](../../batchalign/introduction.md) morphotag
pipeline. Stanza's neural models produce a single disambiguated POS
per token at inference time, so a separate post-pass is not needed.

## Behavior

Invoking `chatter clan post` prints an error and exits non-zero. No
CHAT files are modified.

## See also

- [MOR](mor.md) -- the upstream morphological analyzer (also a stub)
- [POSTLIST](postlist.md), [POSTMODRULES](postmodrules.md),
  [POSTTRAIN](posttrain.md) -- companion commands, all stubs
- [POSTMORTEM](postmortem.md) -- the `%mor`-tier post-processing
  command that **is** implemented
