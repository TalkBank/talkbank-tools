"""Deterministic tests for the FA-confirmation verify engine.

Fake bundle pieces (the house SimpleNamespace pattern from the FA
inference tests) give exact ground truth for the scoring math; the
window shaping and normalization are pure functions. No audio
fixtures, no model downloads.
"""

from __future__ import annotations

from types import SimpleNamespace
from typing import Any

import numpy as np
import pytest

from batchalign.inference.fa_confirm import (
    SAMPLE_RATE,
    WINDOW_MAX_MS,
    WINDOW_MIN_MS,
    WINDOW_PAD_MS,
    FaConfirmHandle,
    confirm_window_span,
    normalize_word,
    normalize_words,
    score_window,
)


def test_normalize_word_keeps_the_aligner_alphabet() -> None:
    assert normalize_word("Don't!") == "don't"
    assert normalize_word("Hello,") == "hello"
    assert normalize_word("123") == ""
    # Non-ASCII letters are dropped, not transliterated.
    assert normalize_word("naïve") == "nave"


def test_normalize_words_drops_vanishing_tokens() -> None:
    # Tokens whose every character falls outside a-z' vanish entirely;
    # note event codes like "&=laughs" do NOT vanish (their letters
    # survive), so callers strip non-speech tokens BEFORE normalizing.
    assert normalize_words(["Hey", "123", "(.)", "you."]) == ("hey", "you")


def test_window_span_pads_both_sides() -> None:
    assert confirm_window_span(5000, 6000) == (
        5000 - WINDOW_PAD_MS,
        6000 + WINDOW_PAD_MS,
    )


def test_window_span_clamps_at_zero_and_floors_length() -> None:
    start, end = confirm_window_span(0, 100)
    assert start == 0
    assert end - start == WINDOW_MIN_MS


def test_window_span_caps_length() -> None:
    start, end = confirm_window_span(10_000, 60_000)
    assert end - start == WINDOW_MAX_MS
    assert start == 10_000 - WINDOW_PAD_MS


def fake_handle(span_scores: list[list[float]]) -> FaConfirmHandle:
    """Handle whose aligner returns fixed per-word token-span scores."""
    import torch

    def model(waveform: torch.Tensor) -> tuple[torch.Tensor, Any]:
        return torch.zeros(1, 4, 2), None

    def tokenizer(transcript: list[str]) -> Any:
        return transcript

    def aligner(emission: torch.Tensor, tokens: Any) -> Any:
        return [
            [SimpleNamespace(score=score) for score in scores] for scores in span_scores
        ]

    return FaConfirmHandle(model=model, tokenizer=tokenizer, aligner=aligner)


def window(seconds: float) -> np.ndarray:
    return np.zeros(int(SAMPLE_RATE * seconds), dtype=np.float32)


def test_score_window_means_token_spans_per_word() -> None:
    handle = fake_handle([[0.8, 0.6], [1.0]])
    result = score_window(handle, window(2.0), SAMPLE_RATE, ["a", "b"])
    assert result.word_count == 2
    assert result.mean_word_score == pytest.approx((0.7 + 1.0) / 2)
    assert result.min_word_score == pytest.approx(0.7)


def test_score_window_rejects_wrong_sample_rate() -> None:
    handle = fake_handle([[1.0]])
    with pytest.raises(ValueError, match="calibrated at 16000"):
        score_window(handle, window(1.0), 44_100, ["a"])


def test_score_window_rejects_empty_words() -> None:
    handle = fake_handle([])
    with pytest.raises(ValueError, match="at least one alignable word"):
        score_window(handle, window(1.0), SAMPLE_RATE, [])
