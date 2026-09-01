"""Contracts for the immutable local-Pyannote model graph."""

from __future__ import annotations

from pathlib import Path

import pytest

from batchalign.inference.pyannote_local import (
    PinnedHuggingFaceArtifact,
    _get_pyannote_pipeline,
    load_local_pyannote_model_graph,
)


@pytest.mark.parametrize(
    "value",
    [
        {"repo_id": "owner/model", "revision": "main", "filename": "config.yaml"},
        {
            "repo_id": "owner / model",
            "revision": "a" * 40,
            "filename": "config.yaml",
        },
        {
            "repo_id": "owner/model",
            "revision": "a" * 40,
            "filename": "../outside.bin",
        },
    ],
)
def test_pinned_artifact_refuses_moving_or_escaping_inputs(value: object) -> None:
    """A parsed artifact is proof of an immutable in-repository object."""

    with pytest.raises(ValueError):
        PinnedHuggingFaceArtifact.parse(value, field="test")


def test_get_pyannote_pipeline_loads_only_pinned_artifacts(
    monkeypatch, tmp_path: Path
) -> None:
    """The runtime must consume the same immutable graph named by the cache."""

    graph = load_local_pyannote_model_graph()
    config_path = tmp_path / "config.yaml"
    config_path.write_text(
        "pipeline:\n"
        "  name: pyannote.audio.pipelines.SpeakerDiarization\n"
        "  params:\n"
        "    segmentation: moving-segmentation-head\n"
        "    embedding: moving-embedding-head\n"
        "params: {}\n"
    )
    embedding_path = tmp_path / "speaker-embedding.onnx"
    embedding_path.write_bytes(b"fixture")
    downloads: list[tuple[str, str, str]] = []

    def fake_download(*, repo_id: str, filename: str, revision: str) -> str:
        downloads.append((repo_id, filename, revision))
        if repo_id == graph.pipeline.repo_id:
            return str(config_path)
        if repo_id == graph.embedding.repo_id:
            return str(embedding_path)
        raise AssertionError(f"unexpected eager download: {repo_id}")

    loaded: list[dict[str, object]] = []

    monkeypatch.setattr(
        "batchalign.inference.pyannote_local.hf_hub_download", fake_download
    )
    monkeypatch.setattr(
        "batchalign.inference.pyannote_local._load_pipeline",
        lambda config: loaded.append(config) or {"pipeline": config},
    )
    monkeypatch.setattr(
        "batchalign.inference.pyannote_local._emit_model_download_if_missing",
        lambda *_args, **_kwargs: None,
    )
    monkeypatch.setattr("batchalign.inference.pyannote_local._PYANNOTE_PIPELINE", None)

    first = _get_pyannote_pipeline()
    second = _get_pyannote_pipeline()

    assert first is second
    assert downloads == [
        (graph.pipeline.repo_id, graph.pipeline.filename, graph.pipeline.revision),
        (graph.embedding.repo_id, graph.embedding.filename, graph.embedding.revision),
    ]
    pipeline = loaded[0]["pipeline"]
    assert isinstance(pipeline, dict)
    params = pipeline["params"]
    assert isinstance(params, dict)
    assert params["segmentation"] == {
        "checkpoint": graph.segmentation.repo_id,
        "revision": graph.segmentation.revision,
    }
    assert params["embedding"] == str(embedding_path)
