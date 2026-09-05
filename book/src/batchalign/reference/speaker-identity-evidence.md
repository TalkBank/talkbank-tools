# Speaker Identity Evidence

**Status:** Current
**Last updated:** 2026-09-04 23:35 EDT

Field-by-field reference for the `<stem>_speaker_identity.json` artifact
written by [`speaker-identify`](../user-guide/commands/speaker-identify.md).
The user page covers when and why to run the command; this page is what a
consumer parses.

## Schema version

`provenance.schema_version` is `1`.

It is bumped when a reader that understood the previous version would
**misread** this one. Adding an optional field is not a bump; changing what an
existing field means is.

## Top level

| Field | Type | Meaning |
| --- | --- | --- |
| `provenance` | object | How this file was made. See below. |
| `utterances` | array | One entry per utterance of the scored tiers, in transcript order. |

## `provenance`

Written unconditionally, into the artifact itself, at the moment it is made.
Every number in the file depends on a choice somebody made, and a reader who
cannot reconstruct those choices treats the file as unreproducible.

| Field | Type | Meaning |
| --- | --- | --- |
| `schema_version` | integer | See above. |
| `interpretation` | string | The standing caveat that scores are agreement, not accuracy. Present because the file is what gets forwarded; the documentation is not. |
| `transcript` | string | The transcript this run read, as named. |
| `media` | string | The recording resolved beside it. |
| `prepared_sample_rate_hz` | integer | Sample rate of the single decode every span indexed into. |
| `embedding_backend` | string | `pyannote`. |
| `embedding_model_revision` | string | `pyannote-embedding:<40-hex Hub commit>`, read from the packaged model manifest. |
| `embedding_dimension` | integer | Width of every vector compared, as the worker reported it. |
| `embedding_minimum_frames` | integer | The model's own minimum span length, as the worker reported it. |
| `match_threshold` | number | The threshold the caller stated. Taken from the policy that produced the verdicts, so the file cannot state one number and have been decided under another. |
| `tiers` | array of string | Tiers scored. `["*"]` means every tier. |
| `enrollments` | array | Each `{ label, start_ms, end_ms }`, in recording order. |
| `produced_by` | string | Build identity of the batchalign3 that wrote this. |

`embedding_dimension` and `embedding_minimum_frames` are **reported by the
worker**, not constants on the Rust side. They are properties of the loaded
model file, so a constant here would go on agreeing with a model that had moved.

## `utterances[]`

| Field | Type | Meaning |
| --- | --- | --- |
| `utterance_index` | integer | Zero-based, counting utterances of the scored tiers. |
| `line` | integer | One-based line of the main tier in the transcript. |
| `speaker` | string | The speaker code the transcript currently carries. |
| `start_ms`, `end_ms` | integer | The bullet, **absent entirely** when the utterance has none. |
| `scores` | array | `{ label, score }` for every enrolled voice. Empty for an unscored utterance. |
| `verdict` | object | Tagged on `verdict`. See below. |

`scores` carries **every** enrolled voice, not only the best one, so a consumer
choosing a different threshold never has to re-run inference.

`score` is a cosine similarity in `[-1, 1]`. It is refused at both ends of the
pipeline if it is not: a NaN, which the model returns for input it cannot
measure, cannot reach this file.

## `verdict`

Internally tagged on `verdict`.

```json
{ "verdict": "matches", "label": "INV", "score": 0.81 }
{ "verdict": "no_match", "best": { "labels": ["INV"], "score": 0.44 } }
{ "verdict": "no_match", "best": { "labels": ["CHI", "INV"], "score": 0.90 } }
{ "verdict": "unscored", "reason": "no_bullet" }
{ "verdict": "unscored", "reason": "no_comparable_embedding" }
{ "verdict": "unscored", "reason": "too_short_for_embedding", "frames": 400, "minimum_frames": 1680 }
```

- **`matches`** carries exactly one `label`. A match with two labels has no
  representation, which is how a tie is prevented from resolving to a speaker.
- **`no_match`**'s `best.labels` normally holds one label. It holds more when
  the evidence ties, including when the tied score clears the threshold.
- **`unscored`**'s `reason` is flattened alongside `verdict`, with its own
  fields. The five reasons are `too_short_for_embedding` (with `frames`,
  `minimum_frames`), `no_bullet`, `no_comparable_embedding`, `audio_missing`
  (with `start_ms`, `end_ms`, `recording_ms`) and `overlaps_enrollment` (with
  `label`). `no_comparable_embedding` means every attempted comparison was
  refused, so the system does not mislabel that state as missing timing.

An unscored utterance carries **no** `score` field at all, rather than a zero.
A zero similarity is a real measurement, and a file that used one to mean
"not measured" would be indistinguishable from one that measured zero.

## Consuming it

The transcript is unchanged, so a consumer that wants to re-tier reads this
file, decides its own mapping from label to CHAT speaker code, and applies it.
Two facts make that safe to automate:

- `line` and `utterance_index` locate the utterance without re-parsing
  anything ambiguous.
- Nothing is omitted: every utterance of the scored tiers appears, so a count
  of entries equals a count of utterances.

Do not infer a decision from `scores` alone without reading `provenance`: the
same numbers under a different enrollment mean something different.
