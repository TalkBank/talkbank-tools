"""Long-audio chunking and runaway detection for the native Qwen3-ASR engine.

Two jobs that the ``qwen-asr`` sidecar used to do for us, and that we own now
that BA3 drives ``transformers``' ``qwen3_asr`` module directly.

**Chunking.** The native path has no long-audio handling of its own: a whole
session fed to it does not merely score worse, it does not complete. This is a
port of the sidecar's ``split_audio_into_chunks``, which cuts at the quietest
window near each target boundary so a chunk edge lands in silence rather than
mid-word. Keeping the same algorithm is deliberate: the migration was decided
on a CER differential measured under this exact chunking, so changing it here
would invalidate the evidence that justified the change.

**Coordinate spaces.** The forced aligner returns times relative to THE AUDIO
IT WAS GIVEN, which under chunking is a 180-second window, not the recording.
Turning one into the other is a single addition that is invisible when omitted
and produces timings that look entirely reasonable until they land past the end
of the file. That has happened here before: FA emitted word timings 28.2
seconds past the end of their own audio on 6 of 226 sessions and went unnoticed
for two months. So the two spaces are named apart, and ``AudioChunk.to_file``
is the only conversion between them.

**Runaway detection.** Greedy decoding on this model can fall into a repetition
loop and emit until it hits the token cap. Measured on the 18-fixture Cantonese
benchmark: one chunk produced 4,584 characters for 179.6 seconds of audio while
every other chunk in the corpus stayed under 4.4 characters per second. A
silent hallucination of that size is worse than a crash, because nothing
downstream can tell it from speech.
"""

from __future__ import annotations

from dataclasses import dataclass

import numpy as np

# Seconds measured from the start of the CHUNK the model was handed.
type WindowSeconds = float
# Seconds measured from the start of the recording. What %wor needs.
type FileSeconds = float

# Qwen3-ASR's audio front end expects 16 kHz.
SAMPLE_RATE = 16000

# The model has a minimum input length; a short tail is zero-padded up to it.
MIN_ASR_INPUT_SECONDS = 0.5

# The sidecar picks between a 1200 s cap for plain ASR and a 180 s cap when
# timestamps are requested. We always want timestamps, so 180 is the one that
# governs. NOTE: the effective ceiling is this plus SEARCH_EXPAND_SECONDS,
# because the boundary search may move a cut later as well as earlier.
MAX_FORCE_ALIGN_INPUT_SECONDS = 180.0
SEARCH_EXPAND_SECONDS = 5.0
MIN_ENERGY_WINDOW_MS = 100.0

# Characters of transcript per second of audio, above which output is treated
# as a decoding runaway rather than speech. Measured, not guessed, over 119
# chunks of the Cantonese benchmark: the median is 2.9, the densest legitimate
# chunk is 4.3, and the one degenerate chunk is 25.5. Eight sits nearly twice
# above anything real we have seen and three times below the failure, so there
# is no plausible transcript in the gap.
MAX_CHARS_PER_SECOND = 8.0


@dataclass(frozen=True)
class AudioChunk:
    """One slice of audio, and where it starts in the recording.

    The offset is carried WITH the samples rather than alongside them, because
    the pairing of a chunk with its own offset is the thing that must never be
    got wrong, and a pair that travels together cannot be mismatched.
    """

    samples: np.ndarray
    offset: FileSeconds

    @property
    def seconds(self) -> float:
        return len(self.samples) / float(SAMPLE_RATE)

    def to_file(self, t: WindowSeconds) -> FileSeconds:
        """Convert a model-reported time into a recording-relative one.

        The only sanctioned route between the two spaces. Everything the
        aligner returns is window-relative and must come through here.
        """
        return t + self.offset


@dataclass(frozen=True)
class RunawayOutput:
    """A chunk whose transcript was too dense to be speech.

    Returned rather than merely logged. The caller has to decide what reaches
    the transcript, and a refusal that exists only in a log line is invisible
    to the artifact, unqueryable afterwards, and tested by nothing.
    """

    offset: FileSeconds
    audio_seconds: float
    chars: int
    limit: float

    @property
    def chars_per_second(self) -> float:
        """Derived, not stored, so it cannot disagree with its own inputs."""
        return self.chars / self.audio_seconds

    def describe(self) -> str:
        return (
            f"{self.chars} chars for {self.audio_seconds:.1f}s of audio at "
            f"{self.offset:.1f}s ({self.chars_per_second:.1f} chars/s, "
            f"limit {self.limit:.1f})"
        )


def detect_runaway(
    text: str,
    chunk: AudioChunk,
    limit: float = MAX_CHARS_PER_SECOND,
) -> RunawayOutput | None:
    """Classify one chunk's transcript as speech or runaway.

    Returns ``None`` for ordinary output, so the caller branches on a value
    that names the problem rather than on a bare boolean.
    """
    seconds = chunk.seconds
    if seconds <= 0:
        return None
    density = len(text) / seconds
    if density <= limit:
        return None
    return RunawayOutput(
        offset=chunk.offset,
        audio_seconds=seconds,
        chars=len(text),
        limit=limit,
    )


def split_audio_into_chunks(
    wav: np.ndarray,
    sr: int = SAMPLE_RATE,
    max_chunk_sec: float = MAX_FORCE_ALIGN_INPUT_SECONDS,
    search_expand_sec: float = SEARCH_EXPAND_SECONDS,
    min_window_ms: float = MIN_ENERGY_WINDOW_MS,
) -> list[AudioChunk]:
    """Split audio at low-energy boundaries, as the sidecar did.

    Concatenating the chunks reproduces the input exactly: no overlaps, no
    gaps, so no audio is transcribed twice or dropped. The only exception is a
    final chunk shorter than the model's minimum input, which is zero-padded.
    """
    wav = np.asarray(wav, dtype=np.float32)
    if wav.ndim > 1:
        wav = np.mean(wav, axis=-1).astype(np.float32)

    total_len = int(wav.shape[0])
    if total_len / float(sr) <= max_chunk_sec:
        return [AudioChunk(samples=wav, offset=0.0)]

    max_len = int(max_chunk_sec * sr)
    expand = int(search_expand_sec * sr)
    win = max(4, int((min_window_ms / 1000.0) * sr))

    chunks: list[AudioChunk] = []
    start = 0
    offset_sec: FileSeconds = 0.0

    while (total_len - start) > max_len:
        cut = start + max_len
        left = max(start, cut - expand)
        right = min(total_len, cut + expand)

        if right - left <= win:
            boundary = cut
        else:
            # Quietest window in the search band, then the quietest single
            # sample inside it: cut where the speech is not.
            seg_abs = np.abs(wav[left:right])
            window_sums = np.convolve(
                seg_abs, np.ones(win, dtype=np.float32), mode="valid"
            )
            min_pos = int(np.argmin(window_sums))
            inner = int(np.argmin(seg_abs[min_pos : min_pos + win]))
            boundary = left + min_pos + inner

        boundary = int(min(max(boundary, start + 1), total_len))
        chunks.append(AudioChunk(samples=wav[start:boundary], offset=offset_sec))
        offset_sec += (boundary - start) / float(sr)
        start = boundary

    chunks.append(AudioChunk(samples=wav[start:total_len], offset=offset_sec))

    min_len = int(MIN_ASR_INPUT_SECONDS * sr)
    return [
        AudioChunk(
            samples=(
                np.pad(
                    c.samples, (0, min_len - c.samples.shape[0]), mode="constant"
                ).astype(np.float32)
                if c.samples.shape[0] < min_len
                else c.samples
            ),
            offset=c.offset,
        )
        for c in chunks
    ]
