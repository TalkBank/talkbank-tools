"""Immutable local-Pyannote model graph and lazy worker-local loader."""

from __future__ import annotations

import json
import re
from dataclasses import dataclass
from functools import lru_cache
from importlib import resources
from pathlib import Path, PurePosixPath
from typing import Any

from huggingface_hub import hf_hub_download
from omegaconf import OmegaConf

_MODEL_MANIFEST_RESOURCE = "local_pyannote_model.json"
_COMMIT_REVISION = re.compile(r"[0-9a-f]{40}")
_HUB_REPOSITORY_ID = re.compile(
    r"[A-Za-z0-9][A-Za-z0-9._-]*/[A-Za-z0-9][A-Za-z0-9._-]*"
)
_PYANNOTE_PIPELINE: object | None = None


@dataclass(frozen=True)
class PinnedHuggingFaceArtifact:
    """One Hub artifact whose mutable repository name has an exact commit."""

    repo_id: str
    revision: str
    filename: str

    @classmethod
    def parse(cls, value: object, *, field: str) -> PinnedHuggingFaceArtifact:
        if not isinstance(value, dict) or set(value) != {
            "repo_id",
            "revision",
            "filename",
        }:
            raise ValueError(
                f"local Pyannote {field} must name exactly one pinned artifact"
            )
        repo_id = value["repo_id"]
        revision = value["revision"]
        filename = value["filename"]
        if (
            not isinstance(repo_id, str)
            or _HUB_REPOSITORY_ID.fullmatch(repo_id) is None
        ):
            raise ValueError(f"local Pyannote {field} has an invalid Hub repository id")
        if (
            not isinstance(revision, str)
            or _COMMIT_REVISION.fullmatch(revision) is None
        ):
            raise ValueError(
                f"local Pyannote {field} must use a 40-hex commit revision"
            )
        if not isinstance(filename, str):
            raise ValueError(f"local Pyannote {field} filename must be a string")
        relative = PurePosixPath(filename)
        if relative.is_absolute() or not relative.parts or ".." in relative.parts:
            raise ValueError(
                f"local Pyannote {field} filename must stay within its repository"
            )
        return cls(repo_id=repo_id, revision=revision, filename=filename)


@dataclass(frozen=True)
class LocalPyannoteModelGraph:
    """Complete immutable dependency graph for the released local backend."""

    pipeline: PinnedHuggingFaceArtifact
    segmentation: PinnedHuggingFaceArtifact
    embedding: PinnedHuggingFaceArtifact


@lru_cache(maxsize=1)
def load_local_pyannote_model_graph() -> LocalPyannoteModelGraph:
    """Validate the packaged graph before any model download is authorized."""

    raw = json.loads(
        resources.files("batchalign.inference")
        .joinpath(_MODEL_MANIFEST_RESOURCE)
        .read_text(encoding="utf-8")
    )
    if not isinstance(raw, dict) or set(raw) != {
        "schema_version",
        "pipeline",
        "segmentation",
        "embedding",
    }:
        raise ValueError("local Pyannote manifest has an unsupported shape")
    if raw["schema_version"] != 1:
        raise ValueError("local Pyannote manifest has an unsupported schema version")
    return LocalPyannoteModelGraph(
        pipeline=PinnedHuggingFaceArtifact.parse(raw["pipeline"], field="pipeline"),
        segmentation=PinnedHuggingFaceArtifact.parse(
            raw["segmentation"], field="segmentation"
        ),
        embedding=PinnedHuggingFaceArtifact.parse(raw["embedding"], field="embedding"),
    )


def _load_pipeline(config: dict[str, Any]) -> object | None:
    """Keep the heavyweight Pyannote import behind the lazy model boundary."""

    try:
        from pyannote.audio import Pipeline as PyannotePipeline
    except ImportError as exc:
        raise ImportError(
            "Speaker diarization requires pyannote.audio, which is not installed.\n"
            "Reinstall the standard batchalign3 package and confirm "
            "'import pyannote.audio' works in the worker Python runtime."
        ) from exc

    return PyannotePipeline.from_pretrained(config)


def _emit_model_download_if_missing(artifact: PinnedHuggingFaceArtifact) -> None:
    """Import progress reporting lazily to avoid the worker protocol cycle."""

    from batchalign.worker._progress import emit_hf_download_if_missing

    emit_hf_download_if_missing(
        artifact.repo_id,
        kind="speaker diarization",
        artifacts=(artifact.filename,),
        revision=artifact.revision,
    )


def _pinned_pipeline_config(graph: LocalPyannoteModelGraph) -> dict[str, Any]:
    """Materialize a config whose transitive model references cannot move."""

    pipeline_config_path = hf_hub_download(
        repo_id=graph.pipeline.repo_id,
        filename=graph.pipeline.filename,
        revision=graph.pipeline.revision,
    )
    embedding_path = hf_hub_download(
        repo_id=graph.embedding.repo_id,
        filename=graph.embedding.filename,
        revision=graph.embedding.revision,
    )
    loaded = OmegaConf.to_container(
        OmegaConf.load(Path(pipeline_config_path)), resolve=True
    )
    if not isinstance(loaded, dict):
        raise ValueError("pinned local Pyannote config is not a mapping")
    pipeline = loaded.get("pipeline")
    if not isinstance(pipeline, dict):
        raise ValueError("pinned local Pyannote config has no pipeline mapping")
    params = pipeline.get("params")
    if not isinstance(params, dict):
        raise ValueError("pinned local Pyannote config has no parameter mapping")

    params["segmentation"] = {
        "checkpoint": graph.segmentation.repo_id,
        "revision": graph.segmentation.revision,
    }
    params["embedding"] = embedding_path
    return loaded


def _get_pyannote_pipeline() -> object:
    """Return one lazily loaded, immutable model graph per worker process."""

    global _PYANNOTE_PIPELINE

    if _PYANNOTE_PIPELINE is None:
        graph = load_local_pyannote_model_graph()
        for artifact in (graph.pipeline, graph.segmentation, graph.embedding):
            _emit_model_download_if_missing(artifact)
        pipeline = _load_pipeline(_pinned_pipeline_config(graph))
        if pipeline is None:
            raise RuntimeError("pinned local Pyannote model graph could not be loaded")
        _PYANNOTE_PIPELINE = pipeline
    return _PYANNOTE_PIPELINE
