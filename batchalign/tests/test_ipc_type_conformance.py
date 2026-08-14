"""Verify that hand-written Python Pydantic models conform to generated Rust schemas.

This test catches Rust/Python IPC type drift at CI time. When a Rust type
changes shape, the generated schemas update (via ``scripts/generate_ipc_types.sh``)
and this test fails until the hand-written Python model is updated to match.

This is not a bridge to generated Python models; that plan was retired on
2026-08-14 because generation would have cost the domain types and validators
the hand-written models carry. See ``ipc-schema/`` for the JSON Schema files
and the "Rust to Python IPC Type Sync" developer page for why one
representation per language, held to one schema, beats a generated mirror.

Cross-language contract note: this is the Python half of the schema conformance
gate. The Rust half lives in ``crates/batchalign/tests/worker_protocol_v2_compat.rs``.
Both sides must pass independently, a change to the wire format must update both.
The ``Cmd2Task`` constant map (formerly tested in ``test_runtime.py``) is also
covered by the IPC schema drift check in CI (``scripts/check_ipc_type_drift.sh``).
"""

from __future__ import annotations

import json
from pathlib import Path

import pytest

# Find project root by looking for Cargo.toml
_here = Path(__file__).resolve().parent
ROOT = _here
while ROOT != ROOT.parent:
    if (ROOT / "Cargo.toml").exists() and (ROOT / "ipc-schema").exists():
        break
    ROOT = ROOT.parent
SCHEMA_DIR = ROOT / "ipc-schema"


def _load_schema(layer: str, type_name: str) -> dict:
    """Load a JSON Schema file for an IPC type."""
    path = SCHEMA_DIR / layer / f"{type_name}.json"
    if not path.exists():
        # Not a skip. A missing schema means the Rust type was renamed or
        # dropped without this test being updated, which is exactly the drift
        # the test exists to catch; skipping would report it as a pass.
        pytest.fail(
            f"Schema not found: {path}. Run 'bash scripts/generate_ipc_types.sh', "
            "or update this test if the Rust type was renamed."
        )
    return json.loads(path.read_text())


def _assert_fields_match(
    schema: dict, model_cls: type, *, known_extra_python: frozenset[str] = frozenset()
) -> None:
    """Assert that the schema's required/optional fields match the Pydantic model."""
    props = schema.get("properties", {})
    required = set(schema.get("required", []))

    model_fields = set(model_cls.model_fields.keys())
    schema_fields = set(props.keys())

    # Every schema field must exist in the Python model
    missing_from_python = schema_fields - model_fields
    assert not missing_from_python, (
        f"{model_cls.__name__} is missing fields defined in Rust schema: {missing_from_python}"
    )

    # Required-ness must agree on the fields both sides define. Without this the
    # helper compared only field NAMES, so a field the Rust schema demands and
    # the Python model defaults could round-trip a missing value silently.
    python_required = {
        name for name in schema_fields if model_cls.model_fields[name].is_required()
    }
    assert python_required == required, (
        f"{model_cls.__name__} required-field mismatch vs the Rust schema: "
        f"required only in Python={sorted(python_required - required)}, "
        f"required only in Rust={sorted(required - python_required)}"
    )

    # Extras must be NAMED, not merely permitted. A boolean here allows any
    # field at all, which is how a dead `lang` on UtsegBatchItem survived
    # unnoticed: the flag that admitted the legitimate extra admitted it too.
    unexpected_in_python = model_fields - schema_fields - known_extra_python
    assert not unexpected_in_python, (
        f"{model_cls.__name__} has fields not in the Rust schema and not "
        f"declared as known extras: {unexpected_in_python}"
    )
    stale_allowances = known_extra_python & schema_fields
    assert not stale_allowances, (
        f"{model_cls.__name__} declares {sorted(stale_allowances)} as Python-only, "
        "but the Rust schema now has them: drop the allowance"
    )


