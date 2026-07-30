//! Per-key coordination for shared GPU worker creation.
//!
//! # Why this type exists
//!
//! The `gpu_workers` map used to hold `Arc<SharedGpuWorker>` directly, and
//! [`WorkerPool::get_or_create_gpu_worker`](super::WorkerPool::get_or_create_gpu_worker)
//! held the map's mutex across the entire slow path: a process-global spawn
//! semaphore, a cross-process host-memory lease, the Python process spawn, the
//! wait for `{"ready": true}` and a capabilities round trip. Holding the lock was
//! deliberate and prevented a real bug (two callers racing to spawn two worker
//! processes for one key), but it did so by making every OTHER user of the map
//! wait for an unrelated key's model load: dispatches whose worker was already
//! warm, and `/health`, which walks the same map. On a real host that is tens of
//! seconds of unrelated stalling per cold key.
//!
//! A slot moves the coordination from the MAP to the KEY. The map lock is held
//! only long enough to hand out a slot; the spawn runs with the lock released,
//! and duplicate spawns are still impossible because every caller for one key
//! shares one slot.
//!
//! # What this deliberately does NOT change
//!
//! Spawns still serialize globally. `memory_guard::acquire_spawn_permit` takes a
//! process-global semaphore with a single permit, held until the worker signals
//! ready, so that each spawn's memory check sees the previous worker's models
//! already resident. Per-key coordination is orthogonal to that: it stops work
//! that needs NO spawn from queuing behind one. Two cold keys still come up one
//! after the other, by design.
//!
//! # Failure is retryable, and eviction would be a bug
//!
//! [`tokio::sync::OnceCell::get_or_try_init`] leaves the cell uninitialized when
//! the initializer returns an error, so a failed spawn simply lets the next
//! caller try again; there is no poisoning to recover from. Removing the slot
//! from the map on failure would be actively wrong: another caller may already
//! hold a clone of that slot and be initializing through it, and its worker
//! would then exist with no map entry pointing at it, i.e. an orphaned worker
//! process that shutdown cannot find.

use std::sync::Arc;

use tokio::sync::OnceCell;

use super::shared_gpu::SharedGpuWorker;

/// One `gpu_workers` entry: a live shared GPU worker, or a spawn in flight for
/// that key.
///
/// Cheap to clone; every clone refers to the same underlying cell, which is what
/// makes "one spawn per key" hold without the map lock.
#[derive(Clone, Default)]
pub(in crate::worker) struct GpuWorkerSlot(Arc<OnceCell<Arc<SharedGpuWorker>>>);

/// What a slot holds at the moment it is read.
///
/// An enum rather than an `Option` so that every reader of the map has to say
/// what an in-flight spawn means for IT. The two answers differ: a status
/// listing wants to report it, and a worker count must not count it.
pub(in crate::worker) enum GpuSlotState<'a> {
    /// A spawn for this key is in flight; no worker exists yet.
    ///
    /// Transient by construction: the slot resolves to [`Self::Ready`] when the
    /// spawn succeeds, or back to `Spawning` (retryable) when it fails.
    Spawning,
    /// The worker is live and usable.
    Ready(&'a Arc<SharedGpuWorker>),
}

impl GpuWorkerSlot {
    /// A slot with no worker yet. Inserted into the map by the caller that
    /// first asks for a key, before its spawn begins.
    pub(in crate::worker) fn pending() -> Self {
        Self::default()
    }

    /// Read the slot without waiting for an in-flight spawn.
    pub(in crate::worker) fn state(&self) -> GpuSlotState<'_> {
        match self.0.get() {
            Some(worker) => GpuSlotState::Ready(worker),
            None => GpuSlotState::Spawning,
        }
    }

    /// The live worker, if this slot has one. For callers that are draining the
    /// map and cannot borrow from it.
    pub(in crate::worker) fn ready_worker(&self) -> Option<Arc<SharedGpuWorker>> {
        self.0.get().cloned()
    }

    /// Return this key's worker, running `init` exactly once across all callers
    /// holding this slot.
    ///
    /// Concurrent callers for the same key await the one initialization;
    /// callers for other keys are unaffected, because they hold different slots
    /// and the map lock is not held here.
    pub(in crate::worker) async fn worker_or_init<Init, Fut>(
        &self,
        init: Init,
    ) -> Result<Arc<SharedGpuWorker>, crate::worker::error::WorkerError>
    where
        Init: FnOnce() -> Fut,
        Fut: Future<Output = Result<Arc<SharedGpuWorker>, crate::worker::error::WorkerError>>,
    {
        self.0.get_or_try_init(init).await.cloned()
    }
}
