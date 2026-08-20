"""Chunking and runaway detection for the native Qwen3-ASR engine.

These are the two jobs BA3 took over from the `qwen-asr` package when the
engine moved onto `transformers`' native `qwen3_asr` module, and they are
where its remaining risk lives. Neither needs a model, so these run in
milliseconds.

What each property is protecting:

* **Exact reconstruction.** A splitter that dropped or duplicated samples
  would still produce a fluent transcript, so nothing downstream would notice
  audio going missing.
* **Offsets.** The forced aligner reports times against the window it was
  handed. The offset is what makes them recording-relative, and omitting it
  produces timings that look reasonable and are silently wrong. FA emitted
  word timings 28.2 seconds past the end of their own audio on 6 of 226
  sessions once, undetected for two months, for exactly this reason.
* **Runaway detection.** Greedy decoding can fall into a repetition loop and
  emit until it hits the token cap, which puts thousands of characters of
  invented text into a transcript with nothing marking it as invented.
"""

from __future__ import annotations

import numpy as np
import pytest

from batchalign.inference.languages.cantonese._qwen_chunking import (
    MIN_ASR_INPUT_SECONDS,
    SAMPLE_RATE,
    AudioChunk,
    detect_runaway,
    split_audio_into_chunks,
)

SR = SAMPLE_RATE


def _noise(seconds: float, seed: int = 0) -> np.ndarray:
    rng = np.random.default_rng(seed)
    return rng.standard_normal(int(seconds * SR)).astype(np.float32)


def test_short_audio_is_one_chunk_at_zero_offset() -> None:
    chunks = split_audio_into_chunks(_noise(10.0))
    assert len(chunks) == 1
    assert chunks[0].offset == 0.0


def test_chunks_reconstruct_the_original_exactly() -> None:
    wav = _noise(430.0)
    chunks = split_audio_into_chunks(wav)
    assert len(chunks) > 1
    np.testing.assert_array_equal(np.concatenate([c.samples for c in chunks]), wav)


def test_offsets_are_the_true_start_time_of_each_chunk() -> None:
    """Each offset must be exactly where its chunk begins in the recording.

    This is the number added to every word timing, so an offset that merely
    looks plausible is not good enough.
    """
    wav = _noise(500.0)
    chunks = split_audio_into_chunks(wav)

    expected = 0.0
    for chunk in chunks:
        assert chunk.offset == pytest.approx(expected, abs=1e-9)
        expected += chunk.seconds
    assert expected == pytest.approx(len(wav) / SR, abs=1e-9)


def test_to_file_shifts_a_window_time_by_the_chunk_offset() -> None:
    chunk = AudioChunk(samples=_noise(5.0), offset=180.0)
    assert chunk.to_file(0.0) == 180.0
    assert chunk.to_file(2.5) == 182.5


def test_the_effective_cap_exceeds_the_named_one_by_the_search_window() -> None:
    """A chunk can run past `max_chunk_sec`, by up to `search_expand_sec`.

    Worth pinning because it is easy to assume otherwise: the boundary search
    looks on BOTH sides of the target cut, so the real ceiling is 185 s rather
    than the 180 s the constant names. Anything downstream keyed to 180 has
    five seconds of slack it did not ask for.
    """
    chunks = split_audio_into_chunks(_noise(700.0))
    longest = max(c.seconds for c in chunks)
    assert longest <= 185.0 + 1e-6
    assert longest > 180.0


def test_a_short_tail_is_padded_up_to_the_model_minimum() -> None:
    """The one place chunking does not preserve the input, pinned deliberately.

    The search window is switched off so the cut lands on the target and the
    tail length is a fact about the input rather than about where the audio
    happened to go quiet.
    """
    chunks = split_audio_into_chunks(_noise(180.1), search_expand_sec=0.0)
    tail = chunks[-1]
    assert len(tail.samples) == int(MIN_ASR_INPUT_SECONDS * SR)
    assert tail.samples[-1] == 0.0


def test_ordinary_speech_density_is_not_a_runaway() -> None:
    """The densest legitimate chunk measured across the whole Cantonese
    benchmark was 4.3 characters per second; the median was 2.9."""
    chunk = AudioChunk(samples=_noise(180.0), offset=0.0)
    assert detect_runaway("x" * 780, chunk) is None


def test_the_measured_runaway_is_caught() -> None:
    """The real failure: 4,584 characters for 179.6 s of audio, 25.5 per
    second, on fixture A020 of the Cantonese benchmark."""
    chunk = AudioChunk(samples=_noise(179.6), offset=360.0)
    runaway = detect_runaway("x" * 4584, chunk)

    assert runaway is not None
    assert runaway.chars == 4584
    assert runaway.chars_per_second == pytest.approx(25.5, abs=0.2)
    # The offset travels with the refusal so an operator can find the audio.
    assert runaway.offset == 360.0
    assert "4584 chars" in runaway.describe()


def test_an_empty_chunk_is_not_reported_as_a_runaway() -> None:
    """Zero-length audio would divide by zero; silence is not a runaway."""
    assert (
        detect_runaway(
            "anything", AudioChunk(samples=np.array([], dtype=np.float32), offset=0.0)
        )
        is None
    )
