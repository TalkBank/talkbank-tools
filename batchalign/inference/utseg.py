"""Stanza constituency inference: words -> utterance boundary assignments.

Pure inference — no CHAT, no caching, no pipeline.
"""

from __future__ import annotations

import logging
import os
import time
from collections.abc import Callable
from typing import TYPE_CHECKING

from pydantic import BaseModel, ValidationError

from batchalign.inference._domain_types import LanguageCode

if TYPE_CHECKING:
    from batchalign.inference.types import ConstituencyTree, StanzaNLP
    from batchalign.models.utterance import BertUtteranceModel

from batchalign.providers import (
    BatchInferRequest,
    BatchInferResponse,
    InferResponse,
)

L = logging.getLogger("batchalign.worker")


class UtsegModelNotFoundError(RuntimeError):
    """Raised when utseg has no language-specific BERT model for the
    requested language and the operator has not opted in to the
    Stanza constituency-parser fallback.

    The default refuses substitution. Operators who want the previous
    Stanza-fallback behavior must set the
    ``BA3_UTSEG_FALLBACK_STANZA=1`` environment variable on the worker
    process. This mirrors the ``WhisperHubModelNotFoundError`` pattern
    in ``batchalign/inference/whisper_hub.py`` — surface the gap rather
    than silently substitute one model for another.
    """


# Worker-level opt-in. When this env var is set to a truthy value
# (``"1"``, ``"true"``, ``"yes"`` — case-insensitive) the dispatcher
# retains the legacy Stanza fallback for languages without a BERT
# entry. Otherwise it raises ``UtsegModelNotFoundError``.
_FALLBACK_OPT_IN_ENV = "BA3_UTSEG_FALLBACK_STANZA"
_FALLBACK_OPT_IN_TRUTHY: frozenset[str] = frozenset({"1", "true", "yes"})


def _stanza_fallback_opted_in() -> bool:
    raw = os.environ.get(_FALLBACK_OPT_IN_ENV, "")
    return raw.strip().lower() in _FALLBACK_OPT_IN_TRUTHY


# Stage identifier for the opt-in fallback notice. Stage names form a
# closed protocol vocabulary consumed by the dashboard / CLI for
# filtering and dedupe; the language goes in user_message, not the stage.
_STAGE_UTSEG_FALLBACK_OPT_IN = "utseg_unsupported_language_fallback"

# Per-process dedupe: warn once per (requested_lang, pack) pair. Worker
# processes don't outlive a deploy, so the set never needs eviction.
_FALLBACK_NOTICE_FIRED: set[tuple[str, str | None]] = set()


def _emit_stanza_fallback_notice(
    requested_lang: str,
    pack: str | None,
) -> None:
    """Surface the BERT-absent → Stanza substitution to the user.

    Only fires when the operator has opted in via
    ``BA3_UTSEG_FALLBACK_STANZA``. The default-refuse path raises
    ``UtsegModelNotFoundError`` instead and never reaches this helper.
    """
    # Avoid a circular import at module load time — the progress
    # protocol pulls in worker config that imports this module
    # transitively in some test setups.
    from batchalign.worker._progress import emit_download_event

    key = (requested_lang, pack)
    if key in _FALLBACK_NOTICE_FIRED:
        return
    _FALLBACK_NOTICE_FIRED.add(key)

    requested_display = requested_lang or "<unspecified>"
    pack_display = pack if pack is not None else "<none>"

    user_message = (
        f"No TalkBank utseg model for language '{requested_display}'; "
        f"using Stanza constituency parsing ({pack_display} pack) "
        f"because {_FALLBACK_OPT_IN_ENV} is set. Quality will vary."
    )

    L.warning(
        "utseg opt-in fallback: lang=%r → Stanza pack %r",
        requested_display,
        pack_display,
    )

    emit_download_event(
        stage=_STAGE_UTSEG_FALLBACK_OPT_IN,
        user_message=user_message,
    )


class UtsegBatchItem(BaseModel):
    """A single item in the batch utseg payload from Rust.

    Matches Rust ``UtsegBatchItem`` in ``batchalign/src/utseg.rs``.
    """

    words: list[str]
    text: str = ""
    lang: LanguageCode = ""


