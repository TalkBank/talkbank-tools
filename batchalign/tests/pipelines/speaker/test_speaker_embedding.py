"""Contracts for span-level speaker embedding over one prepared recording.

The unit under test is the worker-side model host for the
``speaker_embedding`` task: it receives the whole prepared mono PCM view of a
recording plus a list of frame spans, and returns one typed outcome per span.

Why these are tests and not types: every one of them is about the OUTSIDE
world. The model is a file on disk loaded by a third-party library, the
minimum input length is a property of that file rather than of our code, and
the below-minimum behaviour measured here (a silently returned NaN vector) is
an upstream fact no signature of ours can express.
"""

from __future__ import annotations

import numpy as np
import pytest

from batchalign.inference.speaker_embedding import (
    SpanTooShortForEmbedding,
    SpeakerEmbeddingSpanRequest,
    embed_prepared_audio_spans,
    load_speaker_embedding_model,
)


@pytest.fixture(scope="module")
def model() -> object:
    """The pinned local embedding model, or a skip when it cannot be fetched."""

    return load_speaker_embedding_model()


def _tone(sample_rate_hz: int, seconds: float, hz: float) -> np.ndarray:
    """A deterministic non-silent waveform, so the model has something to hear."""

    t = np.arange(int(sample_rate_hz * seconds), dtype=np.float32) / sample_rate_hz
    return (0.2 * np.sin(2.0 * np.pi * hz * t)).astype(np.float32)


@pytest.mark.golden
def test_pinned_model_reports_its_own_minimum_and_dimension(model: object) -> None:
    """The bound a caller must respect is READ from the model, never assumed."""

    assert model.dimension > 0
    assert model.minimum_frames > 0
    assert model.sample_rate_hz == 16000


@pytest.mark.golden
def test_a_span_shorter_than_the_minimum_is_refused_rather_than_embedded(
    model: object,
) -> None:
    """Below its minimum the ONNX graph returns NaN, which reads as a vector.

    Measured on the pinned model: a span one frame short of the minimum comes
    back as a finite-looking float32 array whose every component is NaN. A
    consumer scoring that vector gets a NaN similarity and, depending on the
    comparison, a verdict. The host must therefore refuse the span and say why,
    which is what this test pins.
    """

    sample_rate_hz = model.sample_rate_hz
    audio = _tone(sample_rate_hz, 2.0, 220.0)
    short = model.minimum_frames - 1
    response = embed_prepared_audio_spans(
        audio,
        sample_rate_hz,
        [SpeakerEmbeddingSpanRequest(span_id="short", start_frame=0, end_frame=short)],
        model=model,
    )

    assert response.minimum_frames == model.minimum_frames
    assert len(response.spans) == 1
    outcome = response.spans[0].outcome
    assert isinstance(outcome, SpanTooShortForEmbedding)
    assert outcome.frame_count == short


@pytest.mark.golden
def test_every_requested_span_gets_exactly_one_echoed_outcome(model: object) -> None:
    """Span ids are echoed so the caller never pairs results by position."""

    sample_rate_hz = model.sample_rate_hz
    audio = _tone(sample_rate_hz, 3.0, 220.0)
    requested = [
        SpeakerEmbeddingSpanRequest(
            span_id="a", start_frame=0, end_frame=sample_rate_hz
        ),
        SpeakerEmbeddingSpanRequest(
            span_id="b", start_frame=sample_rate_hz, end_frame=2 * sample_rate_hz
        ),
    ]
    response = embed_prepared_audio_spans(audio, sample_rate_hz, requested, model=model)

    assert [span.span_id for span in response.spans] == ["a", "b"]
    for span in response.spans:
        assert not isinstance(span.outcome, SpanTooShortForEmbedding)
        assert len(span.outcome.vector) == response.dimension
        assert all(np.isfinite(span.outcome.vector))


@pytest.mark.golden
def test_the_same_voice_scores_higher_against_itself_than_against_another(
    model: object,
) -> None:
    """The one behavioural claim the command rests on, stated as a measurement.

    This is an ORDERING claim, not an accuracy claim: it says the embedding
    separates two acoustically different sources, which is the property
    in-session enrollment uses. It deliberately asserts no threshold, because
    no threshold is defensible from synthetic tones.
    """

    sample_rate_hz = model.sample_rate_hz
    voice_a = _tone(sample_rate_hz, 2.0, 180.0)
    voice_b = _tone(sample_rate_hz, 2.0, 900.0)
    audio = np.concatenate([voice_a, voice_b, voice_a])
    second = sample_rate_hz
    response = embed_prepared_audio_spans(
        audio,
        sample_rate_hz,
        [
            SpeakerEmbeddingSpanRequest(
                span_id="a1", start_frame=0, end_frame=2 * second
            ),
            SpeakerEmbeddingSpanRequest(
                span_id="b", start_frame=2 * second, end_frame=4 * second
            ),
            SpeakerEmbeddingSpanRequest(
                span_id="a2", start_frame=4 * second, end_frame=6 * second
            ),
        ],
        model=model,
    )

    vectors = {span.span_id: np.asarray(span.outcome.vector) for span in response.spans}

    def cosine(left: np.ndarray, right: np.ndarray) -> float:
        return float(
            np.dot(left, right) / (np.linalg.norm(left) * np.linalg.norm(right))
        )

    assert cosine(vectors["a1"], vectors["a2"]) > cosine(vectors["a1"], vectors["b"])


@pytest.mark.golden
def test_a_span_outside_the_prepared_audio_is_a_refusal_not_a_short_read(
    model: object,
) -> None:
    """Slicing past the end of a numpy array silently shortens; that is the bug.

    ``audio[start:end]`` with ``end`` past the end returns whatever is there
    instead of failing, so a span the caller got wrong would be embedded from
    the wrong material and reported as a normal result.
    """

    sample_rate_hz = model.sample_rate_hz
    audio = _tone(sample_rate_hz, 1.0, 220.0)
    with pytest.raises(ValueError, match="outside the prepared audio"):
        embed_prepared_audio_spans(
            audio,
            sample_rate_hz,
            [
                SpeakerEmbeddingSpanRequest(
                    span_id="past-end", start_frame=0, end_frame=sample_rate_hz * 2
                )
            ],
            model=model,
        )


@pytest.mark.golden
def test_a_sample_rate_the_model_was_not_trained_at_is_refused(model: object) -> None:
    """A resample the host did not perform must not be assumed to have happened."""

    audio = _tone(8000, 2.0, 220.0)
    with pytest.raises(ValueError, match="sample rate"):
        embed_prepared_audio_spans(
            audio,
            8000,
            [SpeakerEmbeddingSpanRequest(span_id="a", start_frame=0, end_frame=8000)],
            model=model,
        )
