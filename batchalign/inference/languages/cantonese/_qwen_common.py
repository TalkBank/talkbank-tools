"""Qwen3-ASR helpers for the built-in Cantonese engine.

Drives ``transformers``' native ``qwen3_asr`` module directly. Mirrors the
``_funaudio_common.FunAudioRecognizer`` shape so the worker bootstrap can swap
engines uniformly. The actual model load is deferred until first inference
call, which keeps worker startup bounded when no Qwen jobs are dispatched.

Engine selection rationale: Qwen3-ASR is an open-weight Cantonese-capable ASR
model from Alibaba. It is wired here as one engine option among several.

**Why this no longer uses the ``qwen-asr`` package.** That package declared
``transformers==4.57.6``, an exact pin, and it was the single dependency
holding the whole project on the last 4.x release, which in turn blocked the
calibrated AudioFlamingo3 model in ``machine_ear``. Upstream ``transformers``
implements these same checkpoints natively as of 5.13.0, so the pin is
avoidable rather than negotiable. The swap was gated on a CER differential over
all 18 fixtures of the Cantonese benchmark, not on the test suite, because the
failure mode of an engine change is a quality regression that imports cleanly
and passes everything. That differential put the native path ahead on 15 of 18
fixtures and ahead on every CHILDES fixture. Two things it also turned up are
handled in ``_qwen_chunking``: the package's real value-add was long-audio
chunking, which we now own, and greedy decoding can run away into repetition,
which we now detect.

Device default is ``cpu``. Apple Silicon hosts have no CUDA, and MPS is not
usable for this model here: long audio is SIGKILLed with no exception and no
crash report, and float16 on MPS segfaults at load. Hosts with CUDA can opt in
via ``engine_overrides["qwen_device"]="cuda"``.
"""

from __future__ import annotations

import logging
from dataclasses import dataclass, field
from typing import Any

from batchalign.inference._domain_types import LanguageCode

from ._asr_types import AsrElement, AsrGenerationPayload, AsrMonologue, TimedWord
from ._qwen_chunking import (
    SAMPLE_RATE,
    AudioChunk,
    RunawayOutput,
    detect_runaway,
    split_audio_into_chunks,
)

L = logging.getLogger("batchalign.hk.qwen")


# ISO-639-3 → English language label expected by Qwen3-ASR's
# ``language=`` parameter. Pinned in code rather than via pycountry
# because pycountry returns ``"Yue Chinese"`` for ``yue``, which Qwen
# does not accept (silent fall-through to auto-detect). The fix is
# explicit per-code mapping with a fail-loud default.
_QWEN_LANG_LABELS: dict[LanguageCode, str] = {
    "yue": "Cantonese",
    "zho": "Chinese",
    "cmn": "Chinese",
    "eng": "English",
}


# Canonical forced-alignment companion from the Qwen3-ASR family, in the `-hf`
# spelling the native module targets. Word-level timestamps are load-bearing
# (the downstream FA pipeline injects them into `%wor`), so the aligner is
# never optional: see `LoadedQwen`, whose existence is what guarantees it.
_QWEN_FORCED_ALIGNER_MODEL_ID = "Qwen/Qwen3-ForcedAligner-0.6B-hf"

# Caps generation for ONE chunk, which is at most ~185 s of audio. Generous for
# that much speech: the densest legitimate chunk measured across the Cantonese
# benchmark was 4.3 characters per second. Reaching this cap is the signature
# of a decoding runaway rather than of dense speech, which is why
# `detect_runaway` exists rather than this bound being trusted to contain the
# damage: hitting it still yields thousands of characters of repetition.
_MAX_NEW_TOKENS = 4096