def batch_infer_utseg(
    req: BatchInferRequest,
    build_stanza_config: Callable[[list[str]], tuple[list[str], dict[str, dict[str, str | bool]]]],
    utterance_boundary_model: "BertUtteranceModel | None" = None,
) -> BatchInferResponse:
    """Batch Stanza constituency inference: (words) -> tree strings.

    Parameters
    ----------
    req : BatchInferRequest
        Batch of UtsegBatchItem payloads.
    build_stanza_config : callable
        Function ``(langs) -> (lang_alpha2, configs)`` from the utseg engine.

    Returns tree bracket notation strings. Assignment computation is done in Rust.
    """
    t0 = time.monotonic()

    n = len(req.items)
    items: list[UtsegBatchItem | None] = []
    for raw_item in req.items:
        try:
            items.append(UtsegBatchItem.model_validate(raw_item))
        except ValidationError:
            items.append(None)

    results: list[InferResponse] = [
        InferResponse(result={"trees": []}, elapsed_s=0.0) for _ in range(n)
    ]

    miss_indices: list[int] = []
    for i, item in enumerate(items):
        if item is None:
            results[i] = InferResponse(error="Invalid batch item", elapsed_s=0.0)
            continue
        if len(item.words) <= 1:
            results[i] = InferResponse(
                result={"assignments": [0] * len(item.words)},
                elapsed_s=0.0,
            )
            continue
        miss_indices.append(i)

    if not miss_indices:
        return BatchInferResponse(results=results)

    if utterance_boundary_model is not None:
        for idx in miss_indices:
            item = items[idx]
            assert item is not None
            try:
                assignments = utterance_boundary_model.predict_assignments(item.words)
                results[idx] = InferResponse(
                    result={"assignments": assignments},
                    elapsed_s=0.0,
                )
            except (IndexError, AttributeError, TypeError, ValueError) as error:
                L.warning("Utseg boundary-model infer failed for item %d: %s", idx, error)
                results[idx] = InferResponse(
                    result={"assignments": [0] * len(item.words)},
                    elapsed_s=0.0,
                )
        elapsed = time.monotonic() - t0
        if results:
            first = results[0]
            results[0] = InferResponse(
                result=first.result, error=first.error, elapsed_s=elapsed
            )
        L.info("batch_infer utseg(boundary-model): %d items, %.3fs", n, elapsed)
        return BatchInferResponse(results=results)

    if not _stanza_fallback_opted_in():
        raise UtsegModelNotFoundError(
            f"No TalkBank utseg model is configured for language "
            f"'{req.lang or '<unspecified>'}'. Either add a resolver "
            f"entry in batchalign/models/resolve.py for a published "
            f"model, or set {_FALLBACK_OPT_IN_ENV}=1 on the worker to "
            f"opt in to Stanza constituency-parser fallback (quality "
            f"will vary)."
        )

    langs: list[str] = [req.lang] if req.lang else ["eng"]
    lang_alpha2, configs = build_stanza_config(langs)

    import stanza
    from stanza import DownloadMethod

    from batchalign.worker._progress import emit_download_event

    _emit_stanza_fallback_notice(
        req.lang,
        lang_alpha2[0] if lang_alpha2 else None,
    )

    nlp: StanzaNLP
    if len(lang_alpha2) > 1:
        # Multilingual pipeline pulls one language pack per ``lang_alpha2`` plus
        # the language-id model. First-run cost is a sum across packs; emit a
        # single event so the user sees the wait, even if intermediate library
        # progress prints reach only stderr.
        emit_download_event(
            stage="downloading_stanza_utseg_multilingual",
            user_message=(
                "Downloading Stanza utterance-segmentation pipeline for "
                f"{', '.join(lang_alpha2)} (one-time, one language pack per "
                "language; future runs will use the local cache)…"
            ),
        )
        nlp = stanza.MultilingualPipeline(
            lang_configs=configs,
            lang_id_config={"langid_lang_subset": lang_alpha2},
            download_method=DownloadMethod.REUSE_RESOURCES,
        )
    elif lang_alpha2:
        emit_download_event(
            stage=f"downloading_stanza_utseg_{lang_alpha2[0]}",
            user_message=(
                f"Downloading Stanza utterance-segmentation pipeline for "
                f"{lang_alpha2[0]} (one-time, ~250–500 MB; future runs will "
                "use the local cache)…"
            ),
        )
        nlp = stanza.Pipeline(
            lang=lang_alpha2[0],
            **configs[lang_alpha2[0]],
            download_method=DownloadMethod.REUSE_RESOURCES,
        )
    else:
        for idx in miss_indices:
            item = items[idx]
            assert item is not None
            results[idx] = InferResponse(
                result={"trees": []},
                elapsed_s=0.0,
            )
        return BatchInferResponse(results=results)

    for idx in miss_indices:
        item = items[idx]
        assert item is not None
        try:
            # Run Stanza and return raw constituency tree strings.
            # Rust handles tree parsing and assignment computation.
            doc = nlp(" ".join(item.words))
            trees: list[str] = []
            for sent in doc.sentences:
                if sent.constituency is not None:
                    trees.append(str(sent.constituency))
            results[idx] = InferResponse(
                result={"trees": trees},
                elapsed_s=0.0,
            )
        except (IndexError, AttributeError, TypeError) as e:
            L.warning("Utseg infer failed for item %d: %s", idx, e)
            results[idx] = InferResponse(
                result={"trees": []},
                elapsed_s=0.0,
            )

    elapsed = time.monotonic() - t0
    if results:
        first = results[0]
        results[0] = InferResponse(
            result=first.result, error=first.error, elapsed_s=elapsed
        )

    L.info("batch_infer utseg: %d items, %.3fs", n, elapsed)
    return BatchInferResponse(results=results)


