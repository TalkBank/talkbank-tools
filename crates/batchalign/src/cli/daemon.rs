//! Local daemon lifecycle: mirrors `batchalign/cli/daemon.py`.
//!
//! Key difference from Python: the daemon starts the **Rust server binary**
//! (`batchalign3 serve start --foreground`), not the Python server.
//!
//! ## Port policy
//!
//! The daemon takes the port REQUEST from the server config (default 8000)
//! and passes it through to the child, which may be `0` for "let the OS
//! choose". Discovery stays deterministic without a fixed port because the
//! child PUBLISHES the port it actually bound, in its handshake, and every
//! reader takes it from there; the old rule against ephemeral ports existed
//! only because nothing recorded the answer.
//!
//! ## Stale-binary detection
//!
//! `DaemonInfo` carries a `build_hash` (set at write time from
//! [`crate::cli::build_hash()`]).  `ensure_daemon_locked()` compares it against
//! the current binary's hash and auto-restarts on mismatch.  Old daemon.json
//! files that lack `build_hash` fall back to version comparison.

use std::path::{Path, PathBuf};
use std::time::Duration;

use std::num::NonZeroU16;

use crate::config::{PortRequest, RuntimeLayout};
use crate::server_handshake::{HandshakeSlot, ServerHandshake};
use fs2::FileExt;
use serde::{Deserialize, Serialize};
use tracing::debug;

use crate::cli::client::BatchalignClient;
use crate::cli::error::CliError;
use crate::cli::python::resolve_python_executable;

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// Maximum seconds for a newly spawned daemon to become usable, covering BOTH
/// phases: reaching its bind, and then answering `/health`.
///
/// 90 seconds is generous enough for a cold start on a slow machine while
/// still failing within a human-tolerable window if something is genuinely
/// broken (port conflict, missing Python, bad config).
///
/// It is ONE budget on purpose. The two phases used to have separate ceilings,
/// 30 s to publish a port and 90 s to answer afterwards, which put the small
/// number on the slow phase: everything expensive (config validation,
/// host-facts detection, database migration, cache init, and on a
/// freshly-built binary the first-exec cost) happens before the bind, while
/// the second phase only confirms a socket that already exists. A cold start
/// slower than 30 s therefore failed on the wrong clock, and the worst case
/// was 120 s rather than the 90 the constant advertises.
const HEALTH_TIMEOUT: f64 = 90.0;

/// The whole budget for getting a spawned daemon to a usable state.
///
/// Shared by `serve start` and the auto-daemon path so both mean the same
/// thing by "the daemon did not come up".
pub(crate) fn startup_budget() -> Duration {
    Duration::from_secs_f64(HEALTH_TIMEOUT)
}

/// How often to poll the daemon's `/health` endpoint while waiting for
/// startup. 1 second balances responsiveness (the user sees the daemon come
/// up within a second of it being ready) against CPU/network cost (one
/// loopback HTTP request per second is negligible).
const HEALTH_POLL: f64 = 1.0;

fn runtime_layout() -> RuntimeLayout {
    RuntimeLayout::from_env()
}

/// The loopback URL a locally-running server can be reached at.
///
/// Answers "where is the local server" from the strongest evidence available,
/// in order: the port the server PUBLISHED after binding, then the port the
/// config asked for if that request named one. `None` means there is nothing
/// to try, which for an ephemeral request with no published handshake is the
/// honest answer rather than a URL built from a placeholder.
///
/// One owner for a question that had two callers building the URL from
/// `cfg.port` directly, both of which silently assumed the request had been
/// granted.
pub(crate) fn local_server_url(layout: &RuntimeLayout, configured: PortRequest) -> Option<String> {
    local_port(layout, configured).map(|port| format!("http://127.0.0.1:{port}"))
}

/// The port a local server can be reached on, from the strongest evidence.
///
/// Takes the configured request rather than loading `server.yaml` itself.
/// Every caller already holds a validated config, and reloading it here meant a
/// second parse plus a second `is_dir()` on every configured media root, and
/// printed every config warning a second time.
pub(crate) fn local_port(layout: &RuntimeLayout, configured: PortRequest) -> Option<NonZeroU16> {
    if let Ok(Some(handshake)) = ServerHandshake::read(layout.state_dir(), HandshakeSlot::Main)
        && let Some(port) = handshake.bound_port()
    {
        return NonZeroU16::new(port.get());
    }
    // No published port: either an older server, or none running. A fixed
    // request is a usable guess for the first case; an ephemeral one leaves
    // nothing to guess with, and guessing is what this change removed.
    configured.fixed()
}

/// Return the port REQUEST from `server.yaml` (or the default).
///
/// A request, not a port: see [`PortRequest`]. The auto-daemon path needs a
/// concrete number before the child binds, so it must decide what to do with
/// an ephemeral request rather than being handed a plausible-looking integer.
fn configured_port_request(layout: &RuntimeLayout) -> Result<PortRequest, CliError> {
    let (cfg, warnings) = crate::config::load_validated_config_from_layout(layout, None)?;
    for warning in warnings {
        eprintln!("warning: {warning}");
    }
    Ok(cfg.port)
}

// ---------------------------------------------------------------------------
// Profiles
// ---------------------------------------------------------------------------

/// Which daemon role to start or connect to.
///
/// The CLI can manage two independent daemon processes simultaneously.
/// Each profile gets its own state file, lock file, and log file so they
/// do not interfere with each other.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DaemonProfile {
    /// The primary managed daemon profile for explicit `serve` lifecycle
    /// commands, auto-daemon CLI routing, and compatibility helpers. Uses the
    /// system or venv Python resolved by [`resolve_python_executable()`].
    Main,
    /// A secondary daemon dedicated to transcribe workloads that require
    /// a different Python environment or model stack from the main daemon.
    /// Selected when
    /// the dispatch layer detects a transcribe command and the sidecar
    /// Python is available. Uses `BATCHALIGN_SIDECAR_PYTHON` or falls
    /// back to `~/.batchalign3/sidecar/.venv/bin/python`.
    Sidecar,
}

