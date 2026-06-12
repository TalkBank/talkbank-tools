# affects: batchalign/worker/_model_loading/forced_alignment.py
"""Worker-side acceptance of FA engine names across naming schemes.

Regression guard for the 2026-06-11 incident: the Rust control plane
serialized a user's FA engine override with its persistence wire names
("wav2vec_fa", "whisper_fa", "cantonese_fa"); the worker accepted only
the dispatch names ("wave2vec", "whisper", "wav2vec_canto") and died at
bootstrap, before emitting its ready signal, failing four consecutive
align jobs. The Rust side now emits dispatch names at the worker
boundary, and the worker additionally accepts the legacy persistence
spellings as aliases so that jobs persisted by older builds (whose
stored options carry the legacy names) cannot kill a worker on replay.
"""

from __future__ import annotations

import pytest

from batchalign.worker._model_loading.forced_alignment import resolve_fa_engine
from batchalign.worker._types import FaEngine


@pytest.mark.parametrize(
    ("wire_name", "expected"),
    [
        # Dispatch names (what current Rust builds send).
        ("wave2vec", FaEngine.WAVE2VEC),
        ("whisper", FaEngine.WHISPER),
        ("wav2vec_canto", FaEngine.WAV2VEC_CANTO),
        # Legacy Rust persistence wire names (jobs stored by builds
        # before 2026-06-12 carry these in their options JSON).
        ("wav2vec_fa", FaEngine.WAVE2VEC),
        ("whisper_fa", FaEngine.WHISPER),
        ("cantonese_fa", FaEngine.WAV2VEC_CANTO),
    ],
)
def test_resolve_fa_engine_accepts_dispatch_and_legacy_names(
    wire_name: str, expected: FaEngine
) -> None:
    assert resolve_fa_engine({"fa": wire_name}) is expected


def test_resolve_fa_engine_still_rejects_unknown_names() -> None:
    with pytest.raises(ValueError, match="unknown fa engine"):
        resolve_fa_engine({"fa": "definitely_not_an_engine"})


def test_resolve_fa_engine_defaults_to_whisper_without_override() -> None:
    assert resolve_fa_engine(None) is FaEngine.WHISPER
    assert resolve_fa_engine({}) is FaEngine.WHISPER
