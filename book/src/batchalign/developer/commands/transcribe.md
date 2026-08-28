# transcribe: Developer Reference

**Status:** Current
**Last updated:** 2026-08-28 15:08 EDT

Implementation guide for the `transcribe` command. For user-facing
documentation, see [User Guide: transcribe](../../user-guide/commands/transcribe.md).

---

## Implementation map

| Layer | Location | Responsibility |
|-------|----------|----------------|
| CLI args | `crates/batchalign/src/cli/args/commands.rs`: `TranscribeArgs` | Typed ASR and speaker engines, diarization, lang, num-speakers |
| Options builder | `crates/batchalign/src/cli/args/options.rs:195-243` (inline dispatch) | Maps `TranscribeArgs` → `CommandOptions::Transcribe(TranscribeOptions)` |
| Catalog entry | `crates/batchalign/src/recipe_runner/catalog.rs` | the `CatalogEntry` for `transcribe` |
| Stage recipe | `crates/batchalign/src/recipe_runner/recipes.rs` | `TRANSCRIBE_RECIPE` |
| Pipeline orchestration | `crates/batchalign/src/pipeline/transcribe.rs`: `run_transcribe_pipeline()` | ASR, optional dedicated diarization, post-process, speaker projection, pre-CHAT utseg, CHAT assembly, optional morphotag, serialize |
| Per-file dispatch | `crates/batchalign/src/runner/dispatch/transcribe_pipeline.rs` | Concurrent file orchestration bounded by semaphore |
| ASR post-processing | `crates/batchalign-transform/src/asr_postprocess/mod.rs` | 8 stages: compound merge, MWT split, number expand, Cantonese norm, long-turn split, retokenization, disfluency, retrace detection |
| Pre-CHAT utterance segmentation | `crates/batchalign/src/pipeline/transcribe.rs:421-457`: `process_asr_with_prechat_segmentation()` | Runs for eng/cmn/zho/yue: BERT utseg applied to prepared chunks BEFORE build_chat |
| CHAT assembly | `crates/batchalign-transform/src/build_chat/mod.rs:41`: `build_chat()` | Assembles `ChatFile` AST from `TranscriptDescription` (typed bridge) |
| Speaker projection | `crates/batchalign/src/chat_ops/speaker.rs`: `project_speakers_onto_chunks()` | Projects raw segments onto timed ASR words and splits prepared chunks before utseg and CHAT assembly |
| Speaker evidence cache | `crates/batchalign/src/transcribe/evidence_cache.rs` | Content-derived request identity, validated envelope, per-key lease, miss authorization, durable commit, fakeable inference boundary |
| Same-job turn retention | `crates/batchalign/src/runner/debug_dumper.rs`: `dump_speaker_turns()` | When `--debug-dir` is set, writes the exact dedicated turns used by transcribe or returns a typed failure |
| Canonical turns schema | `crates/batchalign/src/runner/dispatch/diarize_turns.rs` | Serializes chatter-compatible turns with backend-derived provenance |
| ASR worker IPC | `batchalign/inference/asr.py` | Python-hosted ASR engines; Rev is Rust-owned |
| Raw Rev evidence cache | `crates/batchalign/src/revai/evidence_cache.rs` | Provider-media identity, raw transcript envelope, miss authorization, durable commit, fakeable Rev boundary |
| Speaker worker IPC | `batchalign/inference/speaker.py`: `batch_infer_speaker()` | Exhaustive dispatch over pyannoteAI, local Pyannote, and NeMo, returning raw millisecond segments |
| pyannoteAI adapter | `batchalign/inference/pyannote_ai.py` | Typed prepare, upload, submit, complete lifecycle for Precision-2 exclusive diarization |

---

## ASR post-processing chain

All ASR post-processing runs in Rust (`crates/batchalign-transform/src/asr_postprocess/`). The pipeline is deterministic and language-aware.

### 8-stage pipeline

1. **Compound merging**: rejoin compound words split by ASR
   - Language-specific: English phrasal verbs, CJK terms, etc.
   - Implemented: `compounds::merge_compounds()`

2. **Multi-word token splitting**: split tokens containing spaces, interpolate timestamps
   - Normalizes ASR outputs that glue multiple words together
   - Distributes timing proportionally by text length

3. **Number expansion**: convert digit strings to word form
   - Cardinals: 47 languages via `NUM2LANG` static table (data/num2lang.json)
   - CJK: specialized `num2chinese` path
   - Ordinals/decades: English-specific `ordinal_year_eng` composer
   - Currency, percent, dash-ranges: dedicated Rust handlers
   - **Runtime:** Pure Rust table lookup (Python `num2words` involved at **build time only** for codegen; removed from runtime 2026-04-26)

4. **Cantonese normalization** (yue only), simplified→HK traditional + domain replacements
   - Uses `ferrous-opencc` crate + replacement table
   - Implemented: `cantonese::normalize()`

