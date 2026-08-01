//! Worker pool preparation for both direct and server execution hosts.
//!
//! This module owns the capability probing and pool construction logic that
//! is shared between direct-mode CLI execution and the HTTP server. It does
//! NOT depend on axum, sqlx, or any server-specific crate.

use std::collections::BTreeMap;
use std::sync::Arc;

use tracing::info;

use crate::cache::UtteranceCache;
use crate::capability::{
    WorkerCapabilitySnapshot, resolve_worker_capability_snapshot, validate_infer_capability_gate,
};
use crate::command_model::command_specs;
use crate::error;
use crate::runner::{ExecutionEngine, RunnerExecutionContext};
use crate::worker::InferTask;
use crate::worker::pool::{PoolConfig, WorkerPool};

// ---------------------------------------------------------------------------
// Prepared worker subsystem
// ---------------------------------------------------------------------------

/// Prepared worker subsystem that can be reused across multiple app instances.
///
/// Tests use this seam to amortize capability probing while still creating a
/// fresh control plane and runtime-owned filesystem layout for each isolated
/// session.
#[derive(Clone)]
pub struct PreparedWorkers {
    pool: Arc<WorkerPool>,
    capabilities: Vec<String>,
    infer_tasks: Vec<InferTask>,
    engine_versions: BTreeMap<String, String>,
    test_echo_mode: bool,
}

/// One host-neutral execution runtime resolved from prepared workers.
pub(crate) struct ResolvedExecutionRuntime {
    pub capability_snapshot: WorkerCapabilitySnapshot,
    pub engine: ExecutionEngine,
}

impl PreparedWorkers {
    /// Resolve the latest capability snapshot, preferring live detected worker
    /// data over the startup placeholder snapshot when available.
    pub(crate) fn capability_snapshot(
        &self,
    ) -> Result<WorkerCapabilitySnapshot, error::ServerError> {
        resolve_worker_capability_snapshot(
            &self.capabilities,
            &self.infer_tasks,
            &self.engine_versions,
            self.test_echo_mode,
            self.pool.detected_capabilities(),
        )
    }

    /// Return the released command surface discovered during worker probing.
    pub fn capabilities(&self) -> &[String] {
        &self.capabilities
    }

    /// Return the infer-task set reported by the prepared worker subsystem.
    pub fn infer_tasks(&self) -> &[InferTask] {
        &self.infer_tasks
    }

    /// Return the latest infer-task view, preferring live worker detection
    /// over the startup placeholder snapshot when available.
    pub fn current_infer_tasks(&self) -> Result<Vec<InferTask>, error::ServerError> {
        Ok(self.capability_snapshot()?.infer_tasks)
    }

    /// Build one host-neutral execution runtime over this prepared worker set.
    pub(crate) fn resolve_execution_runtime(
        &self,
        cache: Arc<UtteranceCache>,
    ) -> Result<ResolvedExecutionRuntime, error::ServerError> {
        let capability_snapshot = self.capability_snapshot()?;
        let engine = ExecutionEngine::new(RunnerExecutionContext::new(
            self.pool.clone(),
            cache,
            capability_snapshot.infer_tasks.clone(),
            capability_snapshot.engine_versions.clone(),
            self.test_echo_mode,
        ));
        Ok(ResolvedExecutionRuntime {
            capability_snapshot,
            engine,
        })
    }

    /// Return a reference to the underlying worker pool.
    ///
    /// Exposed for server-only code that needs direct pool access (e.g.,
    /// building [`AppState`](crate::state::AppState)).
    pub fn pool(&self) -> &Arc<WorkerPool> {
        &self.pool
    }
}

// ---------------------------------------------------------------------------
// Worker probing and preparation
// ---------------------------------------------------------------------------

/// Whether pool preparation adopts TCP workers already listed in the registry.
///
/// A named choice rather than a `bool` parameter: the two call sites differ on
/// a real policy question, not on a flag. Server preparation adopts
/// pre-started daemons; direct inline execution deliberately does not, so a
/// one-shot CLI run never inherits a detached daemon it did not start and will
/// not retire.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegistryDiscovery {
    /// Adopt TCP workers found in the registry file.
    Adopt,
    /// Ignore the registry; use only workers this process creates.
    Ignore,
}

