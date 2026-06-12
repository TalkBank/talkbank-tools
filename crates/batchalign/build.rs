// Build scripts run at build time, not runtime. Panics here fail
// `cargo build`, which is the intended behaviour for missing files
// or invalid embedded data.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

//! Build script — generates `BUILD_HASH` at compile time.
//!
//! The hash changes on every rebuild (even when `CARGO_PKG_VERSION` stays
//! the same), enabling stale-binary detection during development.

fn main() {
    let version = env!("CARGO_PKG_VERSION");

    // Best-effort git describe — empty string if git is not available.
    let git_describe = std::process::Command::new("git")
        .args(["describe", "--always", "--dirty"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_default();

    let epoch = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let build_hash = if git_describe.is_empty() {
        format!("{version}-{epoch}")
    } else {
        format!("{version}-{git_describe}-{epoch}")
    };

    println!("cargo:rustc-env=BUILD_HASH={build_hash}");

    // Rebuild when git state changes (commit, stage, checkout). HEAD only
    // contains "ref: refs/heads/main" on a branch, so also watch the resolved
    // branch ref file or same-branch commits can reuse stale BUILD_HASH values.
    for path in git_state_paths() {
        println!("cargo:rerun-if-changed={path}");
    }
}

fn git_state_paths() -> Vec<String> {
    let mut paths = vec![
        "../../.git/HEAD".to_string(),
        "../../.git/index".to_string(),
    ];

    let head_ref = std::process::Command::new("git")
        .args(["symbolic-ref", "-q", "HEAD"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_default();

    if !head_ref.is_empty() {
        if let Some(path) = git_path(&head_ref) {
            paths.push(path);
        }
    }

    paths
}

fn git_path(path: &str) -> Option<String> {
    std::process::Command::new("git")
        .args(["rev-parse", "--git-path", path])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .filter(|s| !s.is_empty())
}
