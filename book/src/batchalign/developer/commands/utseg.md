# utseg: Developer Reference

**Status:** Current
**Last updated:** 2026-08-30 19:35 EDT

Implementation guide for the `utseg` command. For user-facing documentation,
see [User Guide: utseg](../../user-guide/commands/utseg.md).

---

## Implementation map

| Layer | Location | Responsibility |
|-------|----------|----------------|
| CLI args | `crates/batchalign/src/cli/args/commands.rs`: `UtsegArgs` | lang, num-speakers |
| Catalog entry | `crates/batchalign/src/recipe_runner/catalog.rs` | the `CatalogEntry` for `utseg` |
| Stage recipe | `crates/batchalign/src/recipe_runner/recipes.rs` | `UTSEG_RECIPE` |
| Utseg orchestration | `crates/batchalign/src/utseg.rs` | Per-file and cross-file workflow adapters, typed worker-result admission |
| Worker IPC | `batchalign/inference/utseg.py` | Returns direct model assignments with evidence, or Stanza trees |
| Python model evidence | `batchalign/models/utterance/evidence.py` | Closed actions, fixed-point probability, omission/bypass states |
| Canonical IPC evidence | `crates/batchalign-types/src/worker_v2/utseg_evidence.rs` | Rust wire enums and validated probability newtype |
| Evidence artifacts | `crates/batchalign/src/utseg_evidence.rs` | Versioned pre/post-CHAT transcribe traces and atomic sink |
| Boundary application | `crates/batchalign-transform/src/utseg.rs` | Maps admitted assignments back to typed CHAT structure |

Local submissions (auto-daemon or loopback `--server`) use `paths_mode=true`
as of 2026-04-14: the CLI posts source/output path lists instead of CHAT
bytes. See [Submission Modes](../../reference/command-io.md#submission-modes-paths_modetrue-vs-paths_modefalse).

---

## Caching behavior

Text NLP tasks (`utseg`, `translate`, `morphotag`) do not use the utterance cache.
Boundaries are computed from word sequence, language, exact model revision, and
current postprocessing during each inference run; no per-utterance result cache
exists. Server mode still avoids repeated model-loading startup cost by keeping
the worker warm. Transcribe debug evidence can be replayed for policy research
without rerunning the boundary model, but it is not a production result cache.

---

## Worker IPC: utseg task

Rust freezes the text batch in a prepared artifact, then sends
`execute_v2(task="utseg")` with the language and explicit Stanza-fallback
authorization. Each result item has exactly one success representation:

- `assignments`: one group ID per request word. A boundary-model result also
  carries `boundary_model_evidence`, including model identity and one typed
  evidence state per word.
- `trees`: raw constituency trees from the explicitly authorized Stanza
  fallback. Rust computes assignments from the trees.
- `error`: a per-item failure with no success payload.

`AdmittedUtsegPrediction` rejects error/success mixtures, assignments plus
trees, evidence without assignments, empty model identity, and any assignment
or evidence length that differs from the request words. Its variants preserve
boundary-model, unobserved direct-assignment, and constituency sources. The
legacy `UtsegResponse` can be obtained only through an explicit projection
that erases that source state.

---

## Stanza constituency availability

About 11 languages have Stanza constituency models. A language without a
configured TalkBank boundary model is refused by default; the operator must
pass `--utseg-fallback-stanza`. The available processors are queried at worker
startup via `batchalign/worker/_stanza_capabilities.py`, never hardcoded.

## Evidence retention scope

The transcribe pipeline can retain exact boundary evidence with
`--debug-dir`: pre-CHAT and post-CHAT phases get separate files. The standalone
`utseg` command currently admits the same typed worker result but deliberately
projects it to assignments without writing those transcribe sidecars. Do not
claim that a standalone run retained evidence unless a future command-specific
artifact surface explicitly does so.

## Replaying adjacency policies

`scripts/probe_utterance_boundary_policy.py` compares the current decoder
policy with a boundary-only alternative over retained
`*_asr_response.json` artifacts. It makes no ASR request. The local model is
loaded once, raw evidence is captured once per source monologue, and both
policies are applied to that identical evidence.

```bash
uv run python scripts/probe_utterance_boundary_policy.py \
  <retained-asr-directory> \
  <output-report.json>
```

The probe validates the complete retained input schema, records SHA-256 for
every input, refuses model-identity or evidence-length drift, and atomically
publishes a versioned report. Each assignment-changing case includes lexical
context, fixed-point boundary probability, and a typed known-or-missing
interword timing. The report is experimental evidence, not a production-policy
switch or an accuracy verdict. Candidate promotion requires a human-linked,
controlled comparison.

---

## Pre-validation gate

`utseg` requires CHAT Level 1 (parseable + valid headers). Gate in
`crates/batchalign/src/utseg.rs`. Implemented via
`validate_to_level(chat, ValidationLevel::StructurallyComplete)`.

---

## Testing

```bash
make test
cargo test -p batchalign utseg::
cargo test -p batchalign utseg_evidence::
uv run pytest -q batchalign/tests/models/test_bert_utterance_sliding_window.py
uv run pytest -q batchalign/tests/models/test_utterance_boundary_policy.py
uv run pytest -q batchalign/tests/models/test_utterance_policy_probe.py
uv run pytest -q batchalign/tests/pipelines/utterance/test_utseg_inference.py
# ML golden tests, only on Fleet/Large-tier hosts
cargo test -p batchalign --features ml-golden --test ml_golden utseg::golden
```

---

## Related developer documentation

- [Command Flowcharts: utseg](../../architecture/command-flowcharts.md#utseg)
- [Utterance Segmentation](../../reference/utterance-segmentation.md)
- [Stanza Capability Registry](../../architecture/stanza-capability-registry.md)
