//! Shared output-finalization for audio-backed commands.
//!
//! `align` and `transcribe` both produce one primary CHAT artifact per file and
//! both optionally run `merge_abbrev` before persisting it. `speaker-identify`
//! is audio-backed in exactly the same way and produces a JSON evidence
//! document instead. Keeping the persistence policy in one place makes the
//! per-file orchestrators easier to test and stops the three drifting.
//!
//! # Why the document and its kind travel together
//!
//! [`FileOutput`] is a sum, not a `String` beside a flag. The task that
//! produced the document is the only thing that knows what kind it is, and
//! pairing "here is some text" with "and by the way write it as CHAT" at a
//! separate call site is a relationship maintained by convention, where the
//! wrong combination type-checks. Carrying them together also makes the
//! nonsense pair unrepresentable: `merge_abbreviations` is a field of the CHAT
//! variant, so there is no way to ask for abbreviation merging on a JSON
//! evidence file and have it silently ignored.

use crate::api::{DisplayPath, ReleasedCommand};
use crate::recipe_runner::materialize::PlannedMaterializedFile;
use crate::recipe_runner::runtime::{
    ChatOutputTarget, primary_output_artifact, write_chat_output_artifact_with_provenance_gate,
    write_text_output_artifact,
};
use crate::store::RunnerFilesystemConfig;

/// Whether a CHAT document has its single-letter abbreviations merged before
/// it is written.
///
/// A sum rather than a `bool` because `write_primary_output_artifact(.., true)`
/// says nothing at a call site about which question `true` answers, and this
/// value used to be the eighth positional argument of a nine-argument shell.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MergeAbbreviations {
    /// Collapse runs of single letters that name a known abbreviation.
    Merge,
    /// Write the document as the pipeline produced it.
    Leave,
}

impl MergeAbbreviations {
    /// Read the caller's `merge_abbrev` option into this vocabulary.
    pub(crate) fn from_option(should_merge: bool) -> Self {
        if should_merge {
            Self::Merge
        } else {
            Self::Leave
        }
    }

    fn merges(self) -> bool {
        matches!(self, Self::Merge)
    }
}

/// What one per-file attempt produced, and therefore how it is persisted.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum FileOutput {
    /// A CHAT document, written through the provenance gate so a re-run that
    /// changes only the `[ba3 ...]` timestamp does not touch the file.
    Chat {
        /// The serialized CHAT text.
        text: String,
        /// Whether to merge abbreviations before writing.
        merge_abbreviations: MergeAbbreviations,
    },
    /// A non-CHAT evidence document, written verbatim.
    ///
    /// Carries no content type of its own: the catalog's `output_policy`
    /// already declares one per command, and a second copy here would be a
    /// value the two could disagree about.
    Evidence {
        /// The serialized document body.
        body: String,
    },
}

/// Apply output-finalization policy before the CHAT file is written.
pub(crate) fn finalize_chat_output(
    chat_text: &str,
    merge_abbreviations: MergeAbbreviations,
) -> String {
    if merge_abbreviations.merges() {
        batchalign_transform::merge_abbreviations_in_chat_text(&crate::chat_parser(), chat_text)
    } else {
        chat_text.to_owned()
    }
}

/// Persist the command's primary CHAT output and return its planned artifact.
///
/// The CHAT half of [`write_primary_output_artifact`], kept as its own function
/// because the provenance gate is a CHAT-specific policy with its own reasons,
/// and because the naming and staging behaviour it pins is what `align` and
/// `transcribe` have always done.
pub(crate) async fn write_primary_chat_output_artifact(
    filesystem: &RunnerFilesystemConfig,
    command: ReleasedCommand,
    file_index: usize,
    source_filename: &str,
    chat_text: &str,
    merge_abbreviations: MergeAbbreviations,
) -> std::io::Result<PlannedMaterializedFile> {
    let final_text = finalize_chat_output(chat_text, merge_abbreviations);
    let primary_output = primary_output_artifact(command, &DisplayPath::from(source_filename));
    let target = ChatOutputTarget::new(filesystem, file_index, &primary_output.display_path);
    write_chat_output_artifact_with_provenance_gate(&target, &final_text, command).await?;
    Ok(primary_output)
}