impl DaemonProfile {
    fn label(self) -> &'static str {
        match self {
            Self::Main => "local",
            Self::Sidecar => "sidecar",
        }
    }

    /// The handshake slot this profile's server publishes to.
    ///
    /// Every other per-profile artifact was already namespaced; this closes the
    /// last shared one, which mattered once discovery started reading the
    /// published port rather than re-deriving it from config.
    fn handshake_slot(self) -> HandshakeSlot {
        match self {
            Self::Main => HandshakeSlot::Main,
            Self::Sidecar => HandshakeSlot::Sidecar,
        }
    }

    fn state_file(self, dir: &Path) -> PathBuf {
        match self {
            Self::Main => dir.join("daemon.json"),
            Self::Sidecar => dir.join("sidecar-daemon.json"),
        }
    }

    fn lock_file(self, dir: &Path) -> PathBuf {
        match self {
            Self::Main => dir.join("daemon.lock"),
            Self::Sidecar => dir.join("sidecar-daemon.lock"),
        }
    }

    fn log_file(self, dir: &Path) -> PathBuf {
        match self {
            Self::Main => dir.join("daemon.log"),
            Self::Sidecar => dir.join("sidecar-daemon.log"),
        }
    }

    fn startup_message(self) -> &'static str {
        match self {
            Self::Main => "Starting local daemon...",
            Self::Sidecar => "Starting sidecar daemon for transcribe workloads...",
        }
    }

    fn check_manual_server(self) -> bool {
        matches!(self, Self::Main)
    }

    fn default_python(self, dir: &Path) -> String {
        match self {
            Self::Main => resolve_python_executable(),
            Self::Sidecar => std::env::var("BATCHALIGN_SIDECAR_PYTHON").unwrap_or_else(|_| {
                dir.join("sidecar")
                    .join(".venv")
                    .join("bin")
                    .join("python")
                    .to_string_lossy()
                    .into_owned()
            }),
        }
    }

    fn require_sidecar_python_file(self) -> bool {
        matches!(self, Self::Sidecar) && std::env::var("BATCHALIGN_SIDECAR_PYTHON").is_err()
    }
}

// ---------------------------------------------------------------------------
// DaemonInfo: state file
// ---------------------------------------------------------------------------

/// State persisted to `daemon.json` so the CLI can reconnect to a running daemon.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DaemonInfo {
    /// OS process ID of the daemon.
    pub pid: u32,
    /// Daemon version string (for stale-binary detection).
    #[serde(default)]
    pub version: String,
    /// Unix timestamp when the daemon was started.
    #[serde(default)]
    pub started_at: f64,
    /// Build fingerprint: empty for old state files (falls back to version).
    #[serde(default)]
    pub build_hash: String,
    /// The daemon process's *resolved* `force_cpu` value (operator
    /// intent merged with the host-facts recommendation), captured at
    /// spawn time. Used by `runtime_mismatch` to decide whether the
    /// next CLI invocation needs a restart: if the next call's
    /// resolved value differs from this stored one, the daemon's
    /// view of the host has drifted (operator edited server.yaml, or
    /// the CLI's `--force-cpu` flag changed) and the daemon must
    /// restart to pick up the new value.
    ///
    /// Pre-C2.2 daemon.json files recorded the raw CLI `--force-cpu`
    /// flag here. The shape is unchanged (still a `bool`), but the
    /// semantic shifted: resolved-vs-resolved comparison now matches
    /// the daemon's actual runtime behavior. Existing daemon.json
    /// files trigger one self-correcting restart on first contact
    /// post-upgrade: Apple Silicon hosts go from raw=false to
    /// resolved=true and the next invocation kicks the daemon over.
    #[serde(default)]
    pub force_cpu: bool,
    /// The daemon's resolved `allow_mps` value (the explicit Apple-GPU
    /// opt-in), captured at spawn time, compared by `runtime_mismatch`
    /// the same way as `force_cpu`. `#[serde(default)]` keeps older
    /// daemon.json files readable (they resolve to `false`, matching
    /// the pre-flag behavior, so no spurious restart).
    #[serde(default)]
    pub allow_mps: bool,
    /// The `--workers` value the daemon was started with, if any.
    /// `None` means either the operator didn't pass `--workers` at
    /// startup (so the daemon resolved its own per-job parallelism
    /// from host facts) or the daemon.json file pre-dates this field.
    /// Used by the warm-reuse path to fire the `--workers` shadowing
    /// warning only when the requested value actually differs from
    /// the running daemon's: eliminating false positives when the
    /// operator re-passes a value that already matches.
    #[serde(default)]
    pub workers: Option<u32>,
    /// The `--timeout` value the daemon was started with, if any.
    /// Same backcompat + same false-positive elimination story as
    /// `workers`, applied to the daemon's per-task ceiling
    /// (`audio_task_timeout_s`).
    #[serde(default)]
    pub audio_task_timeout_s: Option<u64>,
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Return the main daemon URL or `None` if the daemon cannot be started.
pub async fn ensure_daemon(
    force_cpu: bool,
    allow_mps: bool,
    workers: Option<usize>,
    timeout: Option<u64>,
) -> Result<Option<String>, CliError> {
    ensure_daemon_for(DaemonProfile::Main, force_cpu, allow_mps, workers, timeout).await
}

/// Return the sidecar daemon URL or `None` if the daemon cannot be started.
pub async fn ensure_sidecar_daemon(
    force_cpu: bool,
    allow_mps: bool,
    workers: Option<usize>,
    timeout: Option<u64>,
) -> Result<Option<String>, CliError> {
    ensure_daemon_for(
        DaemonProfile::Sidecar,
        force_cpu,
        allow_mps,
        workers,
        timeout,
    )
    .await
}

