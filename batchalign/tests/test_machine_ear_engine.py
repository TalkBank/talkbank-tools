"""Deterministic tests for the machine-ear verify engine.

Fake processor/model doubles pin the generation plumbing (prompt
assembly, decode slicing, verdict parsing) without any model download;
the prompt wording test is the calibration lock.
"""

from __future__ import annotations

from pathlib import Path
from typing import Any

import numpy as np
import pytest
import soundfile

from batchalign.inference.machine_ear import (
    SAMPLE_RATE,
    ClipEarAnswer,
    EarVerdict,
    MachineEarHandle,
    ask_clip,
    ear_prompt,
    parse_verdict,
)


def test_prompt_wording_is_calibration_locked() -> None:
    assert ear_prompt("go dog go") == (
        "A child and adults are talking. Does the CHILD in this "
        'clip say: "go dog go"? Answer with exactly one '
        "word: YES or NO."
    )


def test_parse_verdict_prefix_matching() -> None:
    assert parse_verdict("YES") is EarVerdict.YES
    assert parse_verdict("yes, the child says it") is EarVerdict.YES
    assert parse_verdict("  No.") is EarVerdict.NO
    assert parse_verdict("The child says it") is EarVerdict.UNPARSEABLE
    assert parse_verdict("") is EarVerdict.UNPARSEABLE


class _FakeInputs(dict[str, Any]):
    """Processor output double: dict with a .to(device) like BatchFeature."""

    def to(self, device: str) -> _FakeInputs:
        return self


class _FakeProcessor:
    """Records the prompt it saw; decodes a canned answer."""

    def __init__(self, answer: str) -> None:
        self.answer = answer
        self.seen_prompt: str | None = None

    def apply_chat_template(
        self,
        conversation: list[dict[str, Any]],
        *,
        add_generation_prompt: bool,
        tokenize: bool,
    ) -> str:
        parts = [
            item["text"]
            for turn in conversation
            for item in turn["content"]
            if item["type"] == "text"
        ]
        return "\n".join(parts)

    def __call__(
        self,
        *,
        text: str,
        audio: list[np.ndarray],
        sampling_rate: int,
        return_tensors: str,
    ) -> _FakeInputs:
        import torch

        self.seen_prompt = text
        return _FakeInputs(input_ids=torch.zeros(1, 3, dtype=torch.int64))

    def batch_decode(self, generated: Any, *, skip_special_tokens: bool) -> list[str]:
        # The engine slices off the prompt tokens before decoding; a
        # correct slice leaves exactly two generated positions here.
        assert generated.shape[1] == 2
        return [self.answer]


class _FakeModel:
    def parameters(self) -> Any:
        import torch

        return iter([torch.zeros(1, dtype=torch.float32)])

    def generate(self, *, input_ids: Any, max_new_tokens: int, do_sample: bool) -> Any:
        import torch

        assert do_sample is False
        # Echo the 3 prompt positions plus 2 generated ones.
        return torch.zeros(1, input_ids.shape[1] + 2, dtype=torch.int64)


def clip_at(path: Path, rate: int) -> Path:
    soundfile.write(str(path), np.zeros(rate, dtype=np.float32), rate)
    return path


def test_ask_clip_prompts_and_parses(tmp_path: Path) -> None:
    processor = _FakeProcessor("YES it does")
    handle = MachineEarHandle(processor=processor, model=_FakeModel(), device="cpu")
    wav = clip_at(tmp_path / "clip.wav", SAMPLE_RATE)
    result = ask_clip(handle, wav, "go dog go")
    assert result == ClipEarAnswer(verdict=EarVerdict.YES, raw_answer="YES it does")
    assert processor.seen_prompt is not None
    assert ear_prompt("go dog go") in processor.seen_prompt


def test_ask_clip_rejects_wrong_sample_rate(tmp_path: Path) -> None:
    handle = MachineEarHandle(
        processor=_FakeProcessor("NO"), model=_FakeModel(), device="cpu"
    )
    wav = clip_at(tmp_path / "clip44k.wav", 44_100)
    with pytest.raises(ValueError, match="calibrated at 16000"):
        ask_clip(handle, wav, "anything")
