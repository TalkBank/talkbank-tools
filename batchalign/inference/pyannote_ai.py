"""pyannoteAI Precision-2 cloud diarization.

The Rust control plane supplies normalized mono PCM. This module renders that
PCM as WAV, uploads it through pyannoteAI's temporary-media endpoint, submits
one diarization job, and returns typed speaker segments. The request asks for
exclusive diarization because pyannoteAI documents that output as the form
intended for reconciliation with an existing ASR transcript.
"""

from __future__ import annotations

import configparser
import io
import json
import logging
import os
import time
import urllib.error
import urllib.request
import uuid
import wave
from collections.abc import Callable, Mapping
from dataclasses import dataclass
from pathlib import Path
from typing import Any

import numpy as np

from batchalign.inference.speaker import SpeakerSegment

L = logging.getLogger("batchalign.worker")

_BASE_URL = "https://api.pyannote.ai"
_TERMINAL_STATUSES = {"succeeded", "failed", "canceled"}


def _emit_progress(stage: str) -> None:
    # The speaker runner callback does not receive the enclosing V2 request id,
    # so it cannot emit a well-formed progress_v2 frame. An empty id makes the
    # Rust worker reject the frame as unrelated to the live request. Keep these
    # lifecycle markers in the worker log until that id is part of the typed
    # callback boundary.
    L.info("pyannoteAI stage: %s", stage)


@dataclass(frozen=True, slots=True)
class PreparedWav:
    """WAV bytes ready for upload, before any remote resource exists."""

    data: bytes

    def open(self) -> io.BytesIO:
        return io.BytesIO(self.data)


@dataclass(frozen=True, slots=True)
class UploadedMedia:
    """A pyannoteAI temporary-media reference created by an upload."""

    media_url: str


@dataclass(frozen=True, slots=True)
class SubmittedDiarizationJob:
    """A remote diarization job that may still be running."""

    job_id: str


@dataclass(frozen=True, slots=True)
class CompletedDiarizationJob:
    """A successfully completed remote job with immutable output."""

    job_id: str
    output: dict[str, Any]
    warning: str | None = None


def resolve_pyannote_ai_api_key(
    env: Mapping[str, str] | None = None,
    *,
    config_path: Path | None = None,
) -> str | None:
    """Resolve a pyannoteAI key without ever logging or returning its source."""

    values = os.environ if env is None else env
    for name in (
        "BATCHALIGN_PYANNOTE_API_KEY",
        "BATCHALIGN_PYANNOTE_KEY",
        "PYANNOTE_API_KEY",
    ):
        value = values.get(name, "").strip()
        if value:
            return value

    path = config_path or Path.home() / ".batchalign.ini"
    parser = configparser.ConfigParser()
    try:
        loaded = parser.read(path)
    except (configparser.Error, OSError):
        return None
    if not loaded or not parser.has_section("diarize"):
        return None
    value = parser.get("diarize", "engine.pyannote.key", fallback="").strip()
    return value or None


def render_prepared_wav(audio: np.ndarray, sample_rate_hz: int) -> PreparedWav:
    """Encode normalized mono float PCM as a 16-bit mono WAV."""

    mono = np.asarray(audio, dtype=np.float32)
    clipped = np.clip(mono, -1.0, 1.0)
    pcm16 = (clipped * np.iinfo(np.int16).max).astype("<i2")
    output = io.BytesIO()
    with wave.open(output, "wb") as handle:
        handle.setnchannels(1)
        handle.setsampwidth(2)
        handle.setframerate(sample_rate_hz)
        handle.writeframes(pcm16.tobytes())
    return PreparedWav(output.getvalue())


