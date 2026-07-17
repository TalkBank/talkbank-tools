"""Pitch-band verify engine: child vs adult voice, per placed span.

One of the three placement-verification engines behind `merge-verify`
(with forced-alignment confirmation and the machine ear). Probes
fundamental frequency over an audio span with librosa's pyin and bands
the voiced frames into a CHILD / ADULT / AMBIGUOUS verdict.

CALIBRATION-LOCKED: every constant here (sample rate, frame/hop, f0
search range, the 250 Hz child band floor, the 0.50/0.20 verdict
fractions, the 8-voiced-frame floor) matches the pipeline that was
human-calibrated in July 2026 (97.5% measured auto-trust precision).
Changing any of them, or substituting a different f0 estimator,
invalidates that calibration; recalibrate against blind listening
verdicts before shipping such a change.

The whisper caveat: whispered speech carries almost no voicing, so this
engine returns AMBIGUOUS on whispers; whispered ADULT speech is the
measured residual failure mode of the composed rule (~2.5%).
"""

from __future__ import annotations

from dataclasses import dataclass
from enum import Enum
from typing import TYPE_CHECKING

if TYPE_CHECKING:
    import numpy as np

SAMPLE_RATE = 16_000
FRAME_LENGTH = 2048
HOP_LENGTH = 512
F0_MIN_HZ = 80.0
F0_MAX_HZ = 500.0
# Child band floor: young-child voiced f0 sits above the adult-female
# range; the overlap zone (older child vs high adult female) is what the
# AMBIGUOUS verdict and human review are for.
CHILD_BAND_FLOOR_HZ = 250.0
CHILD_VERDICT_FRACTION = 0.50
ADULT_VERDICT_FRACTION = 0.20
MIN_VOICED_FRAMES = 8


class PitchBandVerdict(Enum):
    """Whose voice a probed span carries, per f0 banding."""

    CHILD = "child"
    ADULT = "adult"
    AMBIGUOUS = "ambiguous"


@dataclass(frozen=True, slots=True)
class SpanPitchBand:
    """Pitch statistics plus verdict for one probed span.

    `f0_median_hz` / `child_fraction` are None when the span had no
    voiced frames (the verdict is then necessarily AMBIGUOUS).
    """

    voiced_frames: int
    f0_median_hz: float | None
    child_fraction: float | None
    verdict: PitchBandVerdict


def verdict_of(voiced_frames: int, child_fraction: float | None) -> PitchBandVerdict:
    """Band a span's child-pitched fraction into a verdict."""
    if voiced_frames < MIN_VOICED_FRAMES or child_fraction is None:
        return PitchBandVerdict.AMBIGUOUS
    if child_fraction >= CHILD_VERDICT_FRACTION:
        return PitchBandVerdict.CHILD
    if child_fraction <= ADULT_VERDICT_FRACTION:
        return PitchBandVerdict.ADULT
    return PitchBandVerdict.AMBIGUOUS


def band_audio_span(
    audio: np.ndarray,
    sample_rate: int,
    span_start_s: float,
    span_end_s: float,
) -> SpanPitchBand:
    """Pitch-band one span of mono audio.

    `audio` is float mono at `sample_rate` (callers resample to
    SAMPLE_RATE upstream; a mismatched rate raises rather than silently
    mis-banding, because the f0 search range is calibration-locked).
    """
    # Heavy import kept local so module import stays cheap for callers
    # that only need the verdict types.
    import librosa
    import numpy as np

    if sample_rate != SAMPLE_RATE:
        msg = f"pitch banding is calibrated at {SAMPLE_RATE} Hz; got {sample_rate}"
        raise ValueError(msg)

    f0, voiced_flag, _voiced_prob = librosa.pyin(
        audio,
        fmin=F0_MIN_HZ,
        fmax=F0_MAX_HZ,
        sr=sample_rate,
        frame_length=FRAME_LENGTH,
        hop_length=HOP_LENGTH,
    )
    times = librosa.times_like(f0, sr=sample_rate, hop_length=HOP_LENGTH)

    selector = (times >= span_start_s) & (times <= span_end_s) & voiced_flag
    voiced_f0 = f0[selector]
    voiced_f0 = voiced_f0[~np.isnan(voiced_f0)]
    if len(voiced_f0) == 0:
        return SpanPitchBand(
            voiced_frames=0,
            f0_median_hz=None,
            child_fraction=None,
            verdict=PitchBandVerdict.AMBIGUOUS,
        )
    child_fraction = round(float(np.mean(voiced_f0 > CHILD_BAND_FLOOR_HZ)), 3)
    voiced_frames = int(len(voiced_f0))
    return SpanPitchBand(
        voiced_frames=voiced_frames,
        f0_median_hz=round(float(np.median(voiced_f0)), 1),
        child_fraction=child_fraction,
        verdict=verdict_of(voiced_frames, child_fraction),
    )
