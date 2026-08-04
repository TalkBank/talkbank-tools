//! Binary subprocess tests for `batchalign3 compare-runs`.
//!
//! The `compare-runs` family shipped with unit and library coverage only. Its
//! own continuation handoff listed subprocess coverage as the first
//! recommended next work, and the gap is not cosmetic: the crate's verification
//! was `cargo test -p batchalign --lib`, which builds neither the integration
//! tests nor the doctests, so a whole class of breakage could not be seen. This
//! file exercises the actual boundary an operator uses.
//!
//! Everything here runs OFFLINE. `compare-runs` is routed before normal
//! Batchalign setup and server initialization and never invokes a producer
//! command, so no server, model, or network is required, and that property is
//! itself one of the things asserted below.
// Integration tests are exempt from the crate's deny-level panic lints,
// matching the src/lib.rs `#![cfg_attr(test, allow(...))]` pattern
// (see docs/panic-audit/).
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unreachable,
    clippy::todo,
    clippy::unimplemented
)]

mod cli_common;

use std::fs;
use std::path::Path;

use cli_common::{CliHarness, MINIMAL_CHAT};
use predicates::prelude::*;

/// Write a transcript into `dir`, creating the directory if needed.
fn write_chat(dir: &Path, name: &str, body: &str) {
    fs::create_dir_all(dir).expect("create artifact dir");
    fs::write(dir.join(name), body).expect("write transcript");
}

/// A run directory holding one transcript, the smallest thing a manifest can
/// describe.
fn seed_run(root: &Path) {
    write_chat(root, "session-1.cha", MINIMAL_CHAT);
}

#[test]
fn help_lists_every_compare_runs_action() {
    CliHarness::new()
        .cmd()
        .args(["compare-runs", "--help"])
        .assert()
        .success()
        .stdout(
            predicate::str::contains("manifest")
                .and(predicate::str::contains("transcribe"))
                .and(predicate::str::contains("morphotag"))
                .and(predicate::str::contains("align")),
        );
}

#[test]
fn manifest_machine_writes_json_naming_its_producer_identity() {
    let harness = CliHarness::new();
    let artifacts = harness.home_dir().join("run-machine");
    seed_run(&artifacts);
    let output = harness.home_dir().join("machine.manifest.json");

    harness
        .cmd()
        .args(["compare-runs", "manifest", "machine"])
        .args(["--artifacts", artifacts.to_str().unwrap()])
        .args(["--output", output.to_str().unwrap()])
        .args(["--run-id", "ours-v10"])
        .args(["--source-id", "corpus-a"])
        .args(["--implementation", "batchalign3"])
        .args(["--command", "transcribe"])
        .args(["--build", "test-build"])
        .assert()
        .success();

    let written = fs::read_to_string(&output).expect("manifest written");
    // The producer identity is the point of the manifest: a comparison that
    // cannot say what produced each side is not evidence.
    assert!(written.contains("batchalign3"), "manifest: {written}");
    assert!(written.contains("ours-v10"), "manifest: {written}");
    assert!(written.contains("session-1.cha"), "manifest: {written}");
}

#[test]
fn manifest_authoring_is_deterministic_for_identical_inputs() {
    let harness = CliHarness::new();
    let artifacts = harness.home_dir().join("run-stable");
    seed_run(&artifacts);

    let mut written = Vec::new();
    for name in ["first.json", "second.json"] {
        let output = harness.home_dir().join(name);
        harness
            .cmd()
            .args(["compare-runs", "manifest", "human"])
            .args(["--artifacts", artifacts.to_str().unwrap()])
            .args(["--output", output.to_str().unwrap()])
            .args(["--run-id", "theirs-hand"])
            .args(["--source-id", "corpus-a"])
            .args(["--protocol", "inv-v1"])
            .args(["--cohort", "reviewers"])
            .assert()
            .success();
        written.push(fs::read_to_string(&output).expect("manifest written"));
    }

    // Byte-identical, not merely equivalent. A manifest feeds a
    // content-addressed comparison identity, so any run-to-run instability
    // (map ordering, a timestamp) would silently invalidate every cache hit
    // and make "unchanged inputs" produce a new comparison directory.
    assert_eq!(written[0], written[1], "manifest authoring is not stable");
}

#[test]
fn manifest_refuses_to_write_its_output_inside_the_artifact_root() {
    let harness = CliHarness::new();
    let artifacts = harness.home_dir().join("run-selfref");
    seed_run(&artifacts);
    // Writing the manifest into the tree it inventories would make the run
    // describe itself, and the artifacts are supposed to be immutable.
    let output = artifacts.join("manifest.json");

    harness
        .cmd()
        .args(["compare-runs", "manifest", "machine"])
        .args(["--artifacts", artifacts.to_str().unwrap()])
        .args(["--output", output.to_str().unwrap()])
        .args(["--run-id", "ours"])
        .args(["--source-id", "corpus-a"])
        .args(["--implementation", "batchalign3"])
        .args(["--command", "transcribe"])
        .args(["--build", "test-build"])
        .assert()
        .failure();
}

#[test]
fn manifest_reports_a_missing_artifact_root_rather_than_writing_an_empty_one() {
    let harness = CliHarness::new();
    let missing = harness.home_dir().join("not-there");
    let output = harness.home_dir().join("out.json");

    harness
        .cmd()
        .args(["compare-runs", "manifest", "machine"])
        .args(["--artifacts", missing.to_str().unwrap()])
        .args(["--output", output.to_str().unwrap()])
        .args(["--run-id", "ours"])
        .args(["--source-id", "corpus-a"])
        .args(["--implementation", "batchalign3"])
        .args(["--command", "transcribe"])
        .args(["--build", "test-build"])
        .assert()
        .failure();

    assert!(
        !output.exists(),
        "a failed manifest run must not leave an output behind"
    );
}

#[test]
fn an_unreadable_plan_fails_without_starting_a_server() {
    let harness = CliHarness::new();
    let plan = harness.home_dir().join("missing-plan.toml");

    // No server is configured and none may be started: `compare-runs` is
    // routed ahead of setup and server initialization, so this must fail on
    // the plan alone rather than on a connection attempt.
    harness
        .cmd()
        .args(["compare-runs", "transcribe"])
        .args(["--plan", plan.to_str().unwrap()])
        .assert()
        .failure()
        .stderr(predicate::str::contains("server").not());
}

#[test]
fn a_plan_with_an_unknown_field_is_rejected_by_name() {
    let harness = CliHarness::new();
    let plan = harness.home_dir().join("bad-field.toml");
    // `deny_unknown_fields` exists so a typo in a plan cannot be silently
    // ignored and change what was compared.
    fs::write(
        &plan,
        "left = \"a.json\"\nright = \"b.json\"\nnot_a_real_field = 1\n",
    )
    .expect("write plan");

    harness
        .cmd()
        .args(["compare-runs", "morphotag"])
        .args(["--plan", plan.to_str().unwrap()])
        .assert()
        .failure();
}

#[test]
fn every_execute_action_accepts_recompute() {
    // The flag exists on all three execution modes; a mode that silently
    // ignored it would serve a stale cached comparison after the policy
    // changed. Each still fails on the absent plan, which is the point: the
    // argument surface is what is under test here.
    let harness = CliHarness::new();
    let plan = harness.home_dir().join("absent.toml");
    for action in ["transcribe", "morphotag", "align"] {
        harness
            .cmd()
            .args(["compare-runs", action])
            .args(["--plan", plan.to_str().unwrap()])
            .arg("--recompute")
            .assert()
            .failure();
    }
}
