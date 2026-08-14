"""Tests for FA worker bootstrap engine selection.

Mirrors the ASR engine-resolution tests in
``batchalign/tests/pipelines/asr/test_asr_model_loading.py`` and the
translate ones in
``batchalign/tests/pipelines/translate/test_translation_model_loading.py``.

The FA resolver previously lived inline in ``load_fa_engine`` and
silently fell through to Wave2Vec on any unknown wire string. These
tests pin the extracted typed resolver behavior so the silent
fallthrough cannot recur.
"""

from __future__ import annotations

import pytest

from batchalign.worker._model_loading.forced_alignment import resolve_fa_engine
from batchalign.worker._types import FaEngine


class TestResolveFaEngine:
    """Engine selection must stay deterministic, typed, and loud on bad input."""

    def test_whisper_override_wins(self) -> None:
        assert resolve_fa_engine({"fa": "whisper"}) is FaEngine.WHISPER

    def test_wave2vec_override_wins(self) -> None:
        assert resolve_fa_engine({"fa": "wave2vec"}) is FaEngine.WAVE2VEC

    def test_wav2vec_canto_override_wins(self) -> None:
        assert resolve_fa_engine({"fa": "wav2vec_canto"}) is FaEngine.WAV2VEC_CANTO

    def test_absence_is_a_boundary_bug_not_a_request_for_a_default(self) -> None:
        """Nothing may reach here without an engine named.

        The history is the reason this test is worded as a boundary check
        rather than a default. It asserted a Whisper default until 2026-08-14,
        which is how six weeks of onset-only timings shipped; then a raise,
        which broke profile bootstrap because the control plane genuinely sent
        no engine; then Wave2Vec, which was correct but left the guess in
        place.

        The guess is gone because the absence is: `EngineSelection::for_target`
        names an engine for every task a worker's target preloads, and a
        lazy-profile worker preloads nothing and names its engine in the
        `ensure_task` IPC. So an empty selection here means the control plane
        is broken, and loading *some* model would hide that behind output
        nobody can tell is wrong.
        """
        for absent in (None, {}, {"asr": "whisper"}):
            with pytest.raises(ValueError, match="no 'fa' engine"):
                resolve_fa_engine(absent)

    def test_unknown_engine_raises_value_error(self) -> None:
        with pytest.raises(ValueError, match="unknown fa engine 'wisper'"):
            resolve_fa_engine({"fa": "wisper"})

    def test_unknown_engine_error_mentions_supported_options(self) -> None:
        # The supported-engines list is derived from the FaEngine
        # enum, so adding a 4th variant requires zero changes here.
        with pytest.raises(ValueError) as exc_info:
            resolve_fa_engine({"fa": "x"})
        msg = str(exc_info.value)
        for variant in FaEngine:
            assert variant.value in msg, (
                f"error message {msg!r} missing variant {variant.value!r}"
            )
