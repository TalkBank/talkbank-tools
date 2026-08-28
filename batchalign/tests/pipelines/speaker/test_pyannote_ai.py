"""Contract tests for the typed pyannoteAI diarization lifecycle."""

from __future__ import annotations

import json
import wave
from dataclasses import dataclass
from pathlib import Path
from typing import Any

import numpy as np

from batchalign.inference.pyannote_ai import (
    CompletedDiarizationJob,
    PreparedWav,
    PyannoteAIClient,
    SubmittedDiarizationJob,
    UploadedMedia,
    render_prepared_wav,
    resolve_pyannote_ai_api_key,
)


@dataclass
class _Response:
    payload: bytes

    def __enter__(self) -> _Response:
        return self

    def __exit__(self, *_args: object) -> None:
        return None

    def read(self) -> bytes:
        return self.payload


class _RecordedOpener:
    def __init__(self, responses: list[dict[str, Any] | bytes]) -> None:
        self.responses = list(responses)
        self.requests: list[Any] = []

    def __call__(self, request, *, timeout: float):
        self.requests.append((request, timeout))
        response = self.responses.pop(0)
        payload = (
            response if isinstance(response, bytes) else json.dumps(response).encode()
        )
        return _Response(payload)


def test_render_prepared_wav_returns_a_typed_valid_mono_wav() -> None:
    prepared = render_prepared_wav(
        np.asarray([-1.0, 0.0, 1.0], dtype=np.float32), 16000
    )

    assert isinstance(prepared, PreparedWav)
    with wave.open(prepared.open(), "rb") as handle:
        assert handle.getnchannels() == 1
        assert handle.getsampwidth() == 2
        assert handle.getframerate() == 16000
        assert handle.getnframes() == 3


def test_client_transitions_through_upload_submit_and_completion_typestates() -> None:
    opener = _RecordedOpener(
        [
            {"url": "https://upload.example/audio"},
            b"",
            {"jobId": "job-1", "status": "created"},
            {"jobId": "job-1", "status": "running"},
            {
                "jobId": "job-1",
                "status": "succeeded",
                "output": {
                    "diarization": [{"speaker": "OVERLAP", "start": 0.0, "end": 2.0}],
                    "exclusiveDiarization": [
                        {"speaker": "SPEAKER_01", "start": 1.0, "end": 2.0},
                        {"speaker": "SPEAKER_00", "start": 0.0, "end": 1.0},
                    ],
                },
            },
        ]
    )
    sleeps: list[float] = []
    progress: list[str] = []
    client = PyannoteAIClient(
        "secret",
        base_url="https://api.example",
        poll_interval_s=0.25,
        urlopen=opener,
        sleep=sleeps.append,
        progress=progress.append,
    )
    prepared = render_prepared_wav(np.asarray([0.0], dtype=np.float32), 16000)

    uploaded = client.upload_wav(prepared)
    assert isinstance(uploaded, UploadedMedia)
    submitted = client.submit_diarization(uploaded, num_speakers=2)
    assert isinstance(submitted, SubmittedDiarizationJob)
    completed = client.wait_for_completion(submitted)
    assert isinstance(completed, CompletedDiarizationJob)

    assert completed.output["exclusiveDiarization"] == [
        {"speaker": "SPEAKER_01", "start": 1.0, "end": 2.0},
        {"speaker": "SPEAKER_00", "start": 0.0, "end": 1.0},
    ]
    assert sleeps == [0.25]
    assert progress == [
        "uploading_pyannote_ai_audio",
        "submitting_pyannote_ai_diarization",
        "waiting_for_pyannote_ai_diarization",
        "pyannote_ai_diarization_complete",
    ]

    submit_request = opener.requests[2][0]
    assert json.loads(submit_request.data) == {
        "url": uploaded.media_url,
        "model": "precision-2",
        "exclusive": True,
        "numSpeakers": 2,
    }


def test_api_key_resolution_prefers_environment_then_compatible_config(
    tmp_path: Path,
) -> None:
    config = tmp_path / "batchalign.ini"
    config.write_text("[diarize]\nengine.pyannote.key = from-config\n")

    assert (
        resolve_pyannote_ai_api_key(
            {"PYANNOTE_API_KEY": " from-environment "}, config_path=config
        )
        == "from-environment"
    )
    assert resolve_pyannote_ai_api_key({}, config_path=config) == "from-config"