# ---------------------------------------------------------------------------
# Constituency tree helpers (moved from pipelines/utterance/_utseg_callback.py)
# ---------------------------------------------------------------------------


def _leaf_count(tree: ConstituencyTree) -> int:
    """Count the number of leaf nodes under a constituency subtree."""
    try:
        children = tree.children
    except AttributeError:
        return 0
    count = 0
    for c in children:
        if c.is_leaf():
            count += 1
        else:
            count += _leaf_count(c)
    return count


def _parse_tree_indices(subtree: ConstituencyTree, offset: int) -> list[list[int]]:
    """Recursively extract S-level phrase leaf-index ranges from a constituency tree."""
    try:
        children = subtree.children
    except AttributeError:
        return []

    result: list[list[int]] = []
    subtree_labels = [
        c.label.lower() if c.label else ""
        for c in children
    ]
    has_coordination = any(
        lbl in ("cc", "conj") for lbl in subtree_labels
    )

    child_offset = offset
    for child in children:
        if child.is_leaf():
            child_offset += 1
            continue

        n_leaves = _leaf_count(child)
        child_start = child_offset

        if has_coordination and child.label == "S":
            result.append(list(range(child_start, child_start + n_leaves)))

        result += _parse_tree_indices(child, child_start)

        child_offset = child_start + n_leaves

    return result


def compute_assignments(words: list[str], nlp: StanzaNLP) -> list[int]:
    """Run constituency parsing + tree walking to compute word->utterance assignments.

    Returns a list parallel to *words* where each element is a 0-based group ID.
    """
    from itertools import groupby

    n = len(words)
    if n <= 1:
        return [0] * n

    parse = nlp(" ".join(words)).sentences
    pt = parse[0].constituency

    phrase_ranges = _parse_tree_indices(pt, 0)
    phrase_ranges = sorted(phrase_ranges, key=len)

    unique_ranges: list[list[int]] = []
    for rng in list(reversed(phrase_ranges)) + [list(range(n))]:
        rng_set = set(rng)
        for existing in unique_ranges:
            rng_set -= set(existing)
        if rng_set and not any(rng_set.issubset(set(x)) for x in unique_ranges):
            unique_ranges.append(sorted(rng_set))
    unique_ranges = list(reversed(unique_ranges))

    unique_ranges = [r for r in unique_ranges if len(r) > 1]

    if not unique_ranges:
        return [0] * n

    word_to_phrase = [-1] * n
    for phrase_id, indices in enumerate(unique_ranges):
        for idx in indices:
            if 0 <= idx < n:
                word_to_phrase[idx] = phrase_id

    for i in range(n):
        if word_to_phrase[i] != -1:
            continue
        for j in range(i + 1, n):
            if word_to_phrase[j] != -1:
                word_to_phrase[i] = word_to_phrase[j]
                break
        else:
            for j in range(i - 1, -1, -1):
                if word_to_phrase[j] != -1:
                    word_to_phrase[i] = word_to_phrase[j]
                    break

    if any(x == -1 for x in word_to_phrase):
        return [0] * n

    groups: list[list[int]] = [
        list(word_indices)
        for _, word_indices in groupby(range(n), key=lambda i: word_to_phrase[i])
    ]

    merged: list[list[int]] = []
    pending: list[int] = []
    for grp in groups:
        if len(grp) < 3:
            pending += grp
        else:
            merged.append(pending + grp)
            pending = []
    if pending:
        if merged:
            merged[-1] += pending
        else:
            merged.append(pending)

    assignments = [0] * n
    for group_id, group_indices in enumerate(merged):
        for idx in group_indices:
            assignments[idx] = group_id

    return assignments
