use crate::common::{
    LiveDirectSession, require_live_direct_warmed, require_live_direct_warmed_many,
};
use batchalign::api::{FilePayload, FileResult, JobInfo, ReleasedCommand};
use batchalign::chat_ops::{ChatFile, DependentTier};
use batchalign::options::CommandOptions;
use batchalign::worker::InferTask;
use batchalign_transform::parse::{TreeSitterParser, parse_lenient};
use std::sync::atomic::{AtomicU64, Ordering};

static LIVE_NAME_COUNTER: AtomicU64 = AtomicU64::new(0);

pub(crate) struct DirectGoldenSession {
    session: LiveDirectSession,
}

impl DirectGoldenSession {
    pub(crate) async fn submit_content_job(
        &self,
        command: ReleasedCommand,
        lang: &str,
        filename: &str,
        content: &str,
        options: CommandOptions,
    ) -> (JobInfo, Vec<FileResult>) {
        crate::common::submit_and_complete_direct(
            &self.session,
            command,
            lang,
            vec![FilePayload {
                filename: filename.into(),
                content: content.into(),
            }],
            options,
        )
        .await
    }

    pub(crate) async fn submit_files_job(
        &self,
        command: ReleasedCommand,
        lang: &str,
        files: Vec<FilePayload>,
        options: CommandOptions,
    ) -> (JobInfo, Vec<FileResult>) {
        crate::common::submit_and_complete_direct(&self.session, command, lang, files, options)
            .await
    }
}

/// Acquire a warmed live session, or FAIL LOUDLY.
///
/// This deliberately panics rather than skipping. Building this crate with
/// `--features ml-golden` is an explicit request to run the ML golden suite,
/// so a session that cannot be acquired means the request could not be
/// honoured, and reporting `ok` for a test that never executed is a false
/// green of exactly the kind this suite exists to prevent.
///
/// Found 2026-07-28: two newly added Italian golden tests reported `ok` in
/// 7.23s having produced no output at all. Only a mutation (replacing an
/// assertion with a deliberate lie and watching it still pass) would have
/// distinguished "passed" from "never ran". The suite had also been
/// unreachable for some time, with no Makefile or CI entry point and only
/// nextest's retired `--profile ml` to invoke it, so nobody noticed.
///
/// If the environment genuinely cannot host these tests (no Python worker, no
/// model weights, no credentials), do not run the suite: it is feature-gated
/// precisely so a plain `cargo test` never reaches it.
pub(crate) async fn require_direct_session_warmed(
    task: InferTask,
    command: ReleasedCommand,
    lang: &str,
    skip_message: &str,
) -> Option<DirectGoldenSession> {
    let session = require_live_direct_warmed(task, command, lang, skip_message)
        .await
        .unwrap_or_else(|| {
            panic!(
                "ml-golden requested but no live session for {command:?}/{lang} \
                 ({task:?}): {skip_message}. The suite is feature-gated, so \
                 reaching here means the environment cannot honour an explicit \
                 request; a silent skip would report a pass for a test that \
                 never ran."
            )
        });
    Some(DirectGoldenSession { session })
}

pub(crate) async fn require_direct_session_warmed_many(
    task: InferTask,
    warmups: Vec<(ReleasedCommand, &str)>,
    skip_message: &str,
) -> Option<DirectGoldenSession> {
    let session = require_live_direct_warmed_many(task, warmups, skip_message).await?;
    Some(DirectGoldenSession { session })
}

pub(crate) fn unique_test_dir(prefix: &str) -> String {
    let counter = LIVE_NAME_COUNTER.fetch_add(1, Ordering::Relaxed);
    format!("{prefix}_{counter}")
}

pub(crate) fn parse_output(chat: &str, label: &str) -> ChatFile {
    let parser = TreeSitterParser::new().unwrap();
    let (file, errors) = parse_lenient(&parser, chat);
    assert!(errors.is_empty(), "{label}: CHAT parse errors: {errors:?}");
    file
}

pub(crate) fn has_mor_tier(file: &ChatFile) -> bool {
    file.lines.iter().any(|line| {
        if let batchalign::chat_ops::Line::Utterance(utt) = line {
            utt.dependent_tiers
                .iter()
                .any(|t| matches!(t.tier, DependentTier::Mor(_)))
        } else {
            false
        }
    })
}

pub(crate) fn has_gra_tier(file: &ChatFile) -> bool {
    file.lines.iter().any(|line| {
        if let batchalign::chat_ops::Line::Utterance(utt) = line {
            utt.dependent_tiers
                .iter()
                .any(|t| matches!(t.tier, DependentTier::Gra(_)))
        } else {
            false
        }
    })
}

pub(crate) fn has_user_defined_tier(file: &ChatFile, label: &str) -> bool {
    file.lines.iter().any(|line| {
        if let batchalign::chat_ops::Line::Utterance(utt) = line {
            utt.dependent_tiers.iter().any(|t| match &t.tier {
                DependentTier::UserDefined(ud) => ud.label.as_ref() == label,
                _ => false,
            })
        } else {
            false
        }
    })
}

pub(crate) fn find_mor_line_for(chat: &str, at_s_text: &str) -> Option<String> {
    let lines: Vec<&str> = chat.lines().collect();
    for (i, line) in lines.iter().enumerate() {
        if line.contains(at_s_text)
            && line.starts_with('*')
            && i + 1 < lines.len()
            && lines[i + 1].starts_with("%mor:")
        {
            return Some(lines[i + 1].trim_start_matches("%mor:\t").to_string());
        }
    }
    None
}

macro_rules! assert_golden_snapshot {
    ($name:expr, $value:expr) => {
        insta::with_settings!({snapshot_path => "../snapshots"}, {
            insta::assert_snapshot!($name, $value);
        });
    };
}

pub(crate) use assert_golden_snapshot;
