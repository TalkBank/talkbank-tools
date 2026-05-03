# Portuguese

**Status:** Current
**Last updated:** 2026-05-01 09:47 EDT

## Scope

Portuguese is a Romance language with productive preposition+article
contractions that Stanza's neural MWT processor handles natively:

* `do → de + o`, `da → de + a`, `dos`, `das`
* `no → em + o`, `na → em + a`, `nos`, `nas`
* `pelo → por + o`, `pela`, `pelos`, `pelas`
* `ao → a + o`, `aos`, `à → a + a`, `às`
* Elisions before vowel-initial nouns, e.g. `d'água` (idiomatic,
  "of water")

Portuguese carries no per-language BA3 MWT-override rules as of
2026-04-21. The single BA2-ported rule was audited and removed —
see [History](#history).

## What Stanza handles natively

Paired probes with Stanza 1.11.1 (free-tokenize vs our
postprocessor) show identical output on both paths and 1-to-1
preservation for:

| Pattern | Example | Stanza output |
|---------|---------|---------------|
| do / da / dos / das | `perto do rio` | `de + o + rio` (MWT, 1 CHAT word → 2 UD words, Range preserved) |
| no / na / nos / nas | `na cidade` | `em + a + cidade` (MWT) |
| ao / aos / à / às | `vou ao mercado` | `a + o + mercado` (MWT) |
| `d'água` (idiomatic elision) | `copo d'água`, `d'água` standalone | Stanza emits 1 token with the apostrophe preserved; no split needed |

Probes in
`batchalign/tests/investigations/_cases/portuguese.py` (typed
`ProbeCase` fixtures consumed by the matrix harness at
`test_stanza_mwt_probe_matrix.py`).

## Known Stanza limitations

No Portuguese-specific Stanza defects are tracked.

## History

### Rule that existed and was removed
| Rule | What it did | Audit finding |
|------|-------------|---------------|
| `Exact("d'água") → ForceMwt` | Force MWT expansion on the idiomatic elision `d'água` | **Net harm.** In the standalone case (`d'água` by itself), the forced expansion produced 2 UD words (`de` + `'água`) for 1 CHAT word, violating the morphotag 1-to-1 invariant. In sentence context (`um copo d'água`), Stanza already handled the token correctly without the hint. The rule added zero benefit and broke the invariant on one case. |

Removed in the audit. Portuguese now runs through the
default tokenizer-postprocessor path with no per-language overrides.

### Why the DP alignment is the load-bearing piece

As with French and Italian, the character-level Hirschberg
realignment in `align_tokens` is the unconditional, language-agnostic
rescue that merges any Stanza over-split back to 1 token per CHAT
word. Portuguese inherits this behavior for free.

## Tests

* **Probe matrix cases:**
  `batchalign/tests/investigations/_cases/portuguese.py` — typed
  `ProbeCase` fixtures covering `d'água` standalone and in context,
  plus native MWTs (`do`, `da`, `na`).
* **Matrix harness:**
  `batchalign/tests/investigations/test_stanza_mwt_probe_matrix.py`
  runs every case through paired pipelines. Invoke with
  `uv run pytest batchalign/tests/investigations/ -m golden`.
* **Behavior-table renderer:**
  `scripts/analysis/render_probe_matrix_table.py --lang por`.

## References

* [Morphotag Invariants](../../architecture/morphotag-invariants.md)
