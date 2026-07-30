//! Server configuration: mirrors `batchalign/serve/config.py`.
//!
//! Deserializes from the runtime-owned `server.yaml` under the resolved state
//! directory using serde_yaml.
//! No OmegaConf interpolation needed, plain YAML is sufficient.
//!
//! # Sub-modules
//!
//! | Module    | Purpose |
//! |-----------|---------|
//! | [`layout`]  | `RuntimeLayout`, filesystem path resolution from env/home |
//! | [`server`]  | `ServerConfig` struct, `FleetTarget`, serde defaults, warmup presets |
//! | [`resolve`] | `ServerConfig` methods: validation and memory-tier resolution |
//! | [`load`]    | YAML loading helpers and `ConfigError` |

mod layout;
mod load;
mod resolve;
mod server;

#[cfg(test)]
mod tests;

// Re-export everything at the `config` module level for backwards compatibility.
// Callers use `crate::config::ServerConfig`, `crate::config::RuntimeLayout`, etc.
pub use layout::*;
pub use load::*;
pub use server::*;

/// Default host-memory headroom, in MB: the floor `ServerConfig` falls back to
/// when no `memory_gate_mb` override is set, and the same number the worker
/// pool's admission gate enforces.
///
/// Declared here rather than in `worker/pool/memory_gate.rs`, where it lived
/// until 2026-07-30. Its own doc comment there already conceded that the
/// `ServerConfig` resolver and the host-memory coordinator both had to reach
/// into the pool to read it, which is the wrong direction: configuration
/// declares a default and the gate enforces it, not the reverse. It was also
/// one of the three references that made `types` depend on `worker`, and so
/// held every module downstream of `types` out of the core crate.
///
/// The authoritative TIER-AWARE floor is still
/// `worker::pool::memory_gate::host_min_free_mb_threshold_for_tier`; this is
/// only the default headroom.
pub(crate) const MIN_FREE_MEMORY_MB: u64 = 2048;
