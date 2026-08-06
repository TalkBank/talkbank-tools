use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::api::RevAiJobId;
use crate::error::ServerError;

pub(crate) fn resolve_paths_mode_or_staging_input(
    filesystem: &crate::store::RunnerFilesystemConfig,
    file_index: usize,
    filename: &str,
) -> PathBuf {
    if filesystem.paths_mode && file_index < filesystem.source_paths.len() {
        filesystem.source_paths[file_index]
            .assume_shared_filesystem()
            .as_path()
            .to_owned()
    } else {
        filesystem
            .staging_dir
            .as_path()
            .join("input")
            .join(filename)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PreparedAsrMediaInput {
    pub original_audio_path: PathBuf,
    pub inference_audio_path: PathBuf,
    pub media_name: Option<String>,
    pub rev_job_id: Option<RevAiJobId>,
}

pub(crate) fn preserved_media_name_for_chat(
    original_audio_path: &Path,
    inference_audio_path: &Path,
) -> Option<String> {
    original_audio_path
        .file_stem()
        .or_else(|| original_audio_path.file_name())
        .or_else(|| inference_audio_path.file_stem())
        .or_else(|| inference_audio_path.file_name())
        .map(|s| s.to_string_lossy().into_owned())
}

pub(crate) async fn prepare_asr_media_input(
    original_audio_path: PathBuf,
    rev_job_ids: &HashMap<PathBuf, RevAiJobId>,
    media_name: Option<String>,
    context_label: &str,
) -> Result<PreparedAsrMediaInput, ServerError> {
    // Reject an unusable media name HERE, before the ASR call is made.
    //
    // `@Media` delimits the filename with a comma, so a stem like
    // `interview,part2` cannot be written to a header. The failure used to
    // surface at CHAT serialization, AFTER transcription had been paid for,
    // which is the same late-failure pattern that made this defect expensive
    // in the field. Nothing here is recoverable by the pipeline: the operator
    // renames the file. So the cheapest possible moment to say so is now,
    // while the only thing spent is a path lookup.
    if let Some(name) = media_name.as_deref()
        && let Err(error) = talkbank_model::model::MediaFilename::parse(name)
    {
        return Err(ServerError::Validation(format!(
            "Media name {name:?} for {context_label} cannot be written to an @Media header \
             ({error}); rename the media file and resubmit"
        )));
    }

    let inference_audio_path = crate::ensure_wav::ensure_wav(&original_audio_path, None)
        .await
        .map_err(|error| {
            ServerError::Validation(format!(
                "Media conversion failed for {context_label}: {error}"
            ))
        })?;

    Ok(PreparedAsrMediaInput {
        rev_job_id: rev_job_ids.get(&original_audio_path).cloned(),
        original_audio_path,
        inference_audio_path,
        media_name,
    })
}

#[cfg(test)]
mod tests {
    use super::{preserved_media_name_for_chat, resolve_paths_mode_or_staging_input};
    use crate::store::RunnerFilesystemConfig;

    /// An unusable media name is rejected BEFORE ASR runs, not at CHAT
    /// serialization afterwards.
    ///
    /// This is the whole point of the check's placement: the operator's fix is
    /// to rename the file, and learning that after paying for transcription is
    /// the expensive version of the same message.
    #[tokio::test]
    async fn an_unusable_media_name_is_rejected_before_asr() {
        // `tempfile`'s RAII guard, not a hand-rolled directory: the manual
        // version cleaned up AFTER the assertions, so a failing run leaked a
        // directory every time, which is the run you hit while iterating.
        let dir = tempfile::tempdir().expect("tempdir");
        let audio = dir.path().join("interview,part2.wav");
        std::fs::write(&audio, b"not really audio").expect("write");

        let err = super::prepare_asr_media_input(
            audio.clone(),
            &std::collections::HashMap::new(),
            Some("interview,part2".to_string()),
            "test job",
        )
        .await
        .expect_err("a comma-bearing media name must be rejected up front");

        let message = err.to_string();
        assert!(
            message.contains("@Media") && message.contains("rename"),
            "the error must say what is wrong and what to do: {message}"
        );
    }

    use std::path::PathBuf;

    #[test]
    fn resolve_paths_mode_or_staging_input_prefers_explicit_source_path() {
        let filesystem = RunnerFilesystemConfig {
            paths_mode: true,
            source_paths: vec![batchalign_types::paths::ClientPath::new(
                "/shared/in/clip.mp3".to_string(),
            )],
            output_paths: vec![],
            before_paths: vec![],
            staging_dir: batchalign_types::paths::ServerPath::new("/tmp/staging"),
            media_mapping: Default::default(),
            media_subdir: Default::default(),
            source_dir: Default::default(),
        };

        let path = resolve_paths_mode_or_staging_input(&filesystem, 0, "ignored.mp3");
        assert_eq!(path, PathBuf::from("/shared/in/clip.mp3"));
    }

    #[test]
    fn resolve_paths_mode_or_staging_input_falls_back_to_staging_input() {
        let filesystem = RunnerFilesystemConfig {
            paths_mode: false,
            source_paths: vec![],
            output_paths: vec![],
            before_paths: vec![],
            staging_dir: batchalign_types::paths::ServerPath::new("/tmp/staging"),
            media_mapping: Default::default(),
            media_subdir: Default::default(),
            source_dir: Default::default(),
        };

        let path = resolve_paths_mode_or_staging_input(&filesystem, 0, "clip.mp3");
        assert_eq!(path, PathBuf::from("/tmp/staging/input/clip.mp3"));
    }

    #[test]
    fn preserved_media_name_prefers_original_basename() {
        let original = PathBuf::from("/corpus/interview.mp4");
        let inference = PathBuf::from("/cache/worker/interview.wav");
        assert_eq!(
            preserved_media_name_for_chat(&original, &inference).as_deref(),
            Some("interview")
        );
    }
}
