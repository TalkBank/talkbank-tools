# Stanza Limitations and Their Workarounds

**Status:** Current
**Last updated:** 2026-07-28 09:07 EDT

Stanza is the morphosyntax engine behind `batchalign3 morphotag`. It does not
guarantee Universal Dependencies conformance, and several of its language
models misbehave on the short, single-word utterances that dominate
child-language corpora. This page records every limitation we have found, the
evidence for it, and what the pipeline does about it.

**The governing rule: never trust Stanza's output shape.** Everything it
returns crosses a validation boundary before anything downstream sees it. That
boundary is `validate_ud_words()` in `batchalign/inference/morphosyntax.py`,
called from `batch_infer_morphosyntax` immediately after `doc.to_dict()`.

## Why that boundary is load-bearing (2026-07-28 incident)

The validators existed and were unit-tested for months, and **nothing on the
production path ever called them**. `batch_infer_morphosyntax` took
`doc.to_dict()` straight into the response. The consequences reached published
data:

- `<PAD>` sanitization existed, yet bare `PAD` appears as a `%gra` relation in
  the corpora.
- Non-UD `iob` flowed into `%gra` as `IOB` across many files.

Neither was noticed because nothing checked relations on either side: CLAN
CHECK does not validate `%gra` at all, and chatter only gained the rule (E761)
in v0.4.0, which is what finally surfaced it.

The generalizable lesson, which cost real corpus damage: **a validator that its
own unit tests exercise but no production caller invokes is worse than no
validator, because it produces confident false assurance.** Tests for this
boundary must drive `batch_infer_morphosyntax`, not `UdWord` in isolation.

## Limitation 1: non-UD dependency relations

**Symptom.** Stanza's Italian model emits `deprel="iob"`. Universal
Dependencies has no such relation; the label is `iobj`.

**Evidence** (stanza 1.13.0, verified 2026-07-28):

```
'attenzione .'   id=2 text='ne'  upos=PRON  head=1  deprel='iob'
```

**Impact.** `2|1|IOB` written into `%gra` across published corpora, oldest
observed provenance `ba3 morphotag | engine=stanza-1.11.1` (2026-05-08).

**Workaround.** `UdWord._normalize_deprel_to_ud` checks the relation HEAD
against `UD_RELATIONS` (the 37 UD v2 relations), maps known aliases via
`UD_DEPREL_ALIASES` (`iob` -> `iobj`), and degrades anything unrecognized to
`dep` with a warning.

Only the HEAD is a closed set. UD defines SUBTYPES as open and
language-specific, and the corpora legitimately use many (`nmod:poss`,
`acl:relcl`, `flat:foreign`), so the subtype is preserved verbatim. This
mirrors exactly what chatter's E761 enforces on the reading side; **the two
vocabularies must not drift apart.**

## Limitation 2: Italian MWT destroys single-word `-ne` nouns

**Symptom.** In ISOLATION, every tested Italian noun ending in `-ne` is split
by the MWT processor into a nonexistent verb plus the clitic `ne`.

**Evidence** (stanza 1.13.0, 2026-07-28), 10/10 mis-split:

```
attenzione  -> ('attenzi', VERB, root)  ('ne', PRON, iob)
stazione    -> ('stazio',  VERB, root)  ('ne', PRON, iob)
canzone     -> ('canzo',   VERB, root)  ('ne', PRON, iob)
persone     -> ('perso',   VERB, root)  ('ne', PRON, iob)
opinione, ragione, lezione, situazione, televisione, macchine: same shape
```

**In sentence context the same words are analyzed correctly**, so this is
specific to short utterances:

```
mi piace questa canzone .    -> canzone intact, no iob
ci sono molte persone qui .  -> persone intact, no iob
```

**Why this matters here more than elsewhere.** Single-word utterances are the
norm in child speech (`*CHI:\tmacchine .`), so a defect that only bites short
utterances bites CHILDES hardest. The damage is not limited to the relation
label: `%mor` records `verb|attenzare` for the noun *attenzione*, inventing a
verb that does not exist in Italian.

