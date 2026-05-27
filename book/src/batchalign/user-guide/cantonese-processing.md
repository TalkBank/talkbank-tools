# Cantonese Engines

**Status:** Current
**Last updated:** 2026-05-27 12:00 EDT

Batchalign includes alternative ASR and forced alignment engines for Cantonese.
These are built-in modules activated via `--engine-overrides` and shipped in
the base package.

## Available Engines

| Engine | Task | Description |
|--------|------|-------------|
| `qwen` | ASR | Qwen3-ASR-1.7B local model (Alibaba). Open-weight Cantonese-capable ASR; external evaluations report competitive CER on per-utterance child speech. Downloads ~3.4 GB weights on first use; no cloud credentials. |
| `tencent` | ASR | Tencent Cloud speech recognition with speaker diarization. |
| `aliyun` | ASR | Alibaba Cloud NLS real-time speech recognition (Cantonese only). |
| `funaudio` | ASR | FunASR/SenseVoice local model (no cloud credentials needed). |
| `wav2vec_canto` | FA | Cantonese forced alignment with jyutping preprocessing. |

## Installation

The standard install already includes these engines:

```bash
uv tool install batchalign3
```

For a source checkout, the standard build (`cargo build -p batchalign`
plus `uv run maturin develop` for the PyO3 bridge) already includes
these engines. There are no Cantonese-specific extras to install.

## Usage

Select an alternative engine with `--engine-overrides`:

```bash
# Recommended: Qwen3-ASR (local, no credentials)
batchalign3 transcribe input/ -o output/ --lang yue \
  --engine-overrides '{"asr": "qwen"}'

# Pick the 0.6B model for faster inference on tight hardware
batchalign3 transcribe input/ -o output/ --lang yue \
  --engine-overrides '{"asr": "qwen", "qwen_model": "Qwen/Qwen3-ASR-0.6B"}'

# Transcribe with Tencent Cloud ASR (cloud, needs CAM credentials)
batchalign3 transcribe input/ -o output/ --lang yue \
  --engine-overrides '{"asr": "tencent"}'

# Transcribe with FunASR (local, no credentials)
batchalign3 transcribe input/ -o output/ --lang yue \
  --engine-overrides '{"asr": "funaudio"}'

# Benchmark against a gold CHAT companion in the input directory
batchalign3 benchmark input/ --output output/ --lang yue -n 1 \
  --engine-overrides '{"asr": "qwen"}'

# Force align with Cantonese FA engine
batchalign3 align input/ -o output/ --lang yue \
  --engine-overrides '{"fa": "wav2vec_canto"}'
```

## Credential Configuration

Cloud engines (Tencent, Aliyun) require API credentials in
`~/.batchalign.ini`:

### Tencent Cloud

```ini
[asr]
engine.tencent.id = <secret-id>
engine.tencent.key = <secret-key>
engine.tencent.region = ap-guangzhou
engine.tencent.bucket = <cos-bucket-name>
```

### Aliyun NLS

```ini
[asr]
engine.aliyun.ak_id = <access-key-id>
engine.aliyun.ak_secret = <access-key-secret>
engine.aliyun.ak_appkey = <appkey>
```

Missing or empty credentials raise `ConfigError` with a clear message
indicating which keys are needed.

### Qwen3-ASR

Qwen3-ASR has no cloud credentials — it is a local HuggingFace model
downloaded on first use (~3.4 GB for the default `Qwen/Qwen3-ASR-1.7B`).
Two `--engine-overrides` knobs are recognized:

- `qwen_model` — override the HuggingFace model id. The 1.7B default
  is the recommended-quality variant; pass `Qwen/Qwen3-ASR-0.6B` for
  faster inference at some accuracy cost.
- `qwen_device` — `"cpu"` (default), `"cuda"`, or `"mps"`. The
  Apple Silicon fleet defaults to CPU because empirical testing
  found MPS inference produced degraded output on the 1.7B model
  as of 2026-05-26.

## Cantonese Text Normalization

All Cantonese ASR output is automatically normalized from simplified/mixed
Chinese to Traditional Chinese. This normalization:

1. **Simplified → Traditional** via the `ferrous-opencc` Rust engine
   (embedded OpenCC `S2hk` conversion tables)
2. **Domain-specific corrections** via a 31-entry replacement table for
   Cantonese character variants (e.g., 系→係, 呀→啊, 中意→鍾意)

Normalization is built into the Rust extension (`batchalign_core`) and runs
automatically during ASR post-processing for `lang=yue`. No additional Python
dependencies (like OpenCC) are required.

## Engine Details

### Tencent Cloud ASR

- Supports speaker diarization with configurable speaker count
- Uploads audio to COS (Tencent Cloud Object Storage), submits ASR job, polls
  for results
- 10-minute safety timeout on ASR polling
- Automatic COS cleanup after transcription
- Per-word timestamps with speaker attribution

### Aliyun NLS ASR

- Cantonese only (`lang=yue` required, other languages rejected at load time)
- WebSocket streaming with real-time sentence callbacks
- Automatic token refresh (23-hour TTL)
- WAV format input required (16 kHz mono)
- Shared result shaping and Cantonese fallback tokenization happen in Rust,
  not in the Python transport adapter

### FunASR/SenseVoice

- Local model — no cloud credentials, no network required
- Default model is `FunAudioLLM/SenseVoiceSmall`. Pass
  `--engine-overrides '{"asr": "funaudio", "funaudio_model": "<hf-id>"}'`
  to swap to a different FunASR model (e.g. a Paraformer variant); the
  loader's downstream code branches on whether the chosen model name
  contains `paraformer`.
- VAD (Voice Activity Detection) built in via `fsmn-vad`
- Per-character Cantonese tokenization for timestamp alignment

### Qwen3-ASR

- Local model via the [`qwen-asr`](https://github.com/QwenLM/Qwen3-ASR)
  PyPI package — no cloud credentials, no network at inference time.
- Default model is `Qwen/Qwen3-ASR-1.7B`. The 0.6B variant is
  noticeably faster (smaller model, lighter compute) at some
  accuracy cost.
- First run downloads ~3.4 GB (1.7B fp16) or ~1.2 GB (0.6B) from
  HuggingFace; subsequent runs read from the local cache.
- The `qwen-asr` package handles long-audio chunking internally;
  no per-utterance pre-segmentation is required at the call site.
- Word-level timestamps emitted when the model returns them; falls
  back to whole-utterance text when timestamps aren't available.
- Single-speaker output (no built-in diarization); BA3's downstream
  diarization stage attaches speaker tags.
- Apache-2.0 licensed.

### Cantonese FA

- Converts Chinese characters to jyutping romanization (via pycantonese)
- Strips tones from jyutping (Wave2Vec MMS expects toneless input)
- Runs Wave2Vec forced alignment on the romanized text
- Maps word-level timings back to original Chinese characters

## See Also

- [Cantonese and CJK — Architecture](../../architecture/language-and-multilingual/cantonese-and-cjk.md) — engine architecture, normalization pipeline, segmenter selection
- [Adding Inference Providers](../developer/adding-engines.md) — how to add new built-in engines
