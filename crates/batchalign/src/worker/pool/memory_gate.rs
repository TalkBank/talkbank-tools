//! Live available-memory admission gate.
//!
//! # Why this exists
//!
//! The first fundamental of admission policy is "don't crash from
//! running out of memory." Today the only memory check at the
//! worker-pool layer happens deep inside [`crate::worker::handle::WorkerHandle::spawn`]
//! via `memory_guard::acquire_spawn_permit`, which:
//!
//! 1. only fires after the global / per-key permit dance has already
//!    counted the spawn against capacity, and
//! 2. derives its floor from `recommend_memory_gate_mb` — a
//!    tier-derived formula ("leave 8 GB free on a 256 GB host or 2 GB
//!    free on a 16 GB host") that doesn't reflect what's actually
//!    running.
//!
//! This module replaces (1) with an admission-time predicate that
//! refuses spawns *before* permit acquisition, and replaces (2) with a
//! single hardcoded floor: always leave at least
//! `MIN_FREE_MEMORY_MB` MB available for the OS / non-batchalign
//! processes. The constant is mechanical-policy — no env var, no
//! `PoolConfig` field, no operator override. "Operator" is a fiction
//! at this seam: the daemon is deployed by `pyinfra` from a
//! Claude-generated `server.yaml` that no human reads or edits.
//!
//! # The number
//!
//! 2048 MB = 2 GB. Rationale:
//!
//! - On a 16 GB host (Small tier in the legacy formula), 2 GB free is
//!   the lowest credible OS protection floor — below it, macOS will
//!   start swapping aggressively and the user-facing experience
//!   degrades sharply. 2 GB matches the pre-rearch Small-tier headroom
//!   exactly, so a 16 GB user sees no behavioral change.
//! - On a 256 GB host (Fleet tier), 2 GB is trivial — but the floor
//!   is supposed to be trivial there. Workload sizing on large hosts
//!   is the job of per-process RSS observation (a follow-on), not
//!   this floor.
//! - The OS-protection floor is fundamentally about absolute headroom,
//!   not workload sizing. A single absolute number is the right shape;
//!   tiered numbers were an attempt to encode workload sizing into the
//!   floor and that's what produced the "wrong on every host" failure
//!   modes the rearch is replacing.
//!
//! # Where this plugs in
//!
//! Called from
//! [`crate::worker::pool::WorkerPool::try_claim_spawn_slot`] as
//! Layer 0.5 — after the [`super::cpu_gate`] check, before the
//! global-permit acquisition. Memory and CPU rejections do not
//! consume permits; they refuse the spawn outright and let the
//! caller retry once the host frees up.

/// Minimum free memory in MB that must remain available for the OS
/// and non-batchalign processes. The admission gate refuses worker
/// spawns when [`current_available_memory_mb`] is at or below this
/// value. Hardcoded by design — see module docs for rationale.
///
/// Visible at crate scope so the host-memory coordinator
/// (`crate::host_memory`) and the `ServerConfig` resolver
/// (`crate::types::config::resolve`) agree with the admission gate
/// on what "minimum-free" means. There must be exactly one such
/// number in the codebase; that number is this one.
pub(crate) const MIN_FREE_MEMORY_MB: u64 = 2048;

/// Information about the memory reading that triggered an admission
/// refusal. Carries the live reading, the reservation the projection
/// was made against, and the floor that was breached, so
/// observability can report
/// "available=10240 MB minus new-worker reservation=12000 MB would
/// leave -1760 MB; floor=2048 MB" rather than just a boolean. Not
/// surfaced beyond the admission seam today; the pool's
/// `memory_constrained_rejections_total` counter is the metrics-layer
/// surface.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct MemoryConstrained {
    /// Available memory in MB at the time of the check.
    pub(super) available_mb: u64,
    /// The new worker's projected reservation in MB. Subtracted
    /// from `available_mb` to compute the post-spawn projection.
    /// `0` means "no reservation projection" (i.e., the gate is
    /// only enforcing the floor, equivalent to the original
    /// pre-Mode-A behavior).
    pub(super) reservation_mb: u64,
    /// Floor the projected post-spawn memory was compared against.
    pub(super) threshold_mb: u64,
}

