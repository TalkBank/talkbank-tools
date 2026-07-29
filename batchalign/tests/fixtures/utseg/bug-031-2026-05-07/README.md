# BUG-031 reproducer fixtures: 2026-05-07

**Status:** Reference
**Last updated:** 2026-05-08 12:10 EDT

## Provenance

External bug report on 2026-05-07 against Batchalign3 utterance
segmentation for Mandarin and Cantonese: model splits cohesive
disyllabic words across utterance boundaries.

## Files

| File | Purpose |
|---|---|
| `02-input.cha` | Mandarin (`@Languages: zho`) input. Already-checked transcript with two long unsegmented utterances. |
| `02-bad-output.cha` | The current BA3 output for `02-input.cha`, byte-stable. Contains the documented bad splits (耳/朵 and 李/明). **Pinned to make BUG-031 fixes show as a diff, NOT a desired snapshot.** |
| `40517d-input.cha` | Cantonese (`@Languages: yue`) input, 283 lines. |
| `40517d-bad-output.cha` | The current BA3 output for `40517d-input.cha`, byte-stable. Contains the documented bad splits (原/來, 呢手/機, 正/確圈, etc.). **Pinned to make BUG-031 fixes show as a diff, NOT a desired snapshot.** |

## Verification

The two `*-bad-output.cha` files are byte-identical to the outputs
attached to the original bug report. Reproducible by running:

```bash
batchalign3 utseg 02-input.cha 40517d-input.cha -o /tmp/utseg-out
diff /tmp/utseg-out/02.cha       02-bad-output.cha    # → empty
diff /tmp/utseg-out/40517d.cha   40517d-bad-output.cha # → empty
```

## When this fixture is no longer the bad output

When BUG-031 is fixed (decode-time word-cohesion mask, model
retrain, or both), the `*-bad-output.cha` files will become stale
they're pinning broken behavior. At that point:

1. Replace each `*-bad-output.cha` with a `*-good-output.cha` that
   reflects the new, correct behavior.
2. Add a regression test that loads the inputs, runs them through
   the pipeline, and asserts (a) no boundary inserted inside the
   known-bad bigrams (耳朵, 李明, 原來, 手機, 正確), and (b) the
   output matches `*-good-output.cha`.
3. Update BUG-031's `**State**` to GREEN with a pointer to the
   regression test.

Until then, the bad outputs are evidence, not aspiration.