def _check_native_checkpoint(model_id: str) -> None:
    """Refuse a pre-migration checkpoint id with an instruction, not a stack trace.

    The native ``transformers`` module targets the ``-hf`` repositories; the
    bare ones are the sidecar's. Both exist on the Hub, so an operator who
    kept an old ``--engine-overrides '{"qwen_model": ...}'`` invocation would
    otherwise get an obscure failure deep inside a loader. Deliberately NOT
    rewritten to the ``-hf`` form on their behalf: silently substituting a
    different model than the one asked for is how a run ends up not being the
    run somebody thinks it was.
    """
    if model_id.startswith("Qwen/Qwen3-ASR") and not model_id.endswith("-hf"):
        raise ValueError(
            f"Qwen3-ASR checkpoint {model_id!r} is the pre-2026-08 sidecar "
            f"spelling. The engine now uses transformers' native qwen3_asr "
            f"module, which targets the '-hf' repositories: pass "
            f"{model_id + '-hf'!r} instead."
        )


def _resolve_qwen_language(lang: LanguageCode) -> str:
    label = _QWEN_LANG_LABELS.get(lang)
    if label is None:
        raise ValueError(
            f"Qwen3-ASR has no language label mapped for ISO-639-3 "
            f"{lang!r}; add it to _QWEN_LANG_LABELS in "
            f"_qwen_common.py if the model supports the language."
        )
    return label


@dataclass
class QwenAsrSegment:
    """One chunk of transcript with its word-level timestamps.

    ``text`` is the chunk's full string. ``word_timestamps`` is a list of
    ``(start_s, end_s, word_text)`` tuples in RECORDING-relative seconds,
    already converted out of the aligner's window-relative space. It is empty
    when the aligner returned nothing for the chunk.
    """

    text: str
    word_timestamps: list[tuple[float, float, str]] = field(default_factory=list)


@dataclass(frozen=True)
class QwenTranscription:
    """Everything one recording produced, including what it did NOT produce.

    A named seam rather than a bare pair because `refused` is not a detail of
    the segments: it is audio that reached the transcript as nothing at all,
    and a caller that never looks at it cannot tell that stretch from silence.
    Keeping it beside the segments is what stops the refusal from living only
    in a log line.
    """

    segments: list[QwenAsrSegment]
    refused: list[RunawayOutput]

    @property
    def refused_seconds(self) -> float:
        return sum(r.audio_seconds for r in self.refused)


@dataclass(frozen=True)
class LoadedQwen:
    """Both models, loaded. Its existence is the proof the aligner is wired.

    The engine needs an ASR model AND a forced aligner, because word timings
    are what `%wor` is built from. Previously that pairing was a convention:
    the loader passed an aligner argument and a regression test watched to see
    that it kept doing so, after a production run deadlocked for 32 minutes
    when it did not.

    A test is the wrong instrument for that. This type has one constructor,
    which loads both or raises, so "transcribe with no aligner" has no
    signature to travel through and the test that watched for it is gone.
    """

    processor: Any
    model: Any
    aligner_processor: Any
    aligner: Any

    @classmethod
    def load(
        cls, model_id: str, aligner_id: str, device: str, dtype: Any
    ) -> LoadedQwen:
        from transformers import (  # type: ignore[import-not-found]
            AutoModelForMultimodalLM,
            AutoProcessor,
            Qwen3ASRForTokenClassification,
        )

        return cls(
            processor=AutoProcessor.from_pretrained(model_id),
            model=AutoModelForMultimodalLM.from_pretrained(
                model_id, device_map=device, dtype=dtype
            ),
            aligner_processor=AutoProcessor.from_pretrained(aligner_id),
            aligner=Qwen3ASRForTokenClassification.from_pretrained(
                aligner_id, device_map=device, dtype=dtype
            ),
        )