/// Read the host's currently-available memory in MB.
///
/// Delegates to [`crate::worker::memory_guard::available_memory_mb`]
/// so the admission gate and the in-spawn `memory_guard` agree on
/// what "available" means — important on macOS, where naive
/// `sysinfo::available_memory()` undercounts by excluding inactive
/// pages that the kernel can reclaim on demand.
fn current_available_memory_mb() -> u64 {
    crate::worker::memory_guard::available_memory_mb()
}

/// The hardcoded minimum-free threshold. Returned by a function (not
/// inlined as the constant directly) so unit tests can audit "what
/// number is the production gate using" without duplicating the
/// constant name.
pub(super) fn host_min_free_mb_threshold() -> u64 {
    MIN_FREE_MEMORY_MB
}

/// Check whether admitting one more worker would leave host memory
/// above the OS-protection floor. Forward-looking projection:
/// returns `Err(MemoryConstrained)` when
/// `available_mb < reservation_mb + threshold_mb` (i.e., spawning a
/// worker that consumes its reservation would push live free memory
/// at or below the floor); otherwise `Ok(())`.
///
/// `reservation_mb = 0` reduces this to the pre-projection check
/// (`available_mb > threshold_mb`), used by tests that exercise the
/// floor in isolation. Production callers pass the new worker's
/// `startup_reservation_mb_for_tier` so the projection accounts for
/// the worker's anticipated load. Reservation is a static
/// per-profile estimate — a known imperfection, not the rearch's
/// final word; per-process RSS observation (Mode B follow-on)
/// replaces it with measured-vs-estimated comparison once landed.
///
/// Both inputs are passed (rather than read inside) so unit tests
/// can drive every branch deterministically without depending on
/// host state.
pub(super) fn check_memory_saturation(
    threshold_mb: u64,
    reservation_mb: u64,
) -> Result<(), MemoryConstrained> {
    let available_mb = current_available_memory_mb();
    let projected_after_spawn_mb = available_mb.saturating_sub(reservation_mb);
    if projected_after_spawn_mb <= threshold_mb {
        Err(MemoryConstrained {
            available_mb,
            reservation_mb,
            threshold_mb,
        })
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The production constant must equal exactly 2048 MB. If anyone
    /// ever changes this number, the change should be deliberate
    /// enough to update this test alongside it.
    #[test]
    fn production_threshold_is_2048_mb() {
        assert_eq!(MIN_FREE_MEMORY_MB, 2048);
        assert_eq!(host_min_free_mb_threshold(), 2048);
    }

    /// Threshold of `u64::MAX` makes the gate certain to refuse with
    /// any reservation: any real machine has finite memory and the
    /// post-spawn projection trivially falls at or below the floor.
    /// Exercises the rejection branch deterministically.
    #[test]
    fn rejects_when_floor_alone_exceeds_available() {
        let result = check_memory_saturation(u64::MAX, 0);
        let Err(constrained) = result else {
            panic!("expected MemoryConstrained rejection at threshold=u64::MAX");
        };
        assert_eq!(constrained.threshold_mb, u64::MAX);
        assert_eq!(constrained.reservation_mb, 0);
        assert!(constrained.available_mb <= u64::MAX);
    }

    /// Reservation of `u64::MAX` with floor=0 also forces refusal:
    /// the projected post-spawn memory saturates at 0, which is at
    /// or below the floor. Pins the projection arithmetic.
    #[test]
    fn rejects_when_reservation_alone_exceeds_available() {
        let result = check_memory_saturation(0, u64::MAX);
        let Err(constrained) = result else {
            panic!("expected rejection when reservation > available");
        };
        assert_eq!(constrained.reservation_mb, u64::MAX);
        assert_eq!(constrained.threshold_mb, 0);
    }

    /// Floor=0 and reservation=0 means the predicate reduces to
    /// "is available > 0?" — won't fire on a running machine.
    /// Exercises the admit branch without depending on host
    /// idleness.
    #[test]
    fn admits_when_floor_and_reservation_are_zero() {
        let result = check_memory_saturation(0, 0);
        assert!(
            result.is_ok(),
            "expected Ok at floor=0,reservation=0 on a running host, got {result:?}"
        );
    }

    /// Confirms the projection arithmetic: with reservation 1 MB and
    /// floor 0, the gate must admit (any real host has more than
    /// 1 MB free).
    #[test]
    fn admits_when_projected_post_spawn_well_above_floor() {
        let result = check_memory_saturation(0, 1);
        assert!(
            result.is_ok(),
            "expected Ok at floor=0,reservation=1 on a running host, got {result:?}"
        );
    }
}
