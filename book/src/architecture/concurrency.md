# Concurrency and Async

**Status:** Current
**Last updated:** 2026-03-16

This chapter documents the concurrency and async patterns used across the TalkBank
Rust crates and the batchalign server.

## Threading Model Overview

```mermaid
flowchart TD
    subgraph talkbank-tools
        cli["chatter CLI"]
        workers["std::thread workers\n(crossbeam channels)"]
        cache["CachePool\n(embedded tokio rt)"]
        lsp["talkbank-lsp\n(tokio multi-thread)"]

        cli --> workers
        workers --> cache
    end

    subgraph batchalign3
        server["axum HTTP server\n(tokio multi-thread)"]
        pool["WorkerPool\n(Semaphore + RAII checkout)"]
        python["Python worker processes\n(stdio JSON-lines IPC)"]
        ws["WebSocket\n(broadcast channel)"]

        server --> pool --> python
        server --> ws
    end
```

Two distinct concurrency worlds:
- **talkbank-tools:** CPU-bound validation with `std::thread` + crossbeam channels
- **batchalign3:** I/O-bound server with tokio async runtime

## Validation Parallelism (talkbank-tools)

**Location:** `talkbank-cli/src/commands/validate_parallel/{runtime.rs,audit.rs}`

```mermaid
flowchart LR
    disc["File discovery\n(walkdir)"]
    queue["crossbeam bounded channel\n(capacity = num_files)"]
    w1["Worker thread 1"]
    w2["Worker thread 2"]
    wn["Worker thread N"]
    events["crossbeam unbounded channel\n(ValidationEvent stream)"]
    ui["UI thread\n(progress bars / TUI)"]

    disc --> queue
    queue --> w1 & w2 & wn
    w1 & w2 & wn --> events --> ui
```

- **N worker threads** via `std::thread::spawn()` (default: `num_cpus::get()`)
- Workers pull paths from bounded channel, parse + validate, send events
- UI thread receives events for progress display
- **No Rayon** — crossbeam channels give fine-grained cancellation control

Audit mode follows the same worker model, but its output path is intentionally
separate from the renderer/event loop used by standard validation. Workers send
completed file results to a dedicated audit writer thread via a bounded channel.
That writer thread owns both JSONL file IO and summary statistics, so workers do
not contend on a shared `Mutex<BufWriter<File>>`.

### Cancellation

```
First Ctrl+C  → send to cancel channel → workers check every 1-10 files
Second Ctrl+C → std::process::exit(130) (immediate, no cleanup)
```

Workers poll `cancel_rx.try_recv()` between files. Atomic counter tracks
Ctrl+C presses.

### Cache Bridge

**Problem:** Validation workers are `std::thread` but SQLite uses async sqlx.

**Solution:** `CachePool` embeds a single-threaded tokio runtime:

```rust
pub struct CachePool {
    pool: SqlitePool,              // Async pool
    rt: tokio::runtime::Runtime,   // Single-threaded, embedded
}

impl CachePool {
    fn get(&self, path: &str) -> Option<CacheEntry> {
        self.rt.block_on(async { /* sqlx query */ })
    }
}
```

Each sync worker calls `rt.block_on()`. The embedded runtime is lightweight
(single-threaded, no background tasks).

## Server Concurrency (batchalign3)

### Tokio Runtime

| Component | Runtime | Why |
|-----------|---------|-----|
| `batchalign-app` server | Multi-thread (default) | Concurrent HTTP + WebSocket + job tasks |
| `talkbank-lsp` | Multi-thread | Concurrent LSP requests from editor |
| Validation `CachePool` | `current_thread` (embedded) | Bridge for sync workers; minimal overhead |

### Worker Pool

**Location:** `batchalign-app/src/worker/pool/`

```mermaid
flowchart TD
    req["Job dispatch request"]
    sem["Semaphore\n(permits = idle workers)"]
    queue["Mutex&lt;VecDeque&gt;\n(idle queue)"]
    guard["CheckedOutWorker\n(RAII guard)"]
    work["Process files\n(10-300 seconds)"]
    ret["Drop guard\n→ return to queue\n→ release permit"]

    req -->|acquire permit| sem
    sem -->|pop worker| queue
    queue --> guard --> work --> ret
    ret -->|push worker| queue
    ret -->|add permit| sem
```

