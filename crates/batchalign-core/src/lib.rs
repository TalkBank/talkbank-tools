//! Batchalign's pure core: the typed model, the task runners and the traits
//! they dispatch through, with no async runtime, no I/O, no SQL and no Python.
//!
//! # Why this crate exists
//!
//! Today a task runner cannot be compiled, let alone tested, without the CLI,
//! the jobs database and the worker pool coming with it. That is the single
//! largest structural difference between this codebase and the fork studied in
//! `docs/design/2026-07-28-fork-architecture-essence-and-our-shortfall.md`: a
//! `Dispatcher` trait living in a core crate makes every runner testable
//! against a mock, with no model download, no GPU and no interpreter. The
//! crate has to be extracted before that seam can exist, which is why the split
//! comes first (step 5) and the trait second (step 6).
//!
//! # The boundary, stated as a rule
//!
//! Code belongs here when it needs none of: a tokio scheduler or timer, the
//! filesystem, a subprocess, a socket, the process environment, a database, or
//! the Python bridge. Code that orchestrates those things belongs in
//! `batchalign-engine` and calls in.
//!
//! `tokio::sync` is fine. Its channels and locks are executor-agnostic, and
//! `async fn` plus `async_trait` are language and macro surface rather than a
//! runtime, so a task runner that awaits a dispatcher compiles here. Conflating
//! those three surfaces under the phrase "no tokio" makes every runner look
//! unmovable and contradicts this crate's own definition.
//!
//! # The boundary is enforced, not documented
//!
//! `cargo run -q -p xtask -- lint-core-purity` runs in `make lint` and in CI,
//! and fails on either half of the boundary: a dependency that hands core a
//! capability wholesale (HTTP, SQL, pyo3, a second executor), or a source line
//! in `src/` that reaches for the tokio scheduler, the filesystem, a subprocess,
//! a socket or the process environment. Prose in a module doc does not survive a
//! well-intentioned dependency added eighteen months from now; a failing build
//! does.
//!
//! The source half is not belt-and-braces, it is load-bearing. Cargo unifies
//! features across a workspace, so this crate links whatever `tokio` the CLI
//! asks for regardless of what its own manifest requests, which means
//! `tokio::spawn` written here would compile. Declaring narrow features cannot
//! enforce anything; only the source check can. The same check is the only thing
//! that can see `std::fs`, `std::process`, `std::net` and `std::env`, which need
//! no dependency at all.
//!
//! # Empty on purpose
//!
//! The crate and its gate landed before any code moved, so that the first
//! module to arrive is already constrained. Migration order and per-module
//! measurements: `docs/design/2026-07-28-ba3-phase-2-incremental-sequence.md`.
