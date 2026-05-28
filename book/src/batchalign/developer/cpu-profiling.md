# CPU Profiling

**Status:** Current
**Last updated:** 2026-05-27 22:02 EDT

How to profile CPU usage across batchalign's two languages — Python
(worker process: ML inference, audio decoding, transcript
postprocessing) and Rust (dispatch, FA orchestration, cache,
server). Pick the tool that matches the side and shape of the
question; both can run on the same host without conflict.

## Python — `py-spy`

`py-spy` is a Rust-implemented sampling profiler that attaches to a
running Python process by PID. It reads interpreter state directly
across process boundaries, requires no code changes, and adds < 5%
CPU overhead on the target.

**Install** (macOS): `brew install py-spy`. Cross-platform via
`uv pip install py-spy` or `cargo install py-spy`.

**`py-spy` requires `sudo` on macOS** to read another process's
memory, unless the binary is code-signed for ptrace. For local-
process diagnostics during development, `sudo` is fine.

### The three commands you'll actually use

```bash
# 1. ONE-SHOT STACK DUMP — what's every thread doing right now?
#    First thing to try for a hung worker. Replaces "tail the log
#    and guess." Returns Python frames + thread names instantly.
sudo py-spy dump --pid <worker-pid>

# 2. LIVE TOP — per-function CPU% updated continuously, like `top`
#    but for Python frames. Useful when CPU is high but you don't
#    know which path is hot.
sudo py-spy top --pid <worker-pid>

# 3. FLAME GRAPH — record a sampling session and write SVG. The
#    canonical answer to "where is time being spent over a sustained
#    workload." `--native` includes C/C++ frames (PyTorch, Whisper,
#    Stanza native ops). `--subprocesses` follows forked children
#    (matters for our parallel morphotag dispatch).
sudo py-spy record -o profile.svg \
    --pid $(pgrep -f batchalign3) \
    --native --subprocesses
# Open profile.svg in any browser; click frames to zoom.
```

### When to use which

| Symptom | Command |
|---|---|
| Worker hung at 0% CPU; daemon log is silent | `py-spy dump` — get the Python stack instantly |
| Worker is using 100% CPU but you don't know why | `py-spy top` — watch per-function CPU live |
| Sustained slow performance; want to optimize | `py-spy record --native` — flame graph the workload |
| Multi-worker job; want to see which child is busy | `py-spy record --subprocesses` — follows forks |

### Speedscope alternative

For interactive flame-graph exploration in the browser, use
`--format speedscope` and open the JSON file at <https://speedscope.app>.
Doesn't replace SVG output; complements it for deep dives.

## Rust — `samply` / `flamegraph-rs`

For the Rust side (dispatch chain, server, CLI), use a Rust-native
sampling profiler. Both tools share the perf / dtrace backends and
produce roughly equivalent output.

### `cargo flamegraph` (flamegraph-rs)

```bash
# Build release binary then profile it under a workload
cargo install flamegraph
cargo flamegraph --release --bin batchalign3 -- \
    transcribe input/ -o out/ --lang eng
# Produces flamegraph.svg in the current directory.
```

### `samply` (interactive)

`samply` is a samply-rs profiler that opens the trace in the
Firefox profiler UI for interactive exploration. Better for digging
into specific call paths.

```bash
cargo install samply
samply record ./target/release/batchalign3 transcribe input/ -o out/
# Opens Firefox profiler with the trace.
```

Both tools require elevated permissions on macOS (`sudo`) the same
way `py-spy` does, for the same reason.

## When BOTH halves are slow / stuck

The batchalign dispatch chain crosses a subprocess boundary — Rust
parent owns the orchestration, Python workers own ML inference. A
"slow transcribe" or "hung pipeline" can be either side. The right
move is **profile both halves concurrently**:

```bash
# Terminal 1: launch the workload
batchalign3 transcribe input/ -o out/ --lang yue --engine-overrides '{...}'

# Terminal 2: find the worker pid and dump Python stack
sudo py-spy dump --pid $(pgrep -f batchalign.worker)

# Terminal 3: profile the Rust parent
WORKER_PARENT=$(pgrep -f "batchalign3 transcribe")
samply record --pid $WORKER_PARENT  # samply attaches by pid
```

If the Rust side shows tasks parked on `oneshot::Receiver` waiting
for a worker response, switch from `samply` to `tokio-console` for
the async-task view — see
[Tracing and Debugging](./tracing-and-debugging.md) for the
tokio-console setup and workflow.

## What this doc does not cover

- **Memory profiling.** Use `memray` (Python) or `dhat-rs` (Rust)
  — separate "How to profile memory" doc when those land.
- **Allocation hotpath in PyTorch / Whisper / Stanza native code.**
  Use `py-spy record --native` to get frames into C/C++ ops; full
  symbol resolution requires the native libraries' debug info.
- **Distributed tracing across processes.** That's OpenTelemetry
  via the existing `BATCHALIGN_OTLP_ENABLE` env-var hook in
  `batchalign3`; see [Tracing and Debugging](./tracing-and-debugging.md).