**Status: FIXED 2026-07-28**, in `batchalign/inference/_italian_mwt.py`.

Italian multi-word tokens are very nearly a CLOSED class, unlike Stanza's
treatment of them: preposition+article contractions and `ecco`+enclitic are
both fully enumerable; only verb+enclitic is genuinely open. So for
single-word utterances, where the damage occurs, expansion is suppressed
(Stanza's documented `(text, False)` hint) EXCEPT for the closed classes,
which still expand.

Measured before and after on the real corpus words, via the full pipeline:

| utterance | before | after |
|---|---|---|
| `attenzione` | `verb\|attenzare-Inf-Ind-Imp-S2~pron\|ne` | `noun\|attenzione-Fem` |
| `macchine` | `verb\|maccare-Part-Past-P~pron\|ne` | `noun\|macchina-Fem-Plur` |
| `gallina` | `galli` + `na` | `noun\|gallina-Fem` |
| `cavallo` | `cava` + `lo` | `noun\|cavallo-Masc` |
| `mucche` | `mu` + `cce` + `he` | `noun\|mucca-Fem-Plur` |
| `persone` | `perso` + `ne` | `noun\|persona-Fem-Plur` |
| `eccolo` | `ecco` + `lo` | `adv\|ecco~pron\|lo` (PRESERVED) |

10 of 10 correct, including plural lemmas (*macchine* -> *macchina*) that the
split had destroyed. Multi-word utterances are untouched, since the same nouns
were already analyzed correctly there.

The accepted cost: a one-word verb+enclitic imperative (`dammi`,
`portarmelo`) will no longer split, because verb+enclitic is an open class and
admitting it by surface pattern would also readmit `cavallo` -> *cava*+*lo*
and `attenzione` -> *attenzi*+*ne*, whose bases are equally verb-shaped.
Losing a split leaves a real word coarsely analyzed; a false split invents a
verb. The asymmetry decides it.

## Limitation 3: Italian MWT mis-splits common function words

**Symptom.** Observed in the same 2026-07-28 run, in full sentences:

```
la stazione e molto grande .        -> "il i stazione e molto grande ."
secondo la mia opinione hai ragione . -> "... ha i ragione ."
```

`la` becomes `il` + `i`, and `hai` (2sg of *avere*) becomes `ha` + `i`.

**Status: NOT investigated.** Found while measuring limitation 2; scope,
frequency, and corpus impact are all unmeasured. Do not assume it is rare.

## Open work

1. **Measure limitations 2 and 3 across the corpora** before designing a fix.
   The right question is how many utterances carry a bogus verb+clitic
   analysis, not how many carry `IOB`; the label was only the visible tip.
2. **Design MWT suppression.** The tokenizer postprocessor
   (`batchalign/inference/_tokenizer_realign.py`) already speaks Stanza's
   `(text, bool)` MWT-hint protocol, where `False` suppresses expansion, so
   the mechanism exists. What is missing is the policy for when to emit it.
   Note that CHAT input is ALREADY tokenized by transcription convention, so
   "do not re-tokenize a CHAT word unless it is a known contraction" is a
   defensible default; it is also a broad behavioral change and must be
   corpus-measured before adoption.
3. **Re-generate affected corpus files** once a fix lands. Re-running
   morphotag BEFORE fixing the generator reproduces the defect: verified
   2026-07-28, which is why the tempting "just rerun BA3 on the bad files"
   would have wasted a full corpus pass.

## Related

- `batchalign/inference/morphosyntax.py`: `UD_RELATIONS`, `UD_DEPREL_ALIASES`,
  `validate_ud_words`, `batch_infer_morphosyntax`.
- `batchalign/inference/_tokenizer_realign.py`: the MWT-hint postprocessor.
- `batchalign/worker/_stanza_loading.py`: per-language pipeline construction;
  note Italian takes the `tokenize_postprocessor` branch, NOT the
  `tokenize_pretokenized` one, which is why Stanza re-tokenizes CHAT words at
  all.
- chatter error E761 (`%gra` relation head not a UD relation): the reading-side
  rule that surfaced all of this.