5. **Long-turn splitting**: chunk monologues >300 words
   - Prevents unbounded utterance lengths in downstream processing

6. **Retokenization**: punctuation-based utterance splitting
   - Splits by CHAT-legal sentence terminators (`.` `?` `!` `+...` etc.)
   - Handles long-pause splitting when ASR omits punctuation

7. **Disfluency replacement**: mark filled pauses and orthographic variants
   - Filled pauses: `"um"` → `"&-um"`, `"uh"` → `"&-uh"` (per-language wordlists)
   - Replacements: `"'cause"` → `"(be)cause"`, `"gonna"` → `"going to"` (CJK-aware)
   - Implemented: `cleanup::mark_disfluencies()`

8. **N-gram retrace detection**: detect repeated n-grams, wrap in `<...> [/]` annotation
   - Identifies speaker self-corrections (rephrasings)
   - Implemented: `cleanup::detect_retraces()`

---

## Pre-CHAT utterance segmentation (lang-specific)

For **eng, cmn, zho, yue**, a BERT-based utterance segmentation model runs
**after ASR post-processing and dedicated speaker projection** but **before
CHAT assembly**:

- Implemented in `crates/batchalign/src/pipeline/transcribe.rs:421-457`: `process_asr_with_prechat_segmentation()`
- Called only when `uses_prechat_utterance_model(resolved_lang)` is true (lines 387-389)
- Workflow:
  1. Prepare ASR chunks (stages 1-8 above)
  2. If dedicated diarization ran, project its segments onto timed words with
     `project_speakers_onto_chunks()` and split chunks at speaker changes
  3. Call `infer_utseg_assignments()` to get per-chunk segment boundaries from worker
  4. Apply `split_prepared_chunk_by_assignments()` to split chunks at boundaries
  5. Convert to final utterances and finalize
- **Purpose:** Improve sentence boundary detection for languages with ambiguous punctuation
- For all other languages: skip pre-CHAT segmentation; use punctuation-based retokenization only

---

## Worker IPC: ASR task (V2 protocol)

```text
execute_v2 request:
{
  "task": "asr",
  "prepared_audio": { path, start_ms, end_ms, sample_rate },
  "engine": "rev" | "whisper" | "whisperx" | "whisper_oai" | "tencent" | ...,
  "language": "eng",
  "num_speakers": 2
}

execute_v2 response:
{
  "tokens": [
    { "word": "hello", "start_s": 0.12, "end_s": 0.45,
      "speaker": "SPEAKER_00", "confidence": 0.98 },
    ...
  ]
}
```

The speaker field is optional, Rev.AI always provides it; Whisper omits it.

## Worker IPC: speaker task (V2 protocol)

When `--diarization enabled` is set, a second worker call runs after ASR:

```text
execute_v2 request:
{
  "task": "speaker",
  "prepared_audio": { path, ... },
  "backend": "pyannote_ai" | "pyannote" | "nemo",
  "num_speakers": 2
}

execute_v2 response:
{
  "segments": [
    { "start_s": 0.0, "end_s": 2.3, "speaker": "SPEAKER_00" },
    ...
  ]
}
```

The typed CLI selector `SpeakerEngineName` maps exhaustively to
`SpeakerBackendV2`. When no explicit speaker engine is supplied, enabled
diarization selects `PyannoteAi`. The cloud adapter uses explicit lifecycle
states: `PreparedWav`, `UploadedMedia`, `SubmittedDiarizationJob`, and
`CompletedDiarizationJob`. Only a completed job can be converted to speaker
segments. It requests `exclusive: true` and prefers `exclusiveDiarization`,
which is the provider output designed for ASR reconciliation.

Before that worker call, `SpeakerEvidenceRequest::from_audio()` hashes the full
inference media source and combines the digest with the preparation revision,
backend, expected speaker count, speaker-model revision, and evidence schema.
The model revision is a dedicated `SpeakerEvidenceModelRevision` newtype; the
pipeline cannot substitute its ASR `EngineVersion`.

`resolve_speaker_evidence()` owns the production decision:

1. Acquire the process-local lease for the semantic cache key.
2. Validate and replay a durable hit, or produce a typed
   `SpeakerEvidenceMiss`.
3. Consume the miss into `SpeakerInferenceAuthorization`.
4. Cross the `SpeakerEvidenceInference` boundary exactly once.
5. Validate and durably commit the normalized segments before releasing the
   lease.

`SpeakerWorkerInference` is the production implementation. Tests use the same
resolver with a call-counting fake, which proves how many times the billable
boundary is crossed. `infer_speaker()` itself is private behind the adapter.
Concurrent identical requests wait on the same lease and re-check SQLite after
the first request commits.