/// Stop the main daemon. Returns `true` if a process was killed.
pub async fn stop_daemon() -> Result<bool, CliError> {
    stop_profile(DaemonProfile::Main)
}

/// Stop the sidecar daemon. Returns `true` if a process was killed.
pub async fn stop_sidecar_daemon() -> Result<bool, CliError> {
    stop_profile(DaemonProfile::Sidecar)
}

/// Read the main daemon state file. Public for status checks.
pub fn read_daemon_info() -> Option<DaemonInfo> {
    let layout = runtime_layout();
    read_daemon_info_for(DaemonProfile::Main, layout.state_dir())
}

// ---------------------------------------------------------------------------
// Internal
// ---------------------------------------------------------------------------

async fn ensure_daemon_for(
    profile: DaemonProfile,
    force_cpu: bool,
    allow_mps: bool,
    workers: Option<usize>,
    timeout: Option<u64>,
) -> Result<Option<String>, CliError> {
    let layout = runtime_layout();
    let dir = layout.state_dir();
    std::fs::create_dir_all(dir)?;

    let lock_path = profile.lock_file(dir);
    let lock_file = std::fs::OpenOptions::new()
        .create(true)
        .truncate(false)
        .write(true)
        .open(&lock_path)?;

    match lock_file.try_lock_exclusive() {
        Ok(()) => {}
        Err(_) => {
            debug!(profile = profile.label(), "Waiting for daemon lock...");
            lock_file.lock_exclusive()?;
        }
    }

    let result =
        ensure_daemon_locked(profile, &layout, force_cpu, allow_mps, workers, timeout).await;
    drop(lock_file);
    result
}

fn stop_profile(profile: DaemonProfile) -> Result<bool, CliError> {
    let layout = runtime_layout();
    let dir = layout.state_dir();
    let info = match read_daemon_info_for(profile, dir) {
        Some(i) => i,
        None => return Ok(false),
    };

    let killed = kill_process(info.pid);
    cleanup_state_file_for(profile, dir);
    Ok(killed)
}

/// Check if the daemon state file indicates a stale binary.
///
/// If `build_hash` is present in the state file, compare against the current
/// build hash.  Otherwise, fall back to a version comparison (backward compat
/// with old state files).
fn is_stale(info: &DaemonInfo) -> bool {
    if !info.build_hash.is_empty() {
        return info.build_hash != crate::cli::build_hash();
    }
    // Old state file: fall back to version comparison
    info.version != current_version()
}

fn runtime_mismatch(info: &DaemonInfo, flags: DaemonDeviceFlags) -> bool {
    info.force_cpu != flags.resolved_force_cpu || info.allow_mps != flags.resolved_allow_mps
}

/// True when the operator passed a CLI flag whose value differs from
/// the running daemon's recorded value. Suppresses the warm-reuse
/// shadowing warning when the user re-passes a value that already
/// matches the daemon. A `None` on the daemon side (pre-upgrade
/// daemon.json that lacks the field) is treated as "unknown" and
/// triggers the warning so the user still gets a signal, falling
/// back to the pre-persisted-value behavior on first contact after
/// upgrade.
fn flag_shadows_daemon<T: PartialEq>(requested: Option<T>, persisted: Option<T>) -> bool {
    match (requested, persisted) {
        (None, _) => false,
        (Some(_), None) => true,
        (Some(r), Some(p)) => r != p,
    }
}

/// The operator's device flags as given on the CLI plus their
/// host-resolved values, computed once per invocation so the restart
/// decision and the persisted `DaemonInfo` stay consistent (and so the
/// spawn/persist plumbing threads one named value instead of a parade
/// of positional bools).
#[derive(Clone, Copy, Debug)]
struct DaemonDeviceFlags {
    /// Raw `--force-cpu` switch (drives the spawned subprocess arg).
    cli_force_cpu: bool,
    /// Raw `--allow-mps` switch (drives the spawned subprocess arg).
    cli_allow_mps: bool,
    /// `force_cpu` merged with the host-facts recommendation.
    resolved_force_cpu: bool,
    /// `allow_mps` resolved (no recommendation; operator opt-in only).
    resolved_allow_mps: bool,
}

/// Resolve the daemon's effective `force_cpu` value given the CLI
/// flag plus the host's deployed `server.yaml` and detected facts.
///
/// Mirrors the boundary conversion `serve_cmd::start` does for the
/// daemon process itself: the CLI's presence-only `--force-cpu`
/// switch becomes `Some(true)` in the override, the YAML-side value
/// stays as configured, and the host-facts pipeline merges the two.
/// Used by `ensure_daemon_locked` for restart decisions and by
/// `start_daemon` for the value persisted to `DaemonInfo`.
///
/// A missing or unreadable `server.yaml` falls back to
/// `ServerConfig::default()`: the same behavior `serve_cmd::start`
/// has, so the resolved value computed here matches what the daemon
/// process will see when it boots.
fn resolve_device_flags_for_daemon(
    layout: &RuntimeLayout,
    cli_force_cpu: bool,
    cli_allow_mps: bool,
) -> DaemonDeviceFlags {
    let mut cfg = crate::config::load_config_from_layout(layout, None).unwrap_or_default();
    if cli_force_cpu {
        cfg.force_cpu = Some(true);
    }
    if cli_allow_mps {
        cfg.allow_mps = Some(true);
    }
    let effective = crate::host_facts::EffectiveConfig::resolve_from_server_config(&cfg);
    DaemonDeviceFlags {
        cli_force_cpu,
        cli_allow_mps,
        resolved_force_cpu: effective.force_cpu,
        resolved_allow_mps: effective.allow_mps,
    }
}

