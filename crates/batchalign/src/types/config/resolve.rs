//! `ServerConfig` methods: validation and memory-tier resolution.
//!
//! These are `impl ServerConfig` blocks that depend on runtime types
//! (`MemoryTier`) and the default-value helpers from [`super::server`].

use super::server::*;

impl ServerConfig {
    /// Resolve the effective memory tier, applying config overrides.
    ///
    /// Priority: explicit `memory_tier` field in config → auto-detect from RAM.
    /// Individual startup reservation overrides (`gpu_startup_mb`, etc.) are
    /// applied on top of the resolved tier.
    pub fn resolved_memory_tier(&self) -> crate::types::runtime::MemoryTier {
        use crate::types::runtime::{MemoryTier, MemoryTierKind};

        let mut tier = match self.memory_tier {
            Some(MemoryTierKind::Small) => MemoryTier::from_total_mb(16_000),
            Some(MemoryTierKind::Medium) => MemoryTier::from_total_mb(32_000),
            Some(MemoryTierKind::Large) => MemoryTier::from_total_mb(64_000),
            Some(MemoryTierKind::Fleet) => MemoryTier::from_total_mb(256_000),
            None => MemoryTier::detect(),
        };
        if let Some(value) = self.gpu_startup_mb {
            tier.gpu_startup_mb = value;
        }
        if let Some(value) = self.stanza_startup_mb {
            tier.stanza_startup_mb = value;
        }
        if let Some(value) = self.io_startup_mb {
            tier.io_startup_mb = value;
        }
        tier
    }

    /// Resolve host-memory headroom (MB).
    ///
    /// `Some(n)` is an explicit operator override (used today by
    /// `--sequential` mode to set `MemoryMb(1)`); `None` falls
    /// through to the hardcoded
    /// [`crate::worker::pool::memory_gate::MIN_FREE_MEMORY_MB`]
    /// floor, the same number the worker-pool admission gate
    /// enforces. The previous tier-derived fallback
    /// (`resolved_memory_tier().headroom_mb`, 2/4/8 GB by host RAM)
    /// has been retired — it tried to encode workload sizing into a
    /// floor that should only express OS-protection headroom.
    pub fn resolved_memory_gate_mb(&self) -> crate::api::MemoryMb {
        match self.memory_gate_mb {
            Some(value) => value,
            None => crate::api::MemoryMb(crate::worker::pool::memory_gate::MIN_FREE_MEMORY_MB),
        }
    }

    /// Resolve warmup commands before server-side capability filtering.
    ///
    /// Returns `warmup_commands` directly — the CLI `--warmup` flag and
    /// `server.yaml` both write to this field. An empty list means no warmup.
    pub fn resolved_warmup_commands(&self) -> &[String] {
        &self.warmup_commands
    }

    /// Return a list of warnings (non-fatal) about the config.
    pub fn validate(&mut self) -> Vec<String> {
        let mut warnings = Vec::new();

        for root in &self.media_roots {
            if !root.as_path().is_dir() {
                warnings.push(format!("media_root does not exist: {root}"));
            }
        }
        for (key, root) in &self.media_mappings {
            if !root.as_path().is_dir() {
                warnings.push(format!("media_mapping '{key}' root does not exist: {root}"));
            }
        }
        if self.port == 0 {
            warnings.push(format!(
                "port must be 1-65535 (got {}), defaulting to 8000",
                self.port
            ));
            self.port = 8000;
        }
        if self.job_ttl_days < 1 {
            warnings.push(format!(
                "job_ttl_days must be >= 1 (got {}), defaulting to 1",
                self.job_ttl_days
            ));
            self.job_ttl_days = 1;
        }
        if self.memory_gate_poll_s == 0 {
            warnings.push("memory_gate_poll_s must be >= 1, defaulting to 1".into());
            self.memory_gate_poll_s = 1;
        }
        if self.max_concurrent_worker_startups == 0 {
            warnings.push("max_concurrent_worker_startups must be >= 1, defaulting to 1".into());
            self.max_concurrent_worker_startups = 1;
        }
        warnings
    }
}
