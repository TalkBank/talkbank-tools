"""Audio decoding must not depend on a matching external FFmpeg.

# What these pin, and why a test rather than a type

The failure being guarded is not a wrong value, so no type can refuse it: it is
the PRESENCE of a package. `torchcodec` arrives transitively from
`pyannote.audio` and ships shared libraries for a fixed set of FFmpeg majors,
each linked `@rpath/libavutil.NN.dylib` against the UNVERSIONED Homebrew
prefix. When Homebrew moves that prefix past the supported set, every audio
decode raises, and it raises at JOB time: the daemon is healthy, the CLI
answers, the job submits, and the worker dies in a dynamic-loader traceback.

One upstream package upgrade was enough to stop transcription on every machine.

The exclusion lives in `pyproject.toml` under `[tool.uv] override-dependencies`.
A comment there can be deleted by anyone resolving a merge; these tests fail
loudly instead, which is the closest a dependency set gets to being typed.
"""

from __future__ import annotations

import importlib.util
from pathlib import Path


def test_torchcodec_is_not_installed() -> None:
    """The dependency stays out.

    POLICY, with a real alternative: we could instead keep torchcodec and pin
    an FFmpeg major on every host. That was rejected because it makes a
    clinician's `brew upgrade` a production incident, and because external
    installers cannot be reached at all.
    """
    assert importlib.util.find_spec("torchcodec") is None, (
        "torchcodec is installed again. It binds a fixed set of FFmpeg majors "
        "through an unversioned rpath, so its presence makes every audio "
        "decode hostage to the machine's FFmpeg version. Restore the "
        "`override-dependencies` entry in pyproject.toml."
    )


def test_transformers_does_not_reach_for_torchcodec() -> None:
    """The consumer that actually broke us must see it as unavailable.

    `transformers.pipelines.automatic_speech_recognition.preprocess` runs
    `if is_torchcodec_available(): import torchcodec` UNCONDITIONALLY, for
    every input type. The flag is a cached `find_spec`, so with the package
    absent the branch is skipped and the import never executes. That is the
    exact line that failed every transcription.
    """
    from transformers.utils import is_torchcodec_available

    assert not is_torchcodec_available()


def test_audio_still_decodes() -> None:
    """The capability, which is what actually matters.

    Absence of torchcodec is only good news if audio still decodes, so this
    asserts the positive.

    It calls the SAME function the Rust doctor and the fleet deploy call out of
    process, rather than repeating its five steps inline. This test used to
    carry the third copy of that probe; the copies had already diverged in
    field count and error handling, so a green test here proved nothing about
    what the other two would report.
    """
    from batchalign.inference.audio import verify_decode_capability

    capability = verify_decode_capability()
    assert capability.sample_rate == 16000
    assert capability.frames == 16000


def test_decode_capability_json_is_the_cross_language_contract() -> None:
    """The JSON line is what two out-of-process callers parse.

    WIRE FORMAT, which is one of the things a type cannot pin: the Rust doctor
    deserializes these exact keys, and a rename here would break it silently at
    a language boundary no compiler spans.
    """
    import json

    from batchalign.inference.audio import DecodeCapability

    encoded = json.loads(DecodeCapability(frames=16000, sample_rate=16000).to_json())
    assert encoded == {"frames": 16000, "sample_rate": 16000}


def test_soundfile_backend_install_is_idempotent() -> None:
    """Repeated bootstraps report honestly instead of rewrapping.

    The outcome is returned rather than `None` precisely so this is assertable:
    a second install must say it replaced nothing. Without that, repeated
    worker starts would nest the shim inside itself once per call.
    """
    from batchalign.inference.audio import (
        SoundfileBackendInstall,
        install_soundfile_backend,
    )

    install_soundfile_backend()
    assert install_soundfile_backend() is SoundfileBackendInstall.ALREADY_INSTALLED


def test_funasr_still_loads_audio_when_torchaudio_load_raises() -> None:
    """The assumption the soundfile shim's deletion condition rests on.

    `install_soundfile_backend` exists to keep `torchaudio.load` away from
    torchcodec, and its docstring argues it is NOT load-bearing because its one
    real caller, FunASR, wraps that call in its own `except` and falls back to
    `soundfile.read`. Everything about deleting the shim depends on that
    remaining true.

    It is exactly the kind of fact a dependency bump breaks silently: FunASR
    could tighten its `except`, or drop the fallback, and nothing else in this
    suite would notice. So it is pinned here rather than left to a one-off run,
    which is evidence with no expiry date and no alarm.

    POLICY, not an invariant: we are asserting something about somebody else's
    package, which no type of ours can hold.
    """
    import pytest

    if importlib.util.find_spec("funasr") is None:
        pytest.skip("funasr is not installed")

    import torchaudio
    from funasr.utils.load_utils import load_audio_text_image_video

    fixture = (
        Path(__file__).resolve().parents[1]
        / "languages"
        / "cantonese"
        / "fixtures"
        / "05b_clip.wav"
    )
    if not fixture.is_file():
        pytest.skip(f"missing audio fixture: {fixture}")

    refusals = 0

    def _refuse(*_args: object, **_kwargs: object) -> object:
        nonlocal refusals
        refusals += 1
        raise RuntimeError("torchaudio.load is unavailable in this test")

    original = torchaudio.load
    torchaudio.load = _refuse  # type: ignore[assignment]
    try:
        loaded = load_audio_text_image_video(str(fixture), fs=16000)
    finally:
        torchaudio.load = original  # type: ignore[assignment]

    # Without this the test passes VACUOUSLY if FunASR stops calling
    # `torchaudio.load` at all, which is one of the ways the assumption could
    # change. A fallback that is never reached proves nothing about a fallback.
    assert refusals > 0, (
        "FunASR never called torchaudio.load, so this test exercised no fallback"
    )

    # A fallback that returned an empty buffer would read as a pass, which is
    # the failure mode this whole file exists to refuse.
    assert int(loaded.numel()) > 1000, (
        "FunASR reached a fallback but produced no audio, so the shim's "
        "deletion condition no longer holds"
    )