async fn ensure_daemon_locked(
    profile: DaemonProfile,
    layout: &RuntimeLayout,
    force_cpu: bool,
    allow_mps: bool,
    workers: Option<usize>,
    timeout: Option<u64>,
) -> Result<Option<String>, CliError> {
    let dir = layout.state_dir();
    if profile.check_manual_server()
        && let Some(url) = detect_manual_server(layout).await?
    {
        return Ok(Some(url));
    }

    // Compute the resolved device flags once: they drive both the
    // restart decision (vs. stored DaemonInfo) and the values persisted
    // when start_daemon writes a new DaemonInfo. Resolving here keeps
    // both consistent.
    let device_flags = resolve_device_flags_for_daemon(layout, force_cpu, allow_mps);

    if let Some(info) = read_daemon_info_for(profile, dir) {
        if is_process_alive(info.pid) {
            if is_stale(&info) {
                eprintln!(
                    "Restarting {} daemon (stale build: {} -> {})...",
                    profile.label(),
                    if info.build_hash.is_empty() {
                        &info.version
                    } else {
                        &info.build_hash
                    },
                    crate::cli::build_hash(),
                );
                kill_process(info.pid);
                cleanup_state_file_for(profile, dir);
                return start_daemon(
                    profile,
                    layout,
                    configured_port_request(layout)?,
                    device_flags,
                    workers,
                    timeout,
                )
                .await;
            }

            if runtime_mismatch(&info, device_flags) {
                eprintln!(
                    "Restarting {} daemon (resolved force_cpu {} -> {}, allow_mps {} -> {})...",
                    profile.label(),
                    info.force_cpu,
                    device_flags.resolved_force_cpu,
                    info.allow_mps,
                    device_flags.resolved_allow_mps,
                );
                kill_process(info.pid);
                cleanup_state_file_for(profile, dir);
                return start_daemon(
                    profile,
                    layout,
                    configured_port_request(layout)?,
                    device_flags,
                    workers,
                    timeout,
                )
                .await;
            }

            // The port comes from the handshake, which is where a bound
            // server publishes it. `daemon.json` used to mirror it; two
            // records of one fact is what this change set removed.
            let published = ServerHandshake::published_port(dir, profile.handshake_slot());
            if let Some(published) = published
                && health_check(published.get()).await
            {
                // The daemon's per-task ceiling (`audio_task_timeout_s`)
                // and per-job parallelism (`max_workers_per_job`) are
                // both fixed at daemon startup. On the warm-reuse path
                // the user's `--timeout` / `--workers` are silently
                // discarded, without surfacing that, a request can
                // fail with a timeout below the requested value, or a
                // multi-file batch can run serially because the daemon
                // stayed at workers=1 (the host-facts auto-clamp on
                // hosts without a usable GPU). Auto-restart is not the
                // answer here: the running daemon may be processing
                // other operators' jobs, and killing it would discard
                // their in-flight work. Warning is honest signal.
                if flag_shadows_daemon(timeout, info.audio_task_timeout_s) {
                    let running = info
                        .audio_task_timeout_s
                        .map(|s| format!("{s}s"))
                        .unwrap_or_else(|| "<unknown: daemon pre-dates this field>".to_string());
                    let requested = timeout
                        .map(|s| format!("{s}s"))
                        .unwrap_or_else(|| "<not set>".to_string());
                    eprintln!(
                        "warning: --timeout {requested} requested but the {} daemon was \
                         started with --timeout {running}. The per-task ceiling stays at \
                         the running value for this submission. To apply the new ceiling, \
                         run `batchalign3 serve stop` then `batchalign3 serve start --timeout \
                         <secs>`, or pass `--no-server` to bypass the daemon entirely.",
                        profile.label(),
                    );
                }
                let info_workers = info.workers.map(|n| n as usize);
                if flag_shadows_daemon(workers, info_workers) {
                    let running = info
                        .workers
                        .map(|n| n.to_string())
                        .unwrap_or_else(|| "<unknown: daemon pre-dates this field>".to_string());
                    let requested = workers
                        .map(|n| n.to_string())
                        .unwrap_or_else(|| "<not set>".to_string());
                    eprintln!(
                        "warning: --workers {requested} requested but the {} daemon was \
                         started with --workers {running}. Per-job parallelism stays at \
                         the running value for this submission. To apply the new value, \
                         run `batchalign3 serve stop` then `batchalign3 serve start --workers \
                         <N>`, or pass `--no-server` to bypass the daemon entirely.",
                        profile.label(),
                    );
                }
                return Ok(Some(format!("http://127.0.0.1:{published}")));
            }

            kill_process(info.pid);
            cleanup_state_file_for(profile, dir);
            return start_daemon(
                profile,
                layout,
                configured_port_request(layout)?,
                device_flags,
                workers,
                timeout,
            )
            .await;
        }
        // Process is dead but state file exists -- stale PID file.
        debug!(
            profile = profile.label(),
            pid = info.pid,
            "Cleaning up stale daemon state file (process is dead)"
        );
        cleanup_state_file_for(profile, dir);
    }

    // Read the port REQUEST only on the path that actually spawns. It used to
    // be read at the top of this function and discarded on the warm-reuse
    // path, which is the common one: a second `server.yaml` parse plus an
    // `is_dir()` per configured media root, and every config warning printed a
    // second time, on every invocation that found a healthy daemon.
    start_daemon(
        profile,
        layout,
        configured_port_request(layout)?,
        device_flags,
        workers,
        timeout,
    )
    .await
}

