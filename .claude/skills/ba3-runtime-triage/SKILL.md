---
name: ba3-runtime-triage
description: Diagnose a hung/stuck batchalign3 worker, server, or job ("0% CPU", "job stuck in Queued", "server won't respond"). Use BEFORE restarting anything.
allowed-tools: Bash, Read, Grep
---

# Runtime Triage

Diagnose first; a blind restart destroys the evidence and can
clobber healthy runs.

- Profiling a live worker: py-spy recipes in
  `book/src/batchalign/developer/tracing-and-debugging.md` (needs
  `--native`/`--subprocesses`/sudo specifics from that page).
- Async runtime: tokio-console via the `debug-runtime` feature build,
  `book/src/batchalign/developer/cpu-profiling.md`.
- "Queued forever with no runner" and every restart/requeue failure
  mode: the canonical invariants in
  `book/src/batchalign/architecture/job-state-machine.md`
  (exclusive-runner, RunGeneration, bootstrap reconciliation;
  contract test `tests/restart_handoff.rs`).
- Memory/admission stalls: gates and tier floors in
  `book/src/batchalign/developer/memory-safety.md`; a worker denied
  by the memory gate requeues, it is not hung.
- If a restart IS warranted: restarts bump RunGeneration; stale
  runners cannot clobber the restarted job; verify the daemon came
  back on the intended binary afterward.