class PyannoteAIClient:
    """Small synchronous client for one pyannoteAI diarization job."""

    def __init__(
        self,
        api_key: str,
        *,
        base_url: str = _BASE_URL,
        model: str = "precision-2",
        poll_interval_s: float = 5.0,
        timeout_s: float = 3600.0,
        http_timeout_s: float = 60.0,
        urlopen: Callable[..., Any] = urllib.request.urlopen,
        sleep: Callable[[float], None] = time.sleep,
        progress: Callable[[str], None] = _emit_progress,
    ) -> None:
        if not api_key.strip():
            raise ValueError("pyannoteAI API key must not be empty")
        if model not in {"precision-2", "community-1"}:
            raise ValueError("unsupported pyannoteAI model")
        if poll_interval_s <= 0 or timeout_s <= 0 or http_timeout_s <= 0:
            raise ValueError("pyannoteAI polling and HTTP timeouts must be positive")
        self._api_key = api_key.strip()
        self._base_url = base_url.rstrip("/")
        self._model = model
        self._poll_interval_s = poll_interval_s
        self._timeout_s = timeout_s
        self._http_timeout_s = http_timeout_s
        self._urlopen = urlopen
        self._sleep = sleep
        self._progress = progress

    def upload_wav(self, prepared: PreparedWav) -> UploadedMedia:
        """Transition prepared local audio to uploaded temporary media."""

        self._progress("uploading_pyannote_ai_audio")
        media_url = f"media://batchalign3/{uuid.uuid4().hex}.wav"
        upload = self._request_json(
            "POST", "/v1/media/input", payload={"url": media_url}
        )
        upload_url = upload.get("url")
        if not isinstance(upload_url, str) or not upload_url.startswith("https://"):
            raise RuntimeError("pyannoteAI media endpoint returned no HTTPS upload URL")
        request = urllib.request.Request(
            upload_url,
            data=prepared.data,
            headers={"Content-Type": "application/octet-stream"},
            method="PUT",
        )
        self._open(request, operation="upload media")
        return UploadedMedia(media_url)

    def submit_diarization(
        self,
        uploaded: UploadedMedia,
        num_speakers: int | None,
    ) -> SubmittedDiarizationJob:
        """Transition uploaded media to a submitted diarization job."""

        self._progress("submitting_pyannote_ai_diarization")
        payload: dict[str, Any] = {
            "url": uploaded.media_url,
            "model": self._model,
            "exclusive": True,
        }
        if num_speakers is not None:
            if num_speakers < 1:
                raise ValueError("pyannoteAI num_speakers must be at least 1")
            payload["numSpeakers"] = num_speakers
        submitted = self._request_json("POST", "/v1/diarize", payload=payload)
        job_id = submitted.get("jobId")
        if not isinstance(job_id, str) or not job_id:
            raise RuntimeError("pyannoteAI diarize endpoint returned no jobId")
        return SubmittedDiarizationJob(job_id)

    def wait_for_completion(
        self, submitted: SubmittedDiarizationJob
    ) -> CompletedDiarizationJob:
        """Transition one submitted job to successful immutable output."""

        self._progress("waiting_for_pyannote_ai_diarization")
        deadline = time.monotonic() + self._timeout_s
        while True:
            job = self._request_json("GET", f"/v1/jobs/{submitted.job_id}")
            status = str(job.get("status", "")).lower()
            if status in _TERMINAL_STATUSES:
                if status != "succeeded":
                    detail = job.get("message") or job.get("error") or status
                    raise RuntimeError(
                        f"pyannoteAI job {submitted.job_id} {status}: {detail}"
                    )
                output = job.get("output")
                if not isinstance(output, dict):
                    raise RuntimeError("pyannoteAI succeeded job has no output")
                warning = job.get("warning") or output.get("warning")
                completed = CompletedDiarizationJob(
                    job_id=submitted.job_id,
                    output=output,
                    warning=str(warning) if warning else None,
                )
                self._progress("pyannote_ai_diarization_complete")
                return completed
            remaining = deadline - time.monotonic()
            if remaining <= 0:
                raise TimeoutError(f"pyannoteAI job {submitted.job_id} timed out")
            self._sleep(min(self._poll_interval_s, remaining))

    def diarize_wav(
        self,
        prepared: PreparedWav,
        num_speakers: int | None,
    ) -> CompletedDiarizationJob:
        """Compose the three explicit lifecycle transitions."""

        uploaded = self.upload_wav(prepared)
        submitted = self.submit_diarization(uploaded, num_speakers)
        return self.wait_for_completion(submitted)

    def _request_json(
        self,
        method: str,
        path: str,
        *,
        payload: dict[str, Any] | None = None,
    ) -> dict[str, Any]:
        body = None if payload is None else json.dumps(payload).encode("utf-8")
        request = urllib.request.Request(
            f"{self._base_url}{path}",
            data=body,
            headers={
                "Authorization": f"Bearer {self._api_key}",
                "Content-Type": "application/json",
                "User-Agent": "batchalign3/pyannote-ai",
            },
            method=method,
        )
        raw = self._open(request, operation=f"{method} {path}")
        try:
            parsed = json.loads(raw.decode("utf-8"))
        except (UnicodeDecodeError, json.JSONDecodeError) as error:
            raise RuntimeError(
                f"pyannoteAI {method} {path} returned invalid JSON"
            ) from error
        if not isinstance(parsed, dict):
            raise RuntimeError(f"pyannoteAI {method} {path} returned invalid data")
        return parsed

    def _open(self, request: urllib.request.Request, *, operation: str) -> bytes:
        for attempt in range(4):
            try:
                with self._urlopen(request, timeout=self._http_timeout_s) as response:
                    return bytes(response.read())
            except urllib.error.HTTPError as error:
                if error.code == 429 and attempt < 3:
                    retry_after = (error.headers or {}).get("Retry-After", "1")
                    try:
                        delay = max(float(retry_after), 1.0)
                    except ValueError:
                        delay = 1.0
                    self._sleep(delay)
                    continue
                detail = _http_error_detail(error)
                raise RuntimeError(
                    f"pyannoteAI {operation} failed (HTTP {error.code}): {detail}"
                ) from error
            except (urllib.error.URLError, TimeoutError) as error:
                raise RuntimeError(f"pyannoteAI {operation} failed: {error}") from error
        raise RuntimeError(f"pyannoteAI {operation} exhausted rate-limit retries")