/// Find a manually-started server, if one is running and reachable.
///
/// Reads the port the server PUBLISHED rather than re-deriving it from
/// `server.yaml`. The config's port is a request: it is what an operator asked
/// for, which for an ephemeral request names no port at all, and even for a
/// fixed request can differ from reality if the file was edited after the
/// server started.
async fn detect_manual_server(layout: &RuntimeLayout) -> Result<Option<String>, CliError> {
    let state_dir = layout.state_dir();
    let handshake = match ServerHandshake::read(state_dir, HandshakeSlot::Main) {
        Ok(Some(handshake)) => handshake,
        Ok(None) => return Ok(None),
        Err(error) => {
            // Deliberately left in place rather than deleted. A file we cannot
            // read is the only evidence that a server may still be running,
            // and removing it invites a second server onto the same port.
            debug!(%error, "Ignoring unreadable server handshake");
            return Ok(None);
        }
    };

    let pid = handshake.pid();
    if !is_process_alive(pid) {
        debug!(pid, "Removing stale server handshake (process is dead)");
        let _ = ServerHandshake::remove(state_dir, HandshakeSlot::Main);
        return Ok(None);
    }

    let port = match handshake.bound_port() {
        Some(port) => port.get(),
        None => {
            // The server has not published a port: either it is still starting,
            // or it predates the published handshake. A fixed configured port
            // is a usable guess for the older-server case; an ephemeral request
            // leaves nothing to guess with, and guessing is what this change
            // exists to stop.
            let (cfg, warnings) = crate::config::load_validated_config_from_layout(layout, None)?;
            for warning in warnings {
                eprintln!("warning: {warning}");
            }
            match cfg.port.fixed() {
                Some(port) => {
                    debug!(
                        pid,
                        port = port.get(),
                        "Server published no port; falling back to the configured one"
                    );
                    port.get()
                }
                None => {
                    debug!(pid, "Server published no port and none is configured");
                    return Ok(None);
                }
            }
        }
    };

    if startup_health_check(port).await {
        // Warn if the manual server has a stale build hash
        check_manual_server_staleness(port).await;
        debug!(port, pid, "Reusing manual server");
        Ok(Some(format!("http://127.0.0.1:{port}")))
    } else {
        Ok(None)
    }
}

/// Best-effort check: warn if a manual server's build hash differs from ours.
async fn check_manual_server_staleness(port: u16) {
    let Ok(client) = BatchalignClient::new() else {
        return;
    };
    if let Ok(health) = client
        .health_check(&format!("http://127.0.0.1:{port}"))
        .await
        && !health.build_hash.is_empty()
        && health.build_hash != crate::cli::build_hash()
    {
        eprintln!(
            "warning: manual server on port {port} has a different build ({}). \
             Restart with `batchalign3 serve stop && batchalign3 serve start`.",
            health.build_hash,
        );
    }
}

/// `flags` carries the raw CLI switches (which control the spawned
/// subprocess's arguments: operator intent verbatim) and the resolved
/// values (persisted to `DaemonInfo` so later `runtime_mismatch`
/// calls compare apples to apples).
async fn start_daemon(
    profile: DaemonProfile,
    layout: &RuntimeLayout,
    port: PortRequest,
    flags: DaemonDeviceFlags,
    workers: Option<usize>,
    timeout: Option<u64>,
) -> Result<Option<String>, CliError> {
    let dir = layout.state_dir();
    let python = profile.default_python(dir);
    if profile.require_sidecar_python_file() && !Path::new(&python).is_file() {
        eprintln!(
            "warning: sidecar python not found at {python}. \
             Set BATCHALIGN_SIDECAR_PYTHON to a Python with transcribe deps."
        );
        return Ok(None);
    }

    eprintln!("{}", profile.startup_message());

    let exe = crate::cli::self_exe::resolve_self_exe();
    let mut cmd = std::process::Command::new(&exe);

    // Read verbose level from server.yaml so fleet deployments can set
    // `verbose: 1` for INFO-level logging without hardcoding in the binary.
    let config_verbose = if layout.config_path().exists() {
        {
            crate::config::load_config_from_layout(layout, None)
                .ok()
                .map(|cfg| cfg.verbose)
                .unwrap_or(0)
        }
    } else {
        0
    };
    for _ in 0..config_verbose {
        cmd.arg("-v");
    }

    cmd.args([
        "serve",
        "start",
        "--foreground",
        "--handshake-slot",
        profile.handshake_slot().as_arg(),
        "--port",
        // The REQUEST, which may be 0 for "let the OS choose". The child
        // publishes what it actually bound and we read that back below, so
        // nothing here has to be a prediction.
        &port.bind_value().to_string(),
        "--host",
        "127.0.0.1",
        "--python",
        &python,
    ]);

    let config_path = layout.config_path();
    if config_path.exists() {
        cmd.arg("--config").arg(config_path);
    }
    if flags.cli_force_cpu {
        cmd.arg("--force-cpu");
    }
    if flags.cli_allow_mps {
        cmd.arg("--allow-mps");
    }
    if let Some(n) = workers {
        cmd.args(["--workers", &n.to_string()]);
    }
    if let Some(t) = timeout {
        cmd.args(["--timeout", &t.to_string()]);
    }

    let log_path = profile.log_file(dir);
    // Append mode: preserve previous daemon logs across restarts.
    // Previous behavior (File::create) truncated logs on every restart,
    // destroying crash diagnostics from the previous session.
    let log_file = std::fs::OpenOptions::new()
        .append(true)
        .create(true)
        .open(&log_path)?;
    cmd.stdout(std::process::Stdio::null());
    cmd.stderr(log_file);

    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        unsafe {
            cmd.pre_exec(|| {
                libc::setsid();
                Ok(())
            });
        }
    }

    // On Windows, CREATE_NEW_PROCESS_GROUP + DETACHED_PROCESS ensures the
    // daemon survives after the spawning CLI process exits, analogous to
    // Unix setsid(). The daemon gets its own console group and is not
    // attached to the parent's console.
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        // CREATE_NEW_PROCESS_GROUP (0x200) | DETACHED_PROCESS (0x08)
        cmd.creation_flags(0x00000200 | 0x00000008);
    }

    let proc = match cmd.spawn() {
        Ok(p) => p,
        Err(e) => {
            eprintln!("warning: failed to spawn daemon process: {e}");
            debug!(profile = profile.label(), error = %e, "Failed to spawn daemon");
            eprintln!(
                "warning: could not start local daemon. Check {}\n\
                 hint: run `batchalign3 serve start --foreground` to see startup errors.",
                log_path.display()
            );
            return Ok(None);
        }
    };

    let pid = proc.id();
    // Persist the resolved value, not the CLI raw bool, so future
    // restart decisions compare against the same merged result the
    // daemon process is actually running with. workers/timeout are
    // captured raw so the warm-reuse warning can name the exact value
    // the daemon was started with.
    let persisted_workers = workers.map(|n| n as u32);

    // Wait for the child to publish the port it BOUND before recording
    // anything. `daemon.json` used to be written here with the port we asked
    // for, before the child had bound, which made it a prediction: with an
    // ephemeral request there was no number to write at all, and with a fixed
    // one it could disagree with reality if the bind failed.
    let deadline = std::time::Instant::now() + startup_budget();
    let bound = match ServerHandshake::await_published(dir, profile.handshake_slot(), pid, deadline)
        .await
    {
        Ok(port) => port,
        Err(failure) => {
            // Kill it. A child that has not reported a port is still booting,
            // and leaving it running orphans a daemon that will bind moments
            // later: the next invocation adopts it while this one reports no
            // server available, having thrown away the whole boot.
            let reason = failure.reason();
            eprintln!(
                "warning: {} daemon (PID {pid}) {reason}. Check {}",
                profile.label(),
                log_path.display()
            );
            kill_process(pid);
            cleanup_state_file_for(profile, dir);
            return Ok(None);
        }
    };
    let port = bound.get();
    write_daemon_info_for(profile, dir, pid, flags, persisted_workers, timeout)?;

    if wait_for_health_until(pid, port, deadline).await {
        eprintln!(
            "{} daemon ready on port {} (PID {})",
            profile.label(),
            port,
            pid
        );
        return Ok(Some(format!("http://127.0.0.1:{port}")));
    }

    debug!(
        profile = profile.label(),
        port, "Daemon failed to become healthy"
    );
    kill_process(pid);
    cleanup_state_file_for(profile, dir);

    eprintln!(
        "warning: could not start local daemon. Check {}\n\
         hint: run `batchalign3 serve start --foreground` to see startup errors.",
        log_path.display()
    );
    Ok(None)
}