**Key design:** No lock is held during the 10–300 second dispatch. The semaphore
signals worker availability; the mutex protects only the fast push/pop (< 10 μs).

**Checkout flow:**
1. Try non-blocking semaphore acquire
2. If none available → try spawning new worker (CAS on `AtomicUsize` total)
3. If at capacity → async wait for permit
4. Pop from idle queue, wrap in `CheckedOutWorker`

**Return flow (Drop impl):**
- Push worker back to idle queue
- Add semaphore permit (wakes next waiter)
- If worker died (`take()`), decrement total instead

### Channels

| Channel | Type | Purpose | Capacity |
|---------|------|---------|----------|
| Job events → WebSocket clients | `broadcast` | Fan-out to all connected browsers | 4096 |
| Error streaming | `mpsc` | Async error sink for validation | Unbounded |
| Queue work signal | `Notify` | Wake dispatcher when work arrives | N/A |

**Broadcast pattern:** Lagged clients receive `RecvError::Lagged(n)` and skip
messages — no backpressure on the server. Clients that disconnect break the
receive loop cleanly.

### CancellationToken

**Location:** `tokio_util::sync::CancellationToken`

Used at three levels:
- **Per-job** — checked before/after semaphore acquire and between files
- **Worker pool** — cancels background health check tasks
- **Server** — wired to SIGINT/SIGTERM for graceful shutdown

```rust
// Job dispatch checks cancellation at multiple points
if job.cancel_token.is_cancelled() {
    return Ok(());
}
```

### `select!` Usage

| Location | Branches | Purpose |
|----------|----------|---------|
| Server shutdown | `ctrl_c` / `SIGTERM` | Whichever signal fires first triggers shutdown |
| WebSocket handler | `broadcast.recv()` / `socket.recv()` | Forward events OR handle client messages |
| Health check loop | `cancel.cancelled()` / `interval.tick()` | Stop OR check worker health |
| Queue dispatcher | `notify.notified()` / `sleep` | Work arrived OR timeout |

## IPC: stdio JSON-lines (batchalign3)

**Protocol between Rust server and Python workers:**

```mermaid
sequenceDiagram
    participant Server as Rust Server
    participant Worker as Python Worker

    Server->>Worker: spawn: python -m batchalign.worker --command morphotag --lang eng
    Worker->>Server: {"ready": true, "pid": 1234, "transport": "stdio"}

    loop Request/Response
        Server->>Worker: {"op": "batch_infer", "task": "morphosyntax", ...}\n
        Worker->>Server: {"result": [...], "status": "ok"}\n
    end

    Server->>Worker: {"op": "shutdown"}\n
    Worker->>Server: {"status": "ok"}\n
```

- One JSON object per line (newline-delimited)
- No framing bytes — newline = message boundary
- Worker startup: loads models, prints `ready` message
- Operations: `process`, `infer`, `batch_infer`, `health`, `capabilities`, `shutdown`

## Database Concurrency

**SQLite WAL configuration:**

| Setting | Value | Why |
|---------|-------|-----|
| Journal mode | WAL | Readers don't block writers |
| Synchronous | Normal | Balanced durability vs speed |
| Busy timeout | 5000 ms | Auto-retry on SQLITE_BUSY |
| Max connections | 16 | Matches worker thread count |
| mmap size | 256 MB | Fast random access for 95k+ entries |

## Lock Ordering

No formal documented hierarchy, but the observed invariants are:

1. **Worker pool:** Semaphore acquired first (async) → then idle queue mutex
   (sync, < 10 μs). Never reversed.
2. **Job store:** HashMap lock acquired for short reads/writes only. Never nested
   with worker pool locks.
3. **DashMap** (string interner, media cache): Single-level, no nested locks.
4. All `tokio::sync::Mutex` guards are dropped before `.await` points.

**Deadlock prevention:** No cyclic lock dependencies. Longest lock hold is the
job semaphore (intentional backpressure, not a mutex).

## Signal Handling

### Server (batchalign3)

