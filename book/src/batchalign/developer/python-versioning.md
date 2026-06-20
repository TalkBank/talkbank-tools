# Python Version Support

**Status:** Current
**Last verified:** 2026-05-01 09:47 EDT

## Current policy

Python 3.12 is the current contributor and deployment baseline.

Python 3.14t (free-threaded / no-GIL) is **paused**. Do not treat it as a
supported install path, do not deploy it to fleet machines, and do not weaken
the main package contract just to make 3.14t possible.

**The current gating constraint is wheel coverage, not runtime stability.**

The next realistic revisit point is **the first Python 3.15.x release that
ships `torch` and `onnxruntime` wheels for cp315/cp315t on macOS arm64**.
The current 3.15 alpha does not, torch is the gating wheel for either
ABI on the new interpreter line.

## Why 3.14t is paused again

### 1. The default install must stay complete

`batchalign3 transcribe --diarize` is a real supported CLI path. After the BA2
parity audit, that path once again means "run the dedicated post-ASR speaker
stage" even on top of Rev-labeled output.

Because of that, `pyannote.audio` and `onnxruntime` belong in the **standard**
`batchalign3` install. We are not keeping speaker diarization in a special
optional tier just to unblock a narrow 3.14t experiment.

### 2. 3.14t has a bad operational history that current soak evidence does not fully resolve

Independent 75-minute soaks on CPython 3.14.4t (VM + bare metal) have
completed cleanly, no swap activation, no monotonic VM growth, RSS
bounded under 2 GB, per-thread sentence counts within sub-1% across
workers (strong free-threading balance). A previous kernel-panic
precursor pattern has not reproduced in those configurations.

That's strong evidence, though still not formal proof. The soaks did
NOT exercise:

- the multi-day idle that may have been load-bearing in the prior
  production panic (soaks compressed idle to ~15 min)
- the diarization stack (no cp314t wheels exist, so we couldn't)
- interpreter-shutdown stress, exception-from-thread pathways, or
  signal handling

The production fleet still needs boring, predictable behavior more than
it needs an experimental concurrency win, and we already have a reliable
3.12 deployment path. The reason 3.14t isn't deployable today is
"diarization wheel coverage" (criterion 2, below) rather than panic risk.

### 3. The wheel ecosystem is still incomplete (this is now the dominant blocker)

When speaker diarization stays in the base package, `onnxruntime` comes back
into the required dependency set. That means the old Apple/macOS `cp314t`
wheel-coverage problem is a hard blocker for a full standard 3.14t install.

The current wheels probe shows the coverage state on macOS arm64:

| Interpreter | core (numpy/torch/stanza) | diarization (onnxruntime/pyannote) |
|---|---|---|
| 3.12.11 | ✓ | ✓ (reference green) |
| 3.14.3t | ✓ | **install_fail** (cp314t hole) |
| 3.15.0a8 (non-FT) | numpy ✓, **torch ✗**, stanza ✓ | ✗ |
| 3.15.0a8t | numpy built from source, **torch ✗**, stanza ✓ | ✗ |

There is currently no Python ≥ 3.13 on macOS arm64 with both required wheel
groups present. 3.14t is closest, it has core but lacks diarization. 3.15
is further from usable, not closer.

## Historical finding worth preserving

We should still remember **why** 3.14t was attractive.

The real benefit was never small Python startup wins. It was **shared Stanza
model memory** for `morphotag` and `utseg`.

### Earlier measurements

These measurements came from an earlier pipeline architecture, but the core
observation remains important for future revisits:

| Host | Scenario | Peak RSS | Files/hour |
| --- | --- | ---: | ---: |
| worker-machine | GIL=1, 4 workers | 13.5 GB | 10,069 |
| worker-machine | GIL=0, 4 threads | 3.0 GB | 10,158 |

That is roughly a **77% memory reduction** with essentially unchanged
throughput. If the ecosystem matures, this is the reason to reconsider
free-threaded Python later.

## What remains in the codebase

Some free-threaded groundwork remains in tree and is harmless to keep:

- runtime detection of free-threaded interpreters
- distinct memory-budget tables for process vs. threaded serving
- thread-safe tokenizer realignment state
- harness-side cleanup of `PYTHON_GIL` inheritance

Those are acceptable to keep as future-facing infrastructure. They should **not**
be taken as permission to target 3.14t in packaging, CI, or fleet policy.

## Packaging policy

- `pyannote.audio` and `onnxruntime` are part of the standard package again
- a missing speaker runtime is a broken install, not an alternate supported mode
- the public install contract remains: one normal install provides the supported
  command surface, including dedicated speaker diarization

## Revisit criteria

Revisit no-GIL Python only when **all** of the following are true:

1. Python 3.15 or newer has materially better ecosystem support for our stack.
2. Required diarization/runtime dependencies have compatible wheels on the
   platforms we actually use.
3. We can demonstrate stable end-to-end ML workloads on isolated hosts without
   crashes or pathological memory behavior.
4. CI and release automation can exercise that runtime intentionally instead of
   relying on ad hoc contributor machines.

### Current empirical status

| # | Criterion | Status | Evidence |
|---|-----------|--------|----------|
| 1 | 3.15+ better ecosystem | **Worse**, not better | 3.15 alpha wheels probe, torch absent for cp315 |
| 2 | Diarization wheels available | **Not met** | onnxruntime + pyannote.audio fail install on cp314t and on cp315 (both ABIs) |
| 3 | Stable ML workloads | **Strong; multiple positive data points across VM and bare metal** | 75-min 3.14.4t soaks across VM and bare metal all clean, no swap activation, RSS bounded, sub-1% thread spread; a prior production panic remains a counter-example but cannot be reproduced in current configurations |
| 4 | CI exercises it | **Not met** | Probe + JSONL trail exist, GH Actions matrix not wired |

Until criterion 2 clears:

- use Python 3.12 for development
- use Python 3.12 for deployment
- treat 3.14t as shelved research, not an active engineering target

Once criterion 2 clears (some future Python+OS combination ships diarization
wheels), criteria 3 and 4 become the gating questions, and the soak
infrastructure under
[`freethreaded-danger-probe`](https://github.com/TalkBank/freethreaded-danger-probe)
exists to answer them.