def infer_pyannote_ai(
    audio: np.ndarray,
    sample_rate_hz: int,
    num_speakers: int | None,
) -> list[SpeakerSegment]:
    """Production adapter from prepared PCM to pyannoteAI segments."""

    api_key = resolve_pyannote_ai_api_key()
    if api_key is None:
        raise RuntimeError(
            "pyannoteAI has no API key configured; set PYANNOTE_API_KEY or "
            "BATCHALIGN_PYANNOTE_API_KEY, or add engine.pyannote.key to the "
            "[diarize] section of ~/.batchalign.ini"
        )
    completed = PyannoteAIClient(api_key).diarize_wav(
        render_prepared_wav(audio, sample_rate_hz), num_speakers
    )
    return segments_from_completed_job(completed)


def segments_from_completed_job(
    completed: CompletedDiarizationJob,
) -> list[SpeakerSegment]:
    """Project completed remote output into the worker's segment type."""

    if completed.warning:
        L.warning("pyannoteAI warning: %s", completed.warning)
    raw_segments = completed.output.get("exclusiveDiarization")
    if not isinstance(raw_segments, list):
        raw_segments = completed.output.get("diarization")
    if not isinstance(raw_segments, list):
        raise RuntimeError("pyannoteAI succeeded job has no diarization segments")

    segments: list[SpeakerSegment] = []
    for raw in raw_segments:
        if not isinstance(raw, dict):
            continue
        try:
            start_ms = max(0, round(float(raw["start"]) * 1000))
            end_ms = max(start_ms, round(float(raw["end"]) * 1000))
            speaker = str(raw["speaker"])
        except (KeyError, TypeError, ValueError):
            continue
        segments.append(
            SpeakerSegment(start_ms=start_ms, end_ms=end_ms, speaker=speaker)
        )
    return sorted(segments, key=lambda item: (item.start_ms, item.end_ms, item.speaker))


def _http_error_detail(error: urllib.error.HTTPError) -> str:
    try:
        body = error.read().decode("utf-8", errors="replace")
    except OSError:
        return str(error.reason)
    try:
        payload = json.loads(body)
    except json.JSONDecodeError:
        return body[:500] or str(error.reason)
    if isinstance(payload, dict):
        message = payload.get("message") or payload.get("error")
        if message:
            return str(message)
    return body[:500] or str(error.reason)