```rust
tokio::select! {
    () = signal::ctrl_c() => info!("SIGINT, shutting down"),
    () = sigterm_future   => info!("SIGTERM, shutting down"),
}
```

### CLI (talkbank-tools)

Double Ctrl+C pattern:
- First press: graceful cancel via crossbeam channel
- Second press: `std::process::exit(130)` (immediate)

## Desktop App Concurrency (Tauri)

**Location:** `desktop/src-tauri/src/commands.rs`

The Chatter desktop app uses `ArcSwapOption` (from `arc-swap`) for lock-free
storage of the cancel sender:

```rust
pub struct ValidationState {
    cancel_tx: ArcSwapOption<Sender<()>>,
}
```

- **`validate()`** atomically stores the cancel sender via `.store(Some(...))`
- **`cancel_validation()`** atomically takes it via `.swap(None)`
- Zero contention: no mutex, no lock, no blocking

Event forwarding uses the same crossbeam channel pattern as the TUI:
`validate_directory_streaming()` returns a `Receiver<ValidationEvent>`, and
a dedicated thread forwards events to the Tauri frontend via `app.emit()`.

## Mutex Policy

**Avoid Mutex wherever possible.** Use lock-free alternatives:

| Need | Use | Not |
|------|-----|-----|
| Atomic swap of optional value | `ArcSwapOption` | `Mutex<Option<T>>` |
| Concurrent map | `DashMap` | `Mutex<HashMap>` |
| Lock-free counter | `AtomicUsize` / `AtomicBool` | `Mutex<usize>` |
| Lazy initialization | `OnceLock` / `LazyLock` | `Mutex<Option<T>>` |
| Work distribution | `crossbeam_channel` | `Mutex<VecDeque>` |
| Async event fan-out | `tokio::sync::broadcast` | shared vec behind mutex |
| Async availability gate | `tokio::sync::Semaphore` | mutex-guarded counter |
| One-shot signal | `crossbeam_channel` / `tokio::sync::oneshot` | mutex-guarded bool |

**When Mutex is acceptable:** Sub-microsecond critical sections (push/pop on
a VecDeque, single HashMap lookup) that never cross an `.await` point. Document
the justification in a code comment.

**Current Mutex inventory (exhaustive):**

| Location | Type | Field | Justification |
|----------|------|-------|---------------|
| `talkbank-model` `ErrorCollector` | `parking_lot::Mutex` | `errors: Mutex<Option<Vec<ParseError>>>` | Parser error collection; held for `push()` only (~1 μs) |
| `batchalign-app` `WorkerGroup` | `std::sync::Mutex` | `idle: VecDeque<WorkerHandle>` | push/pop (~1 μs), never across `.await` |
| `batchalign-app` `WorkerPool` | `std::sync::Mutex` | `groups: HashMap<WorkerKey, Arc<WorkerGroup>>` | lookup (~1 μs), never across `.await` |
| `batchalign-app` `WorkerGroup` | `tokio::sync::Mutex` | `bootstrap: Mutex<()>` | Serializes worker spawning (held across `.await`; `tokio::sync` required) |
| `batchalign-app` `JobRegistry` | `tokio::sync::Mutex` | `jobs: HashMap<JobId, Job>` | State transitions held across `.await` |

No `RwLock` usage anywhere. No `std::sync::Mutex` held across `.await` points.

## Design Rationale Summary

| Decision | Why |
|----------|-----|
| crossbeam channels, not Rayon | Fine-grained cancellation polling; simpler progress UI integration |
| Embedded single-threaded tokio in CachePool | Sync workers can't be tokio tasks; lightweight bridge |
| RAII CheckedOutWorker + Semaphore | Eliminates 10–300 s mutex holds; permits track availability, not workers |
| broadcast for WebSocket | Fan-out; lagged clients skip gracefully |
| DashMap for string interning | Shard-level locks; concurrent safe interning without global contention |
| WAL + 5 s busy timeout | Multiple concurrent readers + writers; auto-retry prevents failures |
| std::sync::Mutex for worker queue | Held < 10 μs; no need for async-aware lock |
| ArcSwapOption for desktop cancel sender | Lock-free atomic swap; no contention |