/// Build the worker pool and resolve its capability surface.
///
/// The returned [`PreparedWorkers`] value owns a live [`WorkerPool`] plus the
/// capability metadata derived from it. Callers can share that value across
/// multiple app instances to keep expensive model loads hot while still
/// rebuilding the server control plane and runtime-owned temp directories.
///
/// Returns as soon as capabilities are resolved; no worker is spawned here.
/// Workers arrive either from the registry (a pre-started TCP daemon, adopted
/// when `discovery` is [`RegistryDiscovery::Adopt`]) or from the job runner's
/// per-job `pre_scale_for_command_options`.
///
/// Capabilities are detected lazily on the first real worker spawn rather than
/// at startup, which avoids a 10-30 second delay and a 2-3 GB peak from a probe
/// worker on small machines. In test-echo mode they are synthesized instead.
///
/// This used to be three functions: `prepare_workers`, a
/// `prepare_workers_background` that differed only by spawning startup warmup
/// off-thread, and a `prepare_direct_workers`. With warmup retired (2026-07-30)
/// the first two became identical and the third became this one plus an
/// argument.
pub async fn prepare_workers(
    pool_config: PoolConfig,
    discovery: RegistryDiscovery,
) -> Result<PreparedWorkers, error::ServerError> {
    let test_echo_mode = pool_config.test_echo;
    let pool = Arc::new(WorkerPool::new(pool_config));
    pool.start_background_tasks();

    if discovery == RegistryDiscovery::Adopt {
        let discovered = pool.discover_from_registry().await;
        if discovered > 0 {
            info!(discovered, "Pre-started TCP workers integrated into pool");
        }
    }

    // Optimistic capabilities: accept all released commands.
    // Real capabilities are detected lazily on first worker spawn.
    let (capabilities, infer_tasks, engine_versions) = if test_echo_mode {
        let all_tasks: Vec<InferTask> = optimistic_infer_tasks();
        let caps = validate_infer_capability_gate(&all_tasks, &BTreeMap::new(), true)?;
        (caps, all_tasks, BTreeMap::new())
    } else if let Some(detected) = pool.detected_capabilities() {
        // TCP registry workers were discovered and capabilities probed.
        let caps = validate_infer_capability_gate(
            &detected.infer_tasks,
            &detected.engine_versions,
            false,
        )?;
        info!(
            capabilities = ?caps,
            infer_tasks = ?detected.infer_tasks,
            "Using capabilities detected from TCP registry workers"
        );
        (
            caps,
            detected.infer_tasks.clone(),
            detected.engine_versions.clone(),
        )
    } else {
        // Optimistic mode: advertise all released commands and their infer
        // tasks. Real capabilities are refined lazily when the first worker
        // spawns and reports its actual engine versions. This avoids a
        // 10-30 second startup delay from spawning a probe worker.
        let all_tasks: Vec<InferTask> = optimistic_infer_tasks();
        let caps = optimistic_capabilities();
        info!(
            capabilities = ?caps,
            "Using optimistic capabilities (lazy detection on first worker spawn)"
        );
        (caps, all_tasks, BTreeMap::new())
    };

    Ok(PreparedWorkers {
        pool,
        capabilities,
        infer_tasks,
        engine_versions,
        test_echo_mode,
    })
}

/// All released commands: used as the optimistic capability set before
/// the first real worker spawn confirms what's actually installed.
fn optimistic_capabilities() -> Vec<String> {
    command_specs()
        .iter()
        .map(|spec| spec.command.to_string())
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect()
}

/// Every infer task some released command is advertised from: the optimistic
/// task set claimed before a live worker reports what it can actually do.
fn optimistic_infer_tasks() -> Vec<InferTask> {
    command_specs()
        .iter()
        .map(|spec| spec.capabilities.primary_infer_task)
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect()
}
