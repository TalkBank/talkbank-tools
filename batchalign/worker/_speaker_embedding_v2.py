"""Live worker-protocol V2 speaker-embedding executor.

The Rust control plane owns the request, the prepared audio and everything
downstream of the vectors: similarity, thresholds and verdicts. This module is
the thin model host, exactly as `_speaker_v2` is for diarization.
"""

from __future__ import annotations

from dataclasses import dataclass
from typing import TYPE_CHECKING

from pydantic import TypeAdapter

from batchalign.inference.speaker_embedding import (
    SpeakerEmbeddingResponse,
    SpeakerEmbeddingSpanRequest,
    embed_prepared_audio_spans,
)
from batchalign.worker._types_v2 import (
    ExecuteRequestV2,
    ExecuteResponseV2,
)

if TYPE_CHECKING:
    from collections.abc import Callable

    import numpy as np


@dataclass(frozen=True, slots=True)
class SpeakerEmbeddingExecutionHostV2:
    """Injected speaker-embedding hooks for the live V2 path."""

    pyannote_span_runner: (
        Callable[
            [np.ndarray, int, str],
            SpeakerEmbeddingResponse,
        ]
        | None
    ) = None


def build_default_speaker_embedding_execution_host_v2() -> (
    SpeakerEmbeddingExecutionHostV2
):
    """Build the live V2 embedding host over the pinned local model."""

    def _run_spans(
        audio: np.ndarray,
        sample_rate_hz: int,
        spans_json: str,
    ) -> SpeakerEmbeddingResponse:
        """Validate the Rust span request JSON at the model-host boundary."""

        return embed_prepared_audio_spans(
            audio,
            sample_rate_hz,
            TypeAdapter(list[SpeakerEmbeddingSpanRequest]).validate_json(spans_json),
        )

    return SpeakerEmbeddingExecutionHostV2(pyannote_span_runner=_run_spans)


def execute_speaker_embedding_request_v2(
    request: ExecuteRequestV2,
    host: SpeakerEmbeddingExecutionHostV2,
) -> ExecuteResponseV2:
    """Execute one live V2 embedding request through the Rust control plane."""

    import batchalign_core

    return ExecuteResponseV2.model_validate_json(
        batchalign_core.execute_speaker_embedding_request_v2(
            request,
            host.pyannote_span_runner,
        )
    )


__all__ = [
    "SpeakerEmbeddingExecutionHostV2",
    "build_default_speaker_embedding_execution_host_v2",
    "execute_speaker_embedding_request_v2",
]
