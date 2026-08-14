"""Deterministic tests for the pitch-band verify engine.

Synthesized sine tones give exact ground truth: a 320 Hz tone sits in
the child band (above the 250 Hz floor), a 140 Hz tone in the adult
band, and silence has no voiced frames. No audio fixtures, no models.
"""

from __future__ import annotations

import numpy as np
import pytest

from batchalign.inference.pitch_band import (
    ADULT_VERDICT_FRACTION,
    CHILD_VERDICT_FRACTION,
    MIN_VOICED_FRAMES,
    SAMPLE_RATE,
    PitchBandVerdict,
    band_audio_span,
    verdict_of,
)

CHILD_TONE_HZ = 320.0
ADULT_TONE_HZ = 140.0
TONE_SECONDS = 1.0


def sine(frequency_hz: float, seconds: float) -> np.ndarray:
    t = np.arange(int(SAMPLE_RATE * seconds)) / SAMPLE_RATE
    return (0.5 * np.sin(2.0 * np.pi * frequency_hz * t)).astype(np.float32)


def test_child_tone_bands_child() -> None:
    result = band_audio_span(
        sine(CHILD_TONE_HZ, TONE_SECONDS), SAMPLE_RATE, 0.0, TONE_SECONDS
    )
    assert result.verdict is PitchBandVerdict.CHILD
    assert result.f0_median_hz is not None
    assert result.f0_median_hz == pytest.approx(CHILD_TONE_HZ, rel=0.05)


def test_adult_tone_bands_adult() -> None:
    result = band_audio_span(
        sine(ADULT_TONE_HZ, TONE_SECONDS), SAMPLE_RATE, 0.0, TONE_SECONDS
    )
    assert result.verdict is PitchBandVerdict.ADULT
    assert result.f0_median_hz is not None
    assert result.f0_median_hz == pytest.approx(ADULT_TONE_HZ, rel=0.05)


def test_silence_is_ambiguous_with_no_voicing() -> None:
    silence = np.zeros(SAMPLE_RATE, dtype=np.float32)
    result = band_audio_span(silence, SAMPLE_RATE, 0.0, 1.0)
    assert result.verdict is PitchBandVerdict.AMBIGUOUS
    assert result.voiced_frames == 0
    assert result.f0_median_hz is None
    assert result.child_fraction is None


def test_span_selection_bands_only_the_span() -> None:
    # Adult tone for 1 s, then child tone for 1 s; probing only the
    # second half must band CHILD.
    audio = np.concatenate([sine(ADULT_TONE_HZ, 1.0), sine(CHILD_TONE_HZ, 1.0)])
    result = band_audio_span(audio, SAMPLE_RATE, 1.1, 1.9)
    assert result.verdict is PitchBandVerdict.CHILD


def test_wrong_sample_rate_is_rejected() -> None:
    with pytest.raises(ValueError, match="calibrated at 16000"):
        band_audio_span(sine(CHILD_TONE_HZ, 0.5), 44_100, 0.0, 0.5)


def test_verdict_banding_thresholds_are_calibration_locked() -> None:
    floor = MIN_VOICED_FRAMES
    assert verdict_of(floor - 1, 1.0) is PitchBandVerdict.AMBIGUOUS
    assert verdict_of(floor, None) is PitchBandVerdict.AMBIGUOUS
    assert verdict_of(floor, CHILD_VERDICT_FRACTION) is PitchBandVerdict.CHILD
    assert verdict_of(floor, ADULT_VERDICT_FRACTION) is PitchBandVerdict.ADULT
    between = (CHILD_VERDICT_FRACTION + ADULT_VERDICT_FRACTION) / 2.0
    assert verdict_of(floor, between) is PitchBandVerdict.AMBIGUOUS
