# Caching

**Status:** Current
**Last updated:** 2026-08-28 19:15 EDT

## What gets cached

Batchalign caches **only audio-task results**:

| Analysis | Cached? |
|----------|---------|
| Forced alignment word timings (`align`) | Yes |
| ASR results for utterance timing recovery (`align`'s UTR pre-pass) | Yes |
| Dedicated speaker evidence (`transcribe --diarization enabled`) | Yes |
| Media conversion (`.mp4`/`.m4a` → `.wav`) | Yes |
| Raw Rev.AI transcript evidence (`transcribe`, `benchmark`, Rev-backed `align` UTR) | Yes |
| Other ordinary ASR output (`transcribe`) | No |
| Morphosyntax (`morphotag`) | **No**: always recomputed |
| Utterance segmentation (`utseg`) | **No**: always recomputed |
| Translation (`translate`) | **No**: always recomputed |
| Coreference (`coref`) | **No**: always recomputed |
| Standalone speaker diarization (`diarize`) | No |
| OpenSMILE features (`opensmile`) | No |
| AVQI scores (`avqi`) | No |

The text-NLP cache that previously covered `morphotag`, `utseg`, and
`translate` was **removed** after a benchmark on a 15,748-file corpus
showed it was about 25× slower than just re-inferring (6-16% hit rate;
2,500 ms SQLite lookup beat ~100 ms inference savings). See the
architecture page on Caching for the detailed reasoning.

In practice: a re-run of `morphotag` on the same corpus takes the
same time as the first run. A re-run of `align` on the same audio is
much faster. A repeat `transcribe --diarization enabled` run with the
same speaker settings reuses the exact normalized speaker turns that
the first run consumed, instead of calling the diarization backend
again. BA3 also retains backend-shaped evidence separately, so a changed
local normalization algorithm can derive new turns without repeating
inference.

For Rev.AI transcription, BA3 stores the provider-shaped monologues and
elements before converting them to BA3 tokens. A warm run therefore avoids
both Rev submission and polling, and later post-processing experiments can
replay the same raw transcript evidence locally.

### Dedicated speaker evidence

The speaker cache applies to the dedicated diarization stage of
`transcribe`, for all three speaker engines (`pyannote-ai`, `pyannote`,
and `nemo`). It is especially useful with the paid pyannoteAI service.

The key includes:

- a BLAKE3 digest of the full media-source bytes;
- the canonical audio-preparation recipe revision;
- the selected speaker backend;
- the expected speaker count;
- the speaker-model revision; and
- the stored evidence schema version.

Paths and modification times are deliberately excluded. Renaming or copying
an unchanged recording therefore reuses its speaker evidence. Re-encoding the
recording changes its bytes and causes a miss even if it sounds identical.

BA3 stores two different artifacts:

1. **Raw inference evidence.** For pyannoteAI this is the completed provider
   job ID, complete output object, and optional warning. Local Pyannote and
   NeMo retain their backend-specific segment evidence.
2. **Derived speaker segments.** These are the sorted millisecond intervals
   consumed by transcript speaker projection. Their identity includes both
   the raw-evidence fingerprint and a separate normalization-algorithm
   revision.

Changing only BA3's normalization algorithm invalidates the second artifact,
not the first. BA3 re-normalizes the retained provider response locally and
does not upload audio or submit another paid pyannoteAI job. Changing the
model, speaker count, backend, audio bytes, or preparation recipe changes the
raw identity and therefore requires inference.

BA3 validates schema versions, fingerprints, backend provenance, provider
job identity, speaker labels, interval direction, and ordering. A missing raw
entry permits inference; a corrupt raw or derived entry fails the file instead
of being treated as a miss and silently causing another billable call.
Concurrent identical requests in one BA3 server are serialized: the first
miss performs and commits inference, while followers wait and then replay the
result.

### Raw Rev.AI transcript evidence

Normal `transcribe`, `benchmark`, and Rev-backed `align` UTR runs check durable
raw Rev evidence before submitting anything to Rev. Only a missing entry, or
an explicit `--override-media-cache`, authorizes submission. Corrupt evidence
fails closed without a service call, and concurrent identical requests are
coalesced. Concurrent identical forced refreshes also share the first fresh
commit rather than each issuing a sequential paid call.

The key includes the full bytes of the provider-visible inference media,
requested language, expected speaker count, Rev request-policy revision,
provider/model alias, and evidence schema. The stored envelope contains the
resolved language plus Rev's raw monologues, elements, timings, confidence,
and punctuation. It does not store credentials or temporary job IDs.

The old batch pre-submission shortcut is disabled because it submitted paid
jobs before a cache hit could be known. Cold misses currently fan out through
BA3's normal per-file worker limit. Reintroducing wider parallel preflight is a
performance follow-up; it must consume the same typed miss authorization and
cannot restore the old unguarded optional-job-ID path.

Other ASR engines remain uncached in ordinary `transcribe` runs. Align's UTR
ASR also keeps its older normalized-result cache. If that derived entry is
missing, Rev-backed UTR can re-project the durable raw transcript without a
service call. A corrupt normalized UTR entry fails closed instead of silently
falling through to inference. The legacy Rev pre-submission path is disabled.

## What invalidates the cache

| What changed | What re-runs | What stays cached |
|---|---|---|
| Edited the transcript words | FA (per-group cache key includes text) | UTR ASR (only depends on audio) |
| Re-recorded or replaced the audio | FA, UTR ASR, Rev evidence, speaker evidence | (n/a, audio is the cache key) |
| Changed the language code | UTR ASR and Rev evidence | (other corpora's entries) |
| Changed expected speaker count | Rev evidence and speaker evidence | FA and UTR ASR |
| Changed speaker backend | Speaker evidence | FA, UTR ASR, and Rev evidence |
| Changed only the speaker normalization algorithm | Derived speaker segments | Raw speaker inference evidence |
| Upgraded batchalign or an identified model revision | Affected audio evidence | Entries from unchanged engines/models |

Cache keys hash the inputs relevant to each task. FA and UTR use the legacy
path/mtime/size `AudioIdentity`; Rev and speaker evidence use a true digest of
the inference-media bytes so copies and renames share results. Engine or model
revision strings are stored alongside each entry.

pyannoteAI currently exposes the `precision-2` model alias, but not an
immutable backend build hash. BA3 scopes cloud evidence to that alias and its
own evidence schema. If the provider changes the implementation behind the
same alias and you want fresh evidence, use `--override-media-cache`. The
local Pyannote and NeMo identifiers likewise include their configured model
identity and the BA3 package version; floating external model revisions remain
a reason to force a refresh during controlled experiments.

## How to force fresh results

Use the `--override-media-cache` global flag:

```bash
batchalign3 --override-media-cache align corpus/ -o output/

# Force and store fresh Rev and dedicated speaker evidence. This may incur charges.
batchalign3 --override-media-cache transcribe recordings/ -o output/ \
  --diarization enabled
```

This skips all applicable cache lookups, forcing fresh inference. New results
replace the matching entries and are stored for future runs. With
Rev ASR or `--speaker-engine pyannote-ai`, this can make new paid service calls
even when reusable evidence exists.

The narrower `--override-media-cache-tasks` flag currently accepts only
`forced_alignment` and `utr_asr`. Use the global `--override-media-cache` when
an experiment must refresh Rev or dedicated-speaker evidence. Selective names
for the new evidence tasks are a future CLI extension.

Use this when you suspect cached results are wrong, or after manually
updating model files outside of a normal batchalign upgrade.

## Where the caches are stored

| Cache | macOS default | Linux default |
|---|---|---|
| Analysis cache DB | `~/Library/Caches/batchalign3/cache.db` | `~/.cache/batchalign3/cache.db` |
| Media conversion cache | `~/Library/Application Support/batchalign3/media_cache/` | `~/.local/share/batchalign3/media_cache/` |

The analysis cache is a single SQLite database file. The media cache
stores converted WAV artifacts for inputs such as `.mp4` and `.m4a`.

For isolated runs or testing, you can relocate them with environment
variables:

```bash
export BATCHALIGN_ANALYSIS_CACHE_DIR=/tmp/ba-analysis-cache
export BATCHALIGN_MEDIA_CACHE_DIR=/tmp/ba-media-cache
```

## How to clear the cache

Use the built-in cache command:

```bash
batchalign3 cache stats          # See cache size and entry count
batchalign3 cache clear --yes    # Clear the cache
```

`cache stats` and `cache clear` operate on both the analysis cache and
the media conversion cache.

Or delete the `cache.db` file and/or the media-cache directory directly.

To selectively refresh without clearing everything, use
`--override-media-cache` on specific runs instead, old entries for
other corpora remain available.

## Old text-NLP cache entries

If you used batchalign before the text-NLP cache was removed, your
`cache.db` may still contain old `morphosyntax_v*`, `utseg_v*`, and
`translate_v*` rows. Those are dead weight, they're never read
anymore. Run `batchalign3 cache clear --yes` (or `rm -f
~/Library/Caches/batchalign3/cache.db*`) to reclaim the disk space.