class QwenRecognizer:
    """Wrapper around Qwen3-ASR model invocation.

    The transcribe API returns a typed ``AsrGenerationPayload`` +
    timed-words list, matching the contract the BA3 worker protocol
    expects. Projection from per-utterance results to speaker-tagged
    monologues is done locally in Python (single-speaker for now;
    Qwen3-ASR doesn't emit speaker diarization).

    For multi-speaker audio, the downstream BA3 pipeline applies
    diarization separately: Qwen3-ASR's role here is the
    speech-to-character transcription only. This matches how
    BA3 currently uses other diarization-free ASR engines.
    """

    def __init__(
        self,
        lang: LanguageCode = "yue",
        model_id: str = "Qwen/Qwen3-ASR-1.7B-hf",
        device: str = "cpu",
    ) -> None:
        self.lang = lang
        self.model_id = model_id
        self.device = device
        self._model: LoadedQwen | None = None
        # Both resolved at construction so an unsupported language or a
        # stale checkpoint id surfaces at worker startup rather than on the
        # first inference request, minutes into a job.
        _check_native_checkpoint(model_id)
        self._qwen_language = _resolve_qwen_language(lang)

    def warm(self) -> None:
        """Force the lazy model load. Call at worker bootstrap so the
        first inference request sees a warm cache, not a ~3.4 GB
        download + load delay."""
        self._get_model()

    def _get_model(self) -> LoadedQwen:
        if self._model is not None:
            return self._model

        try:
            import torch  # type: ignore[import-not-found]
        except ImportError as exc:
            raise ImportError(
                "Qwen3-ASR engine dependency 'torch' is missing from this "
                "environment. Reinstall batchalign3."
            ) from exc

        if self.device == "cuda":
            dtype = torch.bfloat16
        elif self.device == "mps":
            # User opted into MPS via override; warn rather than raise because
            # the user is sovereign on device selection. Measured on this
            # engine 2026-08-20: MPS is SIGKILLed with no exception on any
            # file needing more than three chunks, and float16 on MPS
            # segfaults during load.
            L.warning(
                "Qwen3-ASR MPS device requested; measured 2026-08-20 to be "
                "unusable for this model (long audio is SIGKILLed with no "
                "exception; float16 segfaults at load). Prefer cpu or cuda."
            )
            dtype = torch.float16
        else:
            dtype = torch.float32

        self._model = LoadedQwen.load(
            model_id=self.model_id,
            aligner_id=_QWEN_FORCED_ALIGNER_MODEL_ID,
            device=self.device,
            dtype=dtype,
        )
        L.info(
            "Qwen3-ASR loaded: model=%s, aligner=%s, lang=%s (%s), device=%s, dtype=%s",
            self.model_id,
            _QWEN_FORCED_ALIGNER_MODEL_ID,
            self.lang,
            self._qwen_language,
            self.device,
            dtype,
        )
        return self._model

    def _transcribe_chunk(self, loaded: LoadedQwen, chunk: AudioChunk) -> str:
        """ASR one chunk. Returns the chunk's text, window-scoped."""
        import torch  # type: ignore[import-not-found]

        inputs = loaded.processor.apply_transcription_request(
            audio=chunk.samples,
            # The model accepts full language names as well as ISO codes, so
            # the existing label mapping carries over unchanged. We pass an
            # explicit label rather than letting it auto-detect: the caller
            # has already established the session language via ``@Languages``,
            # and auto-detect on short or low-energy audio mis-classifies.
            language=self._qwen_language,
        ).to(loaded.model.device, loaded.model.dtype)
        with torch.inference_mode():
            output_ids = loaded.model.generate(**inputs, max_new_tokens=_MAX_NEW_TOKENS)
        generated = output_ids[:, inputs["input_ids"].shape[1] :]
        parsed = loaded.processor.decode(generated, return_format="parsed")[0]
        return str(parsed.get("transcription", "") or "")

    def _align_chunk(
        self, loaded: LoadedQwen, chunk: AudioChunk, text: str
    ) -> list[tuple[float, float, str]]:
        """Word timings for one chunk, converted into recording-relative time.

        The aligner reports against the window it was handed, so every value
        it returns goes through ``chunk.to_file``. That conversion is the one
        thing in this file that is silently wrong when omitted.
        """
        import torch  # type: ignore[import-not-found]

        inputs, word_lists = loaded.aligner_processor.prepare_forced_aligner_inputs(
            audio=chunk.samples, transcript=text, language=self._qwen_language
        )
        inputs = inputs.to(loaded.aligner.device, loaded.aligner.dtype)
        with torch.inference_mode():
            logits = loaded.aligner(**inputs).logits
        aligned = loaded.aligner_processor.decode_forced_alignment(
            logits=logits,
            input_ids=inputs["input_ids"],
            word_lists=word_lists,
            timestamp_token_id=loaded.aligner.config.timestamp_token_id,
        )[0]

        timings: list[tuple[float, float, str]] = []
        for word in aligned:
            token = str(word.get("text", "") or "")
            if not token.strip():
                continue
            timings.append(
                (
                    chunk.to_file(float(word["start_time"])),
                    chunk.to_file(float(word["end_time"])),
                    token,
                )
            )
        return timings

    def _run_model(self, source_path: str) -> QwenTranscription:
        """Transcribe and align the whole recording, chunk by chunk."""
        import librosa  # type: ignore[import-not-found]

        loaded = self._get_model()
        # A second audio-decode path in this repo, which already has one
        # (`inference.audio.load_audio`, soundfile-backed, used by the Aliyun
        # engine). Deliberate for now: `load_audio` does not resample, so
        # matching this single call means hand-composing load, mono-mix and a
        # torchaudio resampler, which is MORE code and a DIFFERENT resampler
        # than the one the engine's CER was measured under. Unify when
        # somebody re-measures, not before.
        wav, _sr = librosa.load(source_path, sr=SAMPLE_RATE, mono=True)
        chunks = split_audio_into_chunks(wav)

        segments: list[QwenAsrSegment] = []
        refused: list[RunawayOutput] = []
        for chunk in chunks:
            text = self._transcribe_chunk(loaded, chunk)
            if not text.strip():
                continue

            runaway = detect_runaway(text, chunk)
            if runaway is not None:
                L.warning("Qwen3-ASR discarding runaway chunk: %s", runaway.describe())
                refused.append(runaway)
                continue

            segments.append(
                QwenAsrSegment(
                    text=text,
                    word_timestamps=self._align_chunk(loaded, chunk, text),
                )
            )
        return QwenTranscription(segments=segments, refused=refused)

    def transcribe(
        self, source_path: str
    ) -> tuple[AsrGenerationPayload, list[TimedWord]]:
        """Run Qwen3-ASR on the audio file and return the shared
        ``(monologues_payload, timed_words)`` tuple.

        Unlike the FunASR path, this projection is done entirely in
        Python (no Rust ``batchalign_core`` call). The Qwen3-ASR
        output structure is simple enough that a per-engine Rust
        projection isn't warranted yet; if the multi-speaker /
        diarization story changes in a future Qwen release we can
        consolidate into a shared Rust helper alongside the FunASR
        and Tencent projections.
        """
        result = self._run_model(source_path)
        if result.refused:
            # Surfaced here as well as at the discard site, because by this
            # point it is a property of the whole transcript: this many
            # seconds of audio reached the output as nothing at all.
            L.warning(
                "Qwen3-ASR produced no transcript for %d chunk(s), %.1fs of audio: %s",
                len(result.refused),
                result.refused_seconds,
                "; ".join(r.describe() for r in result.refused),
            )
        segments = result.segments

        elements: list[AsrElement] = []
        timed_words: list[TimedWord] = []
        for seg in segments:
            if seg.word_timestamps:
                for start_s, end_s, word in seg.word_timestamps:
                    # Pre-filter empty/whitespace tokens so downstream
                    # never sees them: saves allocation and matches
                    # the FunASR + Tencent shape.
                    if not word.strip():
                        continue
                    elements.append(
                        AsrElement(
                            type="text",
                            ts=start_s,
                            end_ts=end_s,
                            value=word,
                        )
                    )
                    timed_words.append(
                        TimedWord(
                            word=word,
                            start_ms=int(start_s * 1000),
                            end_ms=int(end_s * 1000),
                        )
                    )
            elif seg.text.strip():
                # Fall back to the whole-segment text when the model
                # didn't return per-word timestamps. CJK output is
                # tokenized per-character downstream; we leave that
                # to BA3's standard Cantonese tokenizer rather than
                # splitting here (avoid double-tokenization).
                elements.append(
                    AsrElement(
                        type="text",
                        ts=None,
                        end_ts=None,
                        value=seg.text,
                    )
                )

        monologues: list[AsrMonologue] = [AsrMonologue(speaker=0, elements=elements)]
        payload: AsrGenerationPayload = AsrGenerationPayload(monologues=monologues)
        return payload, timed_words