/// Persist whatever one per-file attempt produced, and return its planned
/// artifact.
///
/// The single writeback seam for every audio-backed command. Both arms derive
/// the output path from the catalog's own `output_policy`, so a command's
/// artifact name is stated once, in the catalog, whichever kind it writes.
pub(crate) async fn write_primary_output_artifact(
    filesystem: &RunnerFilesystemConfig,
    command: ReleasedCommand,
    file_index: usize,
    source_filename: &str,
    output: &FileOutput,
) -> std::io::Result<PlannedMaterializedFile> {
    match output {
        FileOutput::Chat {
            text,
            merge_abbreviations,
        } => {
            write_primary_chat_output_artifact(
                filesystem,
                command,
                file_index,
                source_filename,
                text,
                *merge_abbreviations,
            )
            .await
        }
        FileOutput::Evidence { body } => {
            let primary_output =
                primary_output_artifact(command, &DisplayPath::from(source_filename));
            let target =
                ChatOutputTarget::new(filesystem, file_index, &primary_output.display_path);
            // No provenance gate. That gate exists because re-running a CHAT
            // command rewrites a `[ba3 ...]` timestamp line and produces
            // semantically empty corpus diffs. An evidence document IS a
            // record of one run, so a fresh one differing from the last is
            // the information, not noise.
            write_text_output_artifact(&target, body).await?;
            Ok(primary_output)
        }
    }
}

#[cfg(test)]
mod tests {
    use batchalign_transform::serialize::to_chat_string;

    use super::*;
    use crate::api::ContentType;
    use batchalign_types::paths::{ClientPath, ServerPath};

    fn sample_filesystem(paths_mode: bool) -> RunnerFilesystemConfig {
        RunnerFilesystemConfig {
            paths_mode,
            source_paths: vec![ClientPath::new("/input/test.cha")],
            output_paths: vec![ClientPath::new("/tmp/output/test.cha")],
            before_paths: Vec::new(),
            staging_dir: ServerPath::new("/tmp/staging-audio-output"),
            media_mapping: Default::default(),
            media_subdir: Default::default(),
            source_dir: ClientPath::new("/input"),
        }
    }

    /// Verify that `finalize_chat_output` with `merge_abbrev=true` merges
    /// consecutive single-letter words matching a known abbreviation.
    #[test]
    fn finalize_chat_output_can_merge_abbrev() {
        let chat = "@UTF8\n@Begin\n@Languages:\teng\n@Participants:\tPAR Participant\n@ID:\teng|test|PAR|||||Participant|||\n*PAR:\tF B I do it .\n@End\n";
        let merged = finalize_chat_output(chat, MergeAbbreviations::Merge);
        let parser = crate::chat_parser();
        let (parsed, _) = batchalign_transform::parse::parse_lenient(&parser, &merged);
        let reparsed = to_chat_string(&parsed);
        assert!(
            reparsed.contains("FBI"),
            "merge_abbrev should collapse 'F B I' into 'FBI', got: {reparsed}"
        );
    }

    #[tokio::test]
    async fn write_primary_chat_output_artifact_uses_command_primary_path() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let filesystem = RunnerFilesystemConfig {
            output_paths: vec![ClientPath::new(
                tmp.path().join("requested/test.cha").to_string_lossy(),
            )],
            staging_dir: ServerPath::new(tmp.path().join("staging")),
            ..sample_filesystem(true)
        };
        let chat = "@UTF8\n@Begin\n@Languages:\teng\n@Participants:\tPAR Participant\n@ID:\teng|test|PAR|||||Participant|||\n*PAR:\thello .\n@End\n";

        let artifact = write_primary_chat_output_artifact(
            &filesystem,
            ReleasedCommand::Transcribe,
            0,
            "nested/test.mp3",
            chat,
            MergeAbbreviations::Leave,
        )
        .await
        .expect("write artifact");

        assert_eq!(artifact.content_type, ContentType::Chat);
        assert_eq!(artifact.display_path, DisplayPath::from("nested/test.cha"));
        let written = std::fs::read_to_string(tmp.path().join("requested/test.cha"))
            .expect("read written output");
        assert!(written.contains("*PAR:\thello ."));
    }
}