class TestBatchItemConformance:
    """Verify batch item types match Rust schemas."""

    def test_morphosyntax_batch_item(self) -> None:
        from batchalign.inference.morphosyntax import MorphosyntaxBatchItem

        schema = _load_schema("batch_items", "MorphosyntaxBatchItem")
        _assert_fields_match(schema, MorphosyntaxBatchItem)

    def test_utseg_batch_item(self) -> None:
        from batchalign.inference.utseg import UtsegBatchItem

        schema = _load_schema("batch_items", "UtsegBatchItem")
        _assert_fields_match(schema, UtsegBatchItem)

    def test_translate_batch_item(self) -> None:
        from batchalign.inference.translate import TranslateBatchItem

        schema = _load_schema("batch_items", "TranslateBatchItem")
        _assert_fields_match(schema, TranslateBatchItem)

    def test_coref_batch_item(self) -> None:
        from batchalign.inference.coref import CorefBatchItem

        schema = _load_schema("batch_items", "CorefBatchItem")
        _assert_fields_match(schema, CorefBatchItem)

    def test_chain_ref(self) -> None:
        from batchalign.inference.coref import ChainRef

        schema = _load_schema("batch_items", "ChainRef")
        _assert_fields_match(schema, ChainRef)


class TestWorkerV2Conformance:
    """Verify selected V2 protocol types match Rust schemas."""

    def test_execute_request(self) -> None:
        from batchalign.worker._types_v2 import ExecuteRequestV2

        schema = _load_schema("worker_v2", "ExecuteRequestV2")
        _assert_fields_match(schema, ExecuteRequestV2)

    def test_execute_response(self) -> None:
        from batchalign.worker._types_v2 import ExecuteResponseV2

        schema = _load_schema("worker_v2", "ExecuteResponseV2")
        _assert_fields_match(schema, ExecuteResponseV2)

    def test_morphosyntax_item_result(self) -> None:
        from batchalign.worker._types_v2 import MorphosyntaxItemResultV2

        schema = _load_schema("worker_v2", "MorphosyntaxItemResultV2")
        _assert_fields_match(schema, MorphosyntaxItemResultV2)

    def test_whisper_chunk_span(self) -> None:
        from batchalign.worker._types_v2 import WhisperChunkSpanV2

        schema = _load_schema("worker_v2", "WhisperChunkSpanV2")
        _assert_fields_match(schema, WhisperChunkSpanV2)

    def test_asr_element(self) -> None:
        from batchalign.worker._types_v2 import AsrElementV2

        schema = _load_schema("worker_v2", "AsrElementV2")
        _assert_fields_match(schema, AsrElementV2)

    def test_indexed_word_timing(self) -> None:
        from batchalign.worker._types_v2 import IndexedWordTimingV2

        schema = _load_schema("worker_v2", "IndexedWordTimingV2")
        _assert_fields_match(schema, IndexedWordTimingV2)

    def test_speaker_segment(self) -> None:
        from batchalign.worker._types_v2 import SpeakerSegmentV2

        schema = _load_schema("worker_v2", "SpeakerSegmentV2")
        _assert_fields_match(schema, SpeakerSegmentV2)

    def test_morphosyntax_request(self) -> None:
        from batchalign.worker._types_v2 import MorphosyntaxRequestV2

        schema = _load_schema("worker_v2", "MorphosyntaxRequestV2")
        # Python adds `kind` for Pydantic discrimination; Rust schema doesn't
        # include it (added by the tagged enum wrapper at serialization time).
        _assert_fields_match(
            schema, MorphosyntaxRequestV2, known_extra_python=frozenset({"kind"})
        )

    def test_forced_alignment_request(self) -> None:
        from batchalign.worker._types_v2 import ForcedAlignmentRequestV2

        schema = _load_schema("worker_v2", "ForcedAlignmentRequestV2")
        _assert_fields_match(
            schema, ForcedAlignmentRequestV2, known_extra_python=frozenset({"kind"})
        )

    def test_asr_request(self) -> None:
        from batchalign.worker._types_v2 import AsrRequestV2

        schema = _load_schema("worker_v2", "AsrRequestV2")
        _assert_fields_match(
            schema, AsrRequestV2, known_extra_python=frozenset({"kind"})
        )