async fn wait_for_health_until(pid: u32, port: u16, deadline: std::time::Instant) -> bool {
    while std::time::Instant::now() < deadline {
        if !is_process_alive(pid) {
            return false;
        }

        if startup_health_check(port).await {
            return true;
        }

        tokio::time::sleep(Duration::from_secs_f64(HEALTH_POLL)).await;
    }

    false
}

/// Quick health probe for localhost daemon startup.
///
/// Uses a short timeout (3s request, 1s connect) instead of the full
/// `BatchalignClient` timeout (120s). A connection to 127.0.0.1 should
/// succeed in well under 1 second, so this gives fast failure detection
/// while still tolerating brief startup latency.
async fn startup_health_check(port: u16) -> bool {
    let client = match reqwest::Client::builder()
        .timeout(Duration::from_secs(3))
        .connect_timeout(Duration::from_secs(1))
        .build()
    {
        Ok(client) => client,
        Err(error) => {
            eprintln!("warning: failed to build localhost health-check client: {error}");
            return false;
        }
    };
    client
        .get(format!("http://127.0.0.1:{port}/health"))
        .send()
        .await
        .is_ok_and(|r| r.status().is_success())
}

/// Full health check via `BatchalignClient` (120s timeout).
/// Used for the reuse path when daemon is already running and we want
/// the richer `HealthResponse`.
async fn health_check(port: u16) -> bool {
    let Ok(client) = BatchalignClient::new() else {
        return false;
    };
    client
        .health_check(&format!("http://127.0.0.1:{port}"))
        .await
        .is_ok()
}

fn read_daemon_info_for(profile: DaemonProfile, dir: &Path) -> Option<DaemonInfo> {
    let path = profile.state_file(dir);
    let text = match std::fs::read_to_string(&path) {
        Ok(text) => text,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return None,
        Err(error) => {
            eprintln!(
                "warning: failed to read daemon state at {}: {}",
                path.display(),
                error
            );
            return None;
        }
    };
    match serde_json::from_str(&text) {
        Ok(info) => Some(info),
        Err(error) => {
            eprintln!(
                "warning: ignoring corrupt daemon state at {}: {}",
                path.display(),
                error
            );
            let _ = std::fs::remove_file(&path);
            None
        }
    }
}

fn write_daemon_info_for(
    profile: DaemonProfile,
    dir: &Path,
    pid: u32,
    flags: DaemonDeviceFlags,
    workers: Option<u32>,
    audio_task_timeout_s: Option<u64>,
) -> Result<(), CliError> {
    let started_at = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|error| {
            CliError::Io(std::io::Error::other(format!(
                "system clock before unix epoch while writing daemon state: {error}"
            )))
        })?
        .as_secs_f64();
    let info = DaemonInfo {
        pid,
        version: current_version(),
        started_at,
        build_hash: crate::cli::build_hash().to_string(),
        force_cpu: flags.resolved_force_cpu,
        allow_mps: flags.resolved_allow_mps,
        workers,
        audio_task_timeout_s,
    };

    std::fs::create_dir_all(dir)?;
    let state_path = profile.state_file(dir);
    let tmp = state_path.with_extension("tmp");
    std::fs::write(&tmp, serde_json::to_string(&info)?)?;
    std::fs::rename(&tmp, &state_path)?;
    Ok(())
}

fn cleanup_state_file_for(profile: DaemonProfile, dir: &Path) {
    let _ = std::fs::remove_file(profile.state_file(dir));
}

