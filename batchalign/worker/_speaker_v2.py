"""Live worker-protocol V2 speaker diarization executor.

**See also:** `../../INTERFACE_MAP.md` section "6. Media Analysis V2: Speaker Diarization" for:
- Rust FFI function: `crates/batchalign-pyo3/src/worker_media_exec.rs::execute_speaker_request_v2()`
- Full Rust/Python responsibility split and input/output contracts.
"""

from __future__ import annotations

from dataclasses import dataclass
from typing import TYPE_CHECKING

from batchalign.device import DevicePolicy
from batchalign.inference.speaker import (
    SpeakerEngine,
    SpeakerResponse,
    infer_speaker_prepared_audio,
)
from batchalign.worker._types_v2 import (
    ExecuteRequestV2,
    ExecuteResponseV2,
)

if TYPE_CHECKING:
    from collections.abc import Callable

    import numpy as np


@dataclass(frozen=True, slots=True)
class SpeakerExecutionHostV2:
    """Injected speaker execution hooks for the live V2 path."""

    # The third argument is the speaker count, or None for "not specified",
    # which the CLI defines as auto-detect. It is Optional so that state can
    # reach the diarizer instead of being collapsed into a number.
    pyannote_ai_prepared_audio_runner: (
        Callable[[np.ndarray, int, int | None], SpeakerResponse] | None
    ) = None
    pyannote_prepared_audio_runner: (
        Callable[[np.ndarray, int, int | None], SpeakerResponse] | None
    ) = None
    nemo_prepared_audio_runner: (
        Callable[[np.ndarray, int, int | None], SpeakerResponse] | None
    ) = None


def build_default_speaker_execution_host_v2(
    device_policy: DevicePolicy | None = None,
) -> SpeakerExecutionHostV2:
    """Build the live V2 speaker host from the existing Python adapters."""

    def _run_prepared(
        audio: np.ndarray,
        sample_rate_hz: int,
        num_speakers: int | None,
        engine: SpeakerEngine,
    ) -> SpeakerResponse:
        """Run one prepared-audio item.

        ``num_speakers`` is ``None`` when the caller did not specify a count,
        which the CLI defines as auto-detect. It is passed through rather than
        defaulted here: the Rust boundary used to substitute 2, which made
        "estimate the count" unreachable.
        """

        return infer_speaker_prepared_audio(
            audio,
            sample_rate_hz,
            num_speakers=num_speakers,
            engine=engine,
            device_policy=device_policy,
        )

    return SpeakerExecutionHostV2(
        pyannote_ai_prepared_audio_runner=lambda audio, sample_rate_hz, num_speakers: (
            _run_prepared(
                audio,
                sample_rate_hz,
                num_speakers,
                "pyannote_ai",
            )
        ),
        pyannote_prepared_audio_runner=lambda audio, sample_rate_hz, num_speakers: (
            _run_prepared(
                audio,
                sample_rate_hz,
                num_speakers,
                "pyannote",
            )
        ),
        nemo_prepared_audio_runner=lambda audio, sample_rate_hz, num_speakers: (
            _run_prepared(
                audio,
                sample_rate_hz,
                num_speakers,
                "nemo",
            )
        ),
    )


def execute_speaker_request_v2(
    request: ExecuteRequestV2,
    host: SpeakerExecutionHostV2,
) -> ExecuteResponseV2:
    """Execute one live V2 speaker request through the Rust control plane."""

    import batchalign_core

    return ExecuteResponseV2.model_validate_json(
        batchalign_core.execute_speaker_request_v2(
            request,
            host.pyannote_ai_prepared_audio_runner,
            host.pyannote_prepared_audio_runner,
            host.nemo_prepared_audio_runner,
        )
    )


__all__ = [
    "SpeakerExecutionHostV2",
    "build_default_speaker_execution_host_v2",
    "execute_speaker_request_v2",
]
