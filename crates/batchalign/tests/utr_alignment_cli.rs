//! Binary-boundary tests for offline UTR alignment replay.
//!
//! These invoke the executable an operator uses. No server, model, media, or
//! provider credential is involved.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unreachable,
    clippy::todo,
    clippy::unimplemented
)]

use crate::cli_common;

use cli_common::CliHarness;
use predicates::prelude::*;

const CHAT: &str = "@UTF8\n@Begin\n@Languages:\teng\n@Participants:\tPAR Participant\n@ID:\teng|test|PAR|||||Participant|||\n*PAR:\thello .\n@End\n";
const TOKENS: &str = r#"[{"text":"hello","start_ms":100,"end_ms":300}]"#;

#[test]
fn offline_utr_replay_writes_typed_evidence_without_changing_chat() {
    let harness = CliHarness::new();
    let chat = harness.home_dir().join("input.cha");
    let tokens = harness.home_dir().join("tokens.json");
    let output = harness.home_dir().join("report.json");
    std::fs::write(&chat, CHAT).expect("write CHAT");
    std::fs::write(&tokens, TOKENS).expect("write tokens");

    harness
        .cmd()
        .args(["eval", "utr-alignment"])
        .args(["--chat", chat.to_str().expect("CHAT path")])
        .args(["--tokens", tokens.to_str().expect("token path")])
        .args(["--output", output.to_str().expect("output path")])
        .args(["--fuzzy-threshold", "0.85"])
        .assert()
        .success();

    let report: serde_json::Value = serde_json::from_slice(
        &std::fs::read(&output).expect("the subprocess must publish a report"),
    )
    .expect("report JSON");
    assert_eq!(report["plan"]["utterances"][0]["status"], "matched");
    assert_eq!(
        std::fs::read_to_string(&chat).expect("read unchanged CHAT"),
        CHAT
    );
}

#[test]
fn offline_utr_replay_rejects_invalid_policy_before_writing() {
    let harness = CliHarness::new();
    let chat = harness.home_dir().join("input.cha");
    let tokens = harness.home_dir().join("tokens.json");
    let output = harness.home_dir().join("report.json");
    std::fs::write(&chat, CHAT).expect("write CHAT");
    std::fs::write(&tokens, TOKENS).expect("write tokens");

    harness
        .cmd()
        .args(["eval", "utr-alignment"])
        .args(["--chat", chat.to_str().expect("CHAT path")])
        .args(["--tokens", tokens.to_str().expect("token path")])
        .args(["--output", output.to_str().expect("output path")])
        .args(["--fuzzy-threshold", "1.01"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("between 0 and 1"));

    assert!(
        !output.exists(),
        "invalid CLI state must not create a report"
    );
}

#[test]
fn offline_utr_replay_refuses_to_replace_a_published_report() {
    let harness = CliHarness::new();
    let chat = harness.home_dir().join("input.cha");
    let tokens = harness.home_dir().join("tokens.json");
    let output = harness.home_dir().join("report.json");
    std::fs::write(&chat, CHAT).expect("write CHAT");
    std::fs::write(&tokens, TOKENS).expect("write tokens");
    std::fs::write(&output, "retain this evidence\n").expect("write sentinel");

    harness
        .cmd()
        .args(["eval", "utr-alignment"])
        .args(["--chat", chat.to_str().expect("CHAT path")])
        .args(["--tokens", tokens.to_str().expect("token path")])
        .args(["--output", output.to_str().expect("output path")])
        .assert()
        .failure();

    assert_eq!(
        std::fs::read_to_string(output).expect("read sentinel"),
        "retain this evidence\n"
    );
}