fn is_process_alive(pid: u32) -> bool {
    #[cfg(unix)]
    unsafe {
        libc::kill(pid as i32, 0) == 0
    }
    #[cfg(not(unix))]
    {
        let _ = pid;
        false
    }
}

/// Kill a daemon process: SIGTERM the process group, then the process
/// directly, then wait up to 3 seconds and escalate to SIGKILL if still
/// alive. Returns `true` if the process was signalled at all.
fn kill_process(pid: u32) -> bool {
    #[cfg(unix)]
    {
        let pgid_ok = unsafe { libc::killpg(pid as libc::pid_t, libc::SIGTERM) == 0 };
        let pid_ok = unsafe { libc::kill(pid as libc::pid_t, libc::SIGTERM) == 0 };

        if !pgid_ok && !pid_ok {
            return false;
        }

        // Wait for the process to exit before returning so the port is
        // released by the time the caller tries to rebind.
        for _ in 0..6 {
            std::thread::sleep(std::time::Duration::from_millis(500));
            if !is_process_alive(pid) {
                return true;
            }
        }

        // Still alive after 3 seconds -- escalate to SIGKILL.
        debug!(pid, "Process did not exit after SIGTERM, sending SIGKILL");
        unsafe {
            libc::killpg(pid as libc::pid_t, libc::SIGKILL);
            libc::kill(pid as libc::pid_t, libc::SIGKILL);
        }
        // Brief wait for SIGKILL to take effect.
        std::thread::sleep(std::time::Duration::from_millis(200));
        true
    }
    #[cfg(not(unix))]
    {
        let _ = pid;
        false
    }
}

