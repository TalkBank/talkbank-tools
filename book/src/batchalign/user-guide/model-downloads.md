# Model Downloads and Caching

**Status:** Current
**Last updated:** 2026-05-01 22:47 EDT

## Automatic Model Downloads

batchalign3 downloads ML models automatically the first time you use a
command that needs them. No manual setup is required.

| Command | What Downloads | Size | First-Run Time |
|---------|---------------|------|---------------|
| `morphotag` | Stanza POS/dependency models for your language | ~200-500 MB | 1-3 minutes |
| `morphotag --retokenize --lang yue` | Nothing extra (PyCantonese is bundled) | 0 | Instant |
| `morphotag --retokenize --lang cmn` | Stanza Chinese tokenizer model | ~200 MB | 1-2 minutes |
| `transcribe` | Whisper ASR model | 1-15 GB | 5-30 minutes |
| `align` | Wave2Vec forced alignment model | ~1.2 GB | 2-5 minutes |

After the first download, models are cached locally and reused instantly.

## Where Models Are Stored

Models are cached in standard locations:

| Library | Cache Directory |
|---------|----------------|
| Stanza | `~/stanza_resources/` |
| Whisper / Wave2Vec | `~/.cache/huggingface/hub/` |
| PyCantonese | Bundled with batchalign3 (no separate cache) |

These directories may grow to 10-30 GB depending on how many languages and
model sizes you use.

## Result Caching

batchalign3 caches **audio-bound** intermediate results so repeated runs
of `align` or `transcribe` on the same file with the same settings do
not redo the expensive ASR / forced-alignment passes. Two task kinds are
cached, both per-utterance:

- `forced_alignment` — Wave2Vec word timings.
- `utr_asr` — full-file ASR result used for utterance-timing recovery.

See `crates/batchalign/src/chat_ops/cache_key.rs::CacheTaskName` for the
authoritative list. Cache keys include the engine version, language, and
relevant inputs, so changing any of those produces a fresh entry.

**Text NLP tasks are not cached.** Running `morphotag`, `utseg`,
`translate`, or `coref` twice on the same file runs the model twice.
The CLI accepts `--override-media-cache-tasks morphosyntax` for
backward-compatible scripting but emits a warning ("`batchalign3 does
not cache text NLP`") and ignores it.

To force re-computation of the cached audio tasks (e.g. after a model
update):

```bash
batchalign3 align --override-media-cache corpus/ -o output/ --lang eng
batchalign3 transcribe --override-media-cache recordings/ -o transcripts/ --lang eng
```

`--override-media-cache` clears all audio task caches for the run.
For finer control, pass `--override-media-cache-tasks` with one or
more of `forced_alignment` / `utr_asr`.

## Offline Use

Once models are downloaded, batchalign3 works fully offline. No network
access is needed for any command after the initial model download.

If you need to pre-download models for an offline environment:

```bash
# Download Stanza models for a specific language
python -c "import stanza; stanza.download('en')"
python -c "import stanza; stanza.download('zh')"
```

## Disk Space Management

To free disk space by removing cached models (they will re-download on next use):

```bash
rm -rf ~/stanza_resources/          # Stanza models
rm -rf ~/.cache/huggingface/hub/    # Whisper, Wave2Vec models
```
