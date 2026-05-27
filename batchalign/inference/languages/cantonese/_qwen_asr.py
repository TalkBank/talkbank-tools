"""Qwen3-ASR worker provider for HK/Cantonese.

New-style provider: ``load`` is called once at worker startup,
``infer_*`` is called per request. Mirrors the
``_funaudio_asr`` / ``_tencent_asr`` module shape so the worker
bootstrap dispatch in ``batchalign/worker/_model_loading/asr.py``
can swap engines uniformly.
"""

from __future__ import annotations

import logging
import time

from pydantic import ValidationError

from batchalign.worker._types import (
    BatchInferRequest,
    BatchInferResponse,
    InferResponse,
)
from batchalign.inference.asr import (
    AsrBatchItem,
    AsrElement,
    AsrMonologue,
    MonologueAsrResponse,
)

from batchalign.inference._domain_types import LanguageCode

from ._common import EngineOverrides
from ._qwen_common import QwenRecognizer

L = logging.getLogger("batchalign.hk.qwen_asr")


# ---------------------------------------------------------------------------
# Module-level state
# ---------------------------------------------------------------------------

_recognizer: QwenRecognizer | None = None


# ---------------------------------------------------------------------------
# Load
# ---------------------------------------------------------------------------


def load_qwen_asr(
    lang: LanguageCode, engine_overrides: EngineOverrides | None
) -> None:
    """Initialize the Qwen3-ASR recognizer (called once at worker startup).

    Recognized ``engine_overrides`` keys:

    - ``qwen_model`` — HuggingFace model id of the Qwen3-ASR
      checkpoint. Default ``"Qwen/Qwen3-ASR-1.7B"``. ``"Qwen/Qwen3-ASR-0.6B"``
      is the smaller faster alternative.
    - ``qwen_device`` — torch device string (``"cpu"``, ``"cuda"``, ``"mps"``).
      Default ``"cpu"`` (Apple Silicon fleet has no CUDA, and MPS gives
      degraded output on 1.7B as of 2026-05-26).

    Eager-warms the model at the end of bootstrap so the first
    inference request sees a warm cache. The worker bootstrap
    already selected this engine; requests will come.
    """
    global _recognizer

    model_id = "Qwen/Qwen3-ASR-1.7B"
    device = "cpu"
    if engine_overrides:
        if "qwen_model" in engine_overrides:
            model_id = str(engine_overrides["qwen_model"])
        if "qwen_device" in engine_overrides:
            device = str(engine_overrides["qwen_device"])

    recognizer = QwenRecognizer(lang=lang, model_id=model_id, device=device)
    recognizer.warm()
    _recognizer = recognizer
    L.info(
        "Qwen3-ASR recognizer initialized: lang=%s, model=%s, device=%s",
        lang,
        model_id,
        device,
    )


# ---------------------------------------------------------------------------
# Infer
# ---------------------------------------------------------------------------


def infer_qwen_asr(req: BatchInferRequest) -> BatchInferResponse:
    """Batch-mode handler — kept for symmetry with the other HK ASR
    providers even though the V2 worker path uses ``infer_qwen_asr_v2``."""
    if _recognizer is None:
        return BatchInferResponse(
            results=[
                InferResponse(
                    error="Qwen3-ASR recognizer not loaded",
                    elapsed_s=0.0,
                )
                for _ in req.items
            ],
        )

    t0 = time.monotonic()
    results: list[InferResponse] = []
    for item_idx, raw_item in enumerate(req.items):
        try:
            item = AsrBatchItem.model_validate(raw_item)
        except ValidationError:
            results.append(
                InferResponse(error="Invalid AsrBatchItem", elapsed_s=0.0)
            )
            continue
        try:
            response = _transcribe_to_monologues(item)
            results.append(
                InferResponse(result=response.model_dump(), elapsed_s=0.0)
            )
        except Exception as exc:
            L.warning(
                "Qwen3-ASR failed for item %d: %s",
                item_idx,
                exc,
                exc_info=True,
            )
            results.append(InferResponse(error=str(exc), elapsed_s=0.0))

    elapsed = time.monotonic() - t0
    if results:
        first = results[0]
        results[0] = InferResponse(
            result=first.result, error=first.error, elapsed_s=elapsed
        )
    return BatchInferResponse(results=results)


def _transcribe_to_monologues(item: AsrBatchItem) -> MonologueAsrResponse:
    """Run Qwen3-ASR for one batch item; return the shared monologue payload."""
    if _recognizer is None:
        raise RuntimeError("Qwen3-ASR recognizer not initialized")

    payload, _timed_words = _recognizer.transcribe(item.audio_path)

    return MonologueAsrResponse(
        lang=item.lang,
        monologues=[
            AsrMonologue(
                speaker=monologue["speaker"],
                elements=[
                    AsrElement(
                        value=element["value"],
                        ts=element["ts"],
                        end_ts=element["end_ts"],
                        type=element["type"],
                    )
                    for element in monologue["elements"]
                    if element["value"].strip()
                ],
            )
            for monologue in payload["monologues"]
        ],
    )


def infer_qwen_asr_v2(item: AsrBatchItem) -> MonologueAsrResponse:
    """V2 worker entry point — one item, returns typed payload directly."""
    return _transcribe_to_monologues(item)