fn current_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// All-off device flags for tests that do not exercise them.
    const NO_DEVICE_FLAGS: DaemonDeviceFlags = DaemonDeviceFlags {
        cli_force_cpu: false,
        cli_allow_mps: false,
        resolved_force_cpu: false,
        resolved_allow_mps: false,
    };

    /// Resolved-only flags helper for mismatch tests.
    const fn resolved_flags(force_cpu: bool, allow_mps: bool) -> DaemonDeviceFlags {
        DaemonDeviceFlags {
            cli_force_cpu: false,
            cli_allow_mps: false,
            resolved_force_cpu: force_cpu,
            resolved_allow_mps: allow_mps,
        }
    }

    /// Build a `DaemonInfo` with all the unrelated fields filled in,
    /// so each test can name only what it actually exercises. Keeps the
    /// individual tests focused on the field they pin.
    fn info_with(
        build_hash: String,
        version: String,
        force_cpu: bool,
        workers: Option<u32>,
        audio_task_timeout_s: Option<u64>,
    ) -> DaemonInfo {
        DaemonInfo {
            pid: 1,
            version,
            started_at: 0.0,
            build_hash,
            force_cpu,
            allow_mps: false,
            workers,
            audio_task_timeout_s,
        }
    }

    #[test]
    fn daemon_info_roundtrip() {
        let info = DaemonInfo {
            pid: 12345,
            version: "1.0.0".to_string(),
            started_at: 1700000000.0,
            build_hash: "1.0.0-abc1234-1700000000".to_string(),
            force_cpu: true,
            allow_mps: false,
            workers: Some(4),
            audio_task_timeout_s: Some(3600),
        };
        let json = serde_json::to_string(&info).unwrap();
        let back: DaemonInfo = serde_json::from_str(&json).unwrap();
        assert_eq!(back.pid, 12345);
        assert_eq!(back.version, "1.0.0");
        assert_eq!(back.build_hash, "1.0.0-abc1234-1700000000");
        assert_eq!(back.workers, Some(4));
        assert_eq!(back.audio_task_timeout_s, Some(3600));
    }

    #[test]
    fn daemon_info_missing_build_hash() {
        // Old state files lack build_hash, should default to empty
        let json = r#"{"pid": 999, "port": 8000, "version": "1.0.0"}"#;
        let info: DaemonInfo = serde_json::from_str(json).unwrap();
        assert_eq!(info.build_hash, "");
        // Old state files also lack workers/audio_task_timeout_s, both
        // must default to None so the flag-shadowing warning falls back
        // to fire-on-any-passing-the-flag (the pre-persisted behavior).
        assert_eq!(info.workers, None);
        assert_eq!(info.audio_task_timeout_s, None);
    }

    #[test]
    fn is_stale_detects_build_hash_mismatch() {
        let info = info_with(
            "old-build-hash".to_string(),
            current_version(),
            false,
            None,
            None,
        );
        // Our build hash is different from "old-build-hash"
        assert!(is_stale(&info));
    }

    #[test]
    fn is_stale_falls_back_to_version_when_no_build_hash() {
        let info = info_with(String::new(), "0.0.0-fake".to_string(), false, None, None);
        assert!(is_stale(&info));

        let info_current = info_with(String::new(), current_version(), false, None, None);
        assert!(!is_stale(&info_current));
    }

    #[test]
    fn is_stale_same_build_hash() {
        let info = info_with(
            crate::cli::build_hash().to_string(),
            current_version(),
            false,
            None,
            None,
        );
        assert!(!is_stale(&info));
    }

    #[test]
    fn read_daemon_info_missing_file() {
        let dir = tempfile::tempdir().unwrap();
        assert!(read_daemon_info_for(DaemonProfile::Main, dir.path()).is_none());
    }

    #[test]
    fn read_daemon_info_malformed_json() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("daemon.json"), "not json at all").unwrap();
        assert!(read_daemon_info_for(DaemonProfile::Main, dir.path()).is_none());
    }

    #[test]
    fn read_daemon_info_missing_version_field() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("daemon.json"),
            r#"{"pid": 999, "port": 8000}"#,
        )
        .unwrap();
        let info = read_daemon_info_for(DaemonProfile::Main, dir.path()).unwrap();
        assert_eq!(info.pid, 999);
        // version defaults to "" via serde(default)
        assert_eq!(info.version, "");
        assert_eq!(info.build_hash, "");
        assert!(!info.force_cpu);
    }

    #[test]
    fn write_read_daemon_info_roundtrip() {
        for profile in [DaemonProfile::Main, DaemonProfile::Sidecar] {
            let dir = tempfile::tempdir().unwrap();
            write_daemon_info_for(
                profile,
                dir.path(),
                42,
                DaemonDeviceFlags {
                    cli_force_cpu: true,
                    cli_allow_mps: false,
                    resolved_force_cpu: true,
                    resolved_allow_mps: false,
                },
                Some(6),
                Some(1800),
            )
            .unwrap();
            let info = read_daemon_info_for(profile, dir.path()).unwrap();
            assert_eq!(info.pid, 42);
            assert_eq!(info.version, current_version());
            assert_eq!(info.build_hash, crate::cli::build_hash());
            assert!(info.force_cpu);
            assert_eq!(info.workers, Some(6));
            assert_eq!(info.audio_task_timeout_s, Some(1800));
        }
    }

    #[test]
    fn write_read_daemon_info_roundtrip_no_workers_no_timeout() {
        // Daemon started without --workers or --timeout (the common
        // server-mode case where host facts pick the parallelism).
        let dir = tempfile::tempdir().unwrap();
        write_daemon_info_for(
            DaemonProfile::Main,
            dir.path(),
            7,
            NO_DEVICE_FLAGS,
            None,
            None,
        )
        .unwrap();
        let info = read_daemon_info_for(DaemonProfile::Main, dir.path()).unwrap();
        assert_eq!(info.workers, None);
        assert_eq!(info.audio_task_timeout_s, None);
    }

    #[test]
    fn cleanup_state_file_removes() {
        let dir = tempfile::tempdir().unwrap();
        write_daemon_info_for(
            DaemonProfile::Main,
            dir.path(),
            1,
            NO_DEVICE_FLAGS,
            None,
            None,
        )
        .unwrap();
        assert!(read_daemon_info_for(DaemonProfile::Main, dir.path()).is_some());
        cleanup_state_file_for(DaemonProfile::Main, dir.path());
        assert!(read_daemon_info_for(DaemonProfile::Main, dir.path()).is_none());
    }

    #[test]
    fn runtime_mismatch_detects_force_cpu_changes() {
        let info = info_with(
            crate::cli::build_hash().to_string(),
            current_version(),
            false,
            None,
            None,
        );
        assert!(runtime_mismatch(&info, resolved_flags(true, false)));
        assert!(!runtime_mismatch(&info, resolved_flags(false, false)));
    }

    #[test]
    fn flag_shadows_daemon_silent_when_user_did_not_pass_flag() {
        // User accepted the daemon's default, never warn, regardless
        // of what the daemon was started with.
        assert!(!flag_shadows_daemon::<u32>(None, None));
        assert!(!flag_shadows_daemon(None, Some(4)));
    }

    #[test]
    fn flag_shadows_daemon_silent_when_values_match() {
        // The whole point of this helper: re-passing a value that
        // already matches the running daemon must NOT warn.
        assert!(!flag_shadows_daemon(Some(4), Some(4)));
        assert!(!flag_shadows_daemon(Some(1800u64), Some(1800u64)));
    }

    #[test]
    fn flag_shadows_daemon_warns_on_value_mismatch() {
        assert!(flag_shadows_daemon(Some(4), Some(1)));
        assert!(flag_shadows_daemon(Some(3600u64), Some(1800u64)));
    }

    #[test]
    fn flag_shadows_daemon_warns_when_persisted_is_unknown() {
        // Pre-upgrade daemon.json files lack the field → persisted is
        // None. The user passed a value, so we cannot prove it matches.
        // Warn (the pre-persisted-value behavior, preserved on first
        // contact post-upgrade).
        assert!(flag_shadows_daemon(Some(4), None::<u32>));
        assert!(flag_shadows_daemon(Some(1800u64), None));
    }

    /// `--force-cpu` always wins over the host-facts recommendation,
    /// regardless of which host the test runs on. The resolved value
    /// must equal `true` whenever the CLI flag is `true`. This pins
    /// the operator-override-wins contract from `EffectiveConfig`.
    #[test]
    fn resolve_force_cpu_for_daemon_cli_override_always_resolves_true() {
        let dir = tempfile::tempdir().unwrap();
        let layout = RuntimeLayout::from_state_dir(dir.path().join("state"));
        std::fs::create_dir_all(layout.state_dir()).unwrap();
        // Empty server.yaml -> ServerConfig::default() -> force_cpu = None.
        // CLI flag = true should still resolve to true on every host.
        let resolved = resolve_device_flags_for_daemon(&layout, true, false);
        assert!(
            resolved.resolved_force_cpu,
            "CLI --force-cpu must resolve to true regardless of host facts"
        );
        assert!(
            !resolved.resolved_allow_mps,
            "allow_mps stays false unless the operator opts in"
        );
    }

    #[test]
    fn cleanup_state_file_idempotent() {
        let dir = tempfile::tempdir().unwrap();
        // Cleaning up a nonexistent file should not panic
        cleanup_state_file_for(DaemonProfile::Main, dir.path());
        cleanup_state_file_for(DaemonProfile::Main, dir.path());
    }

    #[test]
    fn is_process_alive_current_pid() {
        let pid = std::process::id();
        assert!(is_process_alive(pid));
    }

    #[test]
    fn configured_port_request_defaults_to_a_fixed_port() {
        let dir = tempfile::tempdir().unwrap();
        let layout = RuntimeLayout::from_state_dir(dir.path().join("state"));
        let request = configured_port_request(&layout).unwrap();
        assert!(
            request.fixed().is_some(),
            "the default must be a fixed port, so the auto-daemon path works \
             with no server.yaml: got {}",
            request.describe()
        );
    }
}