Cache corruption and cache-write errors fail the file. They never become a
miss, because that would make broken local state authorize a surprise paid
call. `--override-media-cache` deliberately constructs a forced-refresh miss,
then replaces the entry after successful inference.

The envelope stores normalized `SpeakerSegmentV2` evidence, not the complete
raw provider JSON. The pyannoteAI key uses its visible `precision-2` alias;
the provider does not expose an immutable backend build hash. These limits are
documented rather than hidden behind an overclaim of perfect invalidation.

Rev.AI transcription follows the same stronger pattern through
`resolve_rev_asr_evidence()`. Its durable `CompletedRevAsrEvidence` retains the
resolved language and provider-shaped `Transcript` before token conversion.
`RevAsrService` cannot perform provider work without
`RevAsrInferenceAuthorization`, and generic ASR accepts `NonRevAsrBackend`, so
`RustRevAi` cannot bypass the cache gate through that function.

The legacy batch preflight shortcut is disabled for transcribe, benchmark, and
align because it submitted before evidence lookup. Cold calls currently use
normal per-file concurrency. A future cache-aware parallel preflight must use
plan variants that contain either validated evidence or an authorized miss;
an optional untyped job ID is not an acceptable replacement.

`project_speakers_onto_chunks()` treats those segments as authoritative before
utterance segmentation. Each timed ASR word receives the label with the
greatest summed overlap. The operation then splits prepared chunks at label
changes. It reports contested words, unattested words, and inserted boundaries;
untimed tokens inherit adjacent evidence, and timed gaps take the nearest
dedicated segment. Once the dedicated segment set is nonempty, the projection
type cannot emit an ASR-origin label.

`DiarizationLabelCoordinates` is the sole coordinate map from model-native
labels to anonymous speaker indices. Both CHAT projection and canonical turns
serialization consume this type. This prevents a valid but false state where
`PAR0` in CHAT identifies a different voice from `PAR0` in the retained turns
artifact.

When `--debug-dir` is enabled, `stage_speaker_diarization()` calls
`DebugDumper::dump_speaker_turns()` before moving the segments into pipeline
state. Its result is `SpeakerTurnsDumpOutcome::Disabled` or
`SpeakerTurnsDumpOutcome::Written(PathBuf)`. Enabled failures are typed as
`SpeakerTurnsDumpError` and fail the file. Provenance is derived exhaustively
from `SpeakerBackendV2` through `SpeakerTurnsSource`; callers cannot attach an
arbitrary source string. The current file is an interim research artifact, not
the final versioned evidence-sidecar architecture.

---

## Language resolution flow

When the user specifies `--lang auto`, language detection happens in two phases:

### Phase 1: ASR-level detection
- ASR worker returns detected `lang` field in response (e.g., "spa", "fra", "eng")
- This becomes the **resolved language** for CHAT headers and NLP stages (utseg, morphosyntax)
- Implemented: `resolved_asr_language()` in `crates/batchalign/src/pipeline/transcribe.rs:362-385`

### Phase 2: Per-utterance code-switching detection (if lang=auto)
- During `build_chat()`, each utterance text is analyzed with `lang_detect::detect_utterance_language()`
- Detected language stored in `Utterance.lang` field
- Used to emit `[- lang]` code-switching precodes in CHAT tier (if different from resolved language)
- Implemented: `build_chat()` stage lines 629-657

For fixed languages (not auto):
- No per-utterance detection; entire file uses specified language
- No code-switching precodes emitted

---

## Rev.AI `skip_postprocessing` gate

For `lang == eng || lang == fra`, Rev.AI is called with
`skip_postprocessing=true`. This suppresses Rev.AI's built-in punctuation
so that BA3's BERT utseg model handles sentence boundary detection. For all
other languages, Rev.AI post-processing is applied. Gate implemented in
`batchalign/inference/asr.py`: `_revai_request()`.

---

## `transcribe_s` vs `transcribe`

`transcribe_s` is not a separate CLI command. It is an internal command
variant triggered by `--diarization enabled`. Both share the same
`transcribe_pipeline.rs` orchestrator; the only difference is whether the
dedicated speaker stage runs.

---

## Testing

```bash
# Fast unit tests (no ML models)
make test

# Transcribe golden tests (real ASR models, only on Fleet/Large-tier hosts)
cargo test -p batchalign --features ml-golden --test ml_golden transcribe::

# Python ASR inference tests
uv run pytest batchalign/tests/test_asr.py -m golden
```

---

## Related developer documentation

- [Command Flowcharts: transcribe](../../architecture/command-flowcharts.md#transcribe), detailed runtime flowchart
- [ASR Token Pipeline](../../architecture/asr-token-pipeline.md), post-processing details
- [Cantonese and CJK, Architecture](../../../architecture/language-and-multilingual/cantonese-and-cjk.md), Tencent, Aliyun, FunASR engine dispatch
- [Number Expansion](../../reference/number-expansion.md), per-language Rust expansion
