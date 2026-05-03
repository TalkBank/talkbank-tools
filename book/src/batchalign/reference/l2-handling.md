# L2 and Language Switching

**Status:** Current behavior reference
**Last updated:** 2026-04-15 20:32 EDT

## Current behavior

Batchalign distinguishes between:

- utterance-level language directives
- word-level language markers such as `@s` and `@s:lang`

L2 dispatch is on by default. The legacy `L2|xxx` behavior is
available via the `--no-l2-morphotag` opt-out.

## `%mor` behavior A: real morphology via secondary dispatch (default)

For `@s` or `@s:lang` words, the pipeline routes each word to a
secondary-language Stanza model and merges the response with the
primary model's structural analysis. The `%mor` tier carries real
POS/lemma/features:

```
# Default
%mor:  ... adp|auf noun|film noun|study-Plur .
```

## `%mor` behavior B: conservative `L2|xxx` (opt-out)

With `--no-l2-morphotag`, `@s` words are blanked:

- the word is recognized as foreign/code-switched
- `%mor` output is `L2|xxx`
- no lexical/morphological analysis is preserved inside `%mor`

This is the legacy behavior researchers cite in work published
before the L2 morphotag feature landed, and remains the honest
fallback when a secondary Stanza model is known to be weak.

```
# --no-l2-morphotag
%mor:  ... adp|auf L2|xxx L2|xxx .
```

Validated at scale: **99.96% dispatch rate** (16,838 / 16,845 `@s`
words successfully routed to a secondary-language Stanza model;
7 fell back to `L2|xxx`) across 19 language pairs in the
`l2-eval-runs/2026-04-15/per-pair.csv` aggregate eval.
Contractions expand correctly (`it's@s:eng` →
`pron|it~aux|be`), phrasal verbs are recognized (`wake up@s` →
`verb|wake part|up`). See
[L2 Morphotag: Per-Word Code-Switching Analysis](l2-morphotag.md)
for the full design and merge algorithm.

## Utterance-level versus word-level behavior

### Utterance-level

Utterance-level language directives affect utterance handling and routing
boundaries.

### Word-level

Word-level language markers identify foreign/code-switched words, but do not
currently trigger full per-word language-specific morphosyntax routing.

## Current limit

The parsed word-level language information is not currently used to route each
marked word through a separate language-specific NLP pipeline.

So the current public boundary is:

- preserve that the word is foreign/code-switched
- avoid claiming full morphology for it

## Related references

- [L2 Morphotag: Per-Word Code-Switching Analysis](l2-morphotag.md)
  — full design, merge algorithm, phrasal-verb diagram
- [Language Routing](../../architecture/language-and-multilingual/language-routing.md) — per-utterance + per-word routing, auto-detection
- [Language Data Model](language-handling.md)
