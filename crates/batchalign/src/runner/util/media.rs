//! Media resolution, preflight validation, and output path handling.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::api::ReleasedCommand;
use crate::options::CommandOptions;
use crate::store::{PendingJobFile, RunnerJobSnapshot};
use batchalign_types::paths::ClientPath;

use crate::media::MediaExtensions;
use crate::media::probe::MediaProbe;
use tracing::warn;

/// Check if a job should use the legacy untyped Rev.AI preflight submission.
///
/// Disabled for every command: it crossed a paid boundary before the durable
/// evidence cache could prove a miss. A future parallel preflight must carry
/// the same typed miss authorization as per-file Rev inference.
pub(in crate::runner) fn should_preflight(
    _command: ReleasedCommand,
    _typed_options: Option<&CommandOptions>,
) -> bool {
    false
}

/// Pre-validate media files before dispatch.
///
/// For non-CHAT files in paths_mode, checks:
/// 1. File exists on disk
/// 2. File is non-empty
/// 3. File extension is a known audio/video format
///
/// Returns the set of file indices that failed validation.
pub(in crate::runner) async fn preflight_validate_media(
    file_list: &[PendingJobFile],
    source_paths: &[ClientPath],
    paths_mode: bool,
) -> HashMap<usize, String> {
    if !paths_mode {
        return HashMap::new();
    }

    let mut failures = HashMap::new();

    for file in file_list {
        // Only validate non-CHAT (media) files
        if file.has_chat {
            continue;
        }

        let Some(client_path) = source_paths.get(file.file_index) else {
            failures.insert(file.file_index, "No source path for file index".to_string());
            continue;
        };

        // In paths_mode the client and server share a filesystem.
        let path = client_path.assume_shared_filesystem();

        // Check file exists and non-empty via metadata (one syscall)
        match tokio::fs::metadata(&path).await {
            Err(_) => {
                failures.insert(file.file_index, format!("File not found: {}", path));
                continue;
            }
            Ok(meta) if meta.len() == 0 => {
                failures.insert(file.file_index, format!("File is empty: {}", path));
                continue;
            }
            Ok(_) => {}
        }

        // Check known extension
        let ext = path
            .as_path()
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e.to_lowercase());
        if let Some(ref ext) = ext {
            if !MediaExtensions::is_known(ext) {
                failures.insert(
                    file.file_index,
                    format!("Unknown media format '.{ext}': {}", path),
                );
            }
        } else {
            failures.insert(file.file_index, format!("File has no extension: {}", path));
        }
    }

    failures
}

/// Collect the original media paths that should be pre-submitted to Rev.AI for
/// one job.
///
/// The returned paths must be the original provider-visible files, not
/// temporary WAV conversions, because preflight submission happens before the
/// per-file processing pipeline starts.
pub(in crate::runner) async fn collect_preflight_audio_paths(
    command: ReleasedCommand,
    job: &RunnerJobSnapshot,
    file_list: &[PendingJobFile],
) -> Vec<PathBuf> {
    match command {
        ReleasedCommand::Align => collect_align_preflight_audio_paths(job, file_list).await,
        _ => file_list
            .iter()
            .filter(|file| !file.has_chat)
            .filter_map(|file| {
                if job.filesystem.paths_mode && file.file_index < job.filesystem.source_paths.len()
                {
                    Some(
                        job.filesystem.source_paths[file.file_index]
                            .assume_shared_filesystem()
                            .as_path()
                            .to_owned(),
                    )
                } else {
                    None
                }
            })
            .collect(),
    }
}

/// Collect align-job media paths for Rev.AI preflight.
///
/// Align jobs usually begin from CHAT files, so preflight must resolve the
/// sibling media path first. This helper currently supports the local
/// `paths_mode` shape, which is where Rev.AI preflight provides the main
/// throughput win for large corpora.
async fn collect_align_preflight_audio_paths(
    job: &RunnerJobSnapshot,
    file_list: &[PendingJobFile],
) -> Vec<PathBuf> {
    if !job.filesystem.paths_mode {
        return Vec::new();
    }

    let mut paths = Vec::new();
    for file in file_list {
        let Some(client_path) = job.filesystem.source_paths.get(file.file_index) else {
            continue;
        };
        let server_path = client_path.assume_shared_filesystem();
        if let Some(audio_path) = resolve_audio_for_chat(server_path.as_path()).await {
            paths.push(audio_path);
        }
    }
    paths
}

/// The audio file for a CHAT file sitting beside it.
///
/// Kept because preflight still calls it; the `_with_media_dir` variant beside
/// it is gone, because the FA pipeline's rungs became `MediaSearch` places and
/// nothing else wanted the two-directory form. This is now a single call to the
/// verb rather than its own extension loop.
pub(in crate::runner) async fn resolve_audio_for_chat(chat_path: &Path) -> Option<PathBuf> {
    let stem = chat_path.file_stem()?.to_str()?;
    MediaExtensions::find_in(chat_path.parent()?, stem).await
}

/// Compute audio identity for cache keying.
///
/// Returns an [`AudioIdentity`] built from the file's resolved path,
/// modification time, and size.
pub(in crate::runner) async fn compute_audio_identity(
    audio_path: &str,
) -> Option<crate::chat_ops::fa::AudioIdentity> {
    let meta = tokio::fs::metadata(audio_path).await.ok()?;
    let size = meta.len();
    let mtime = meta
        .modified()
        .ok()?
        .duration_since(std::time::UNIX_EPOCH)
        .ok()?
        .as_secs();
    Some(crate::chat_ops::fa::AudioIdentity::from_metadata(
        audio_path, mtime, size,
    ))
}

/// Get audio duration in milliseconds via ffprobe.
///
/// `None` is a DOWNGRADE the caller asks for, not an erasure: the duration is
/// optional for untimed-utterance estimation, so a failure is survivable, but
/// the reason is logged before it is dropped. This function used to return
/// `Option` with no logging, folding "ffprobe is not installed", "ffprobe was
/// killed", "ffprobe refused the file" and "ffprobe printed nonsense" into one
/// silent `None`, so an operator could not tell a missing dependency from a
/// corrupt recording.
pub(in crate::runner) async fn get_audio_duration_ms(audio_path: &str) -> Option<u64> {
    match MediaProbe::new(audio_path).duration().await {
        Ok(duration) => Some(duration.0),
        Err(error) => {
            warn!(
                audio = %audio_path,
                error = %error,
                "Could not determine audio duration; untimed utterances will be estimated without it"
            );
            None
        }
    }
}

/// Replace the filename in `output_path` with `result_filename`.
#[cfg(test)]
pub(in crate::runner) fn apply_result_filename(
    output_path: &Path,
    result_filename: &str,
) -> PathBuf {
    let result_name = Path::new(result_filename).file_name().unwrap_or_default();
    output_path
        .parent()
        .map(|p| p.join(result_name))
        .unwrap_or_else(|| result_name.into())
}

#[cfg(test)]
mod tests {
    use super::should_preflight;
    use crate::api::ReleasedCommand;
    use crate::options::{
        AsrEngineName, BenchmarkOptions, CommandOptions, CommonOptions, TranscribeOptions,
    };

    #[test]
    fn transcribe_asr_override_disables_rev_preflight() {
        let mut common = CommonOptions::default();
        common.engine_overrides.asr = Some(AsrEngineName::HkTencent);
        let opts = CommandOptions::Transcribe(TranscribeOptions {
            common,
            asr_engine: AsrEngineName::RevAi,
            diarize: false,
            wor: false.into(),
            merge_abbrev: false.into(),
            batch_size: 8,
            utseg_fallback: false.into(),
        });

        assert!(!should_preflight(ReleasedCommand::Transcribe, Some(&opts)));
    }

    #[test]
    fn benchmark_asr_override_disables_rev_preflight() {
        let mut common = CommonOptions::default();
        common.engine_overrides.asr = Some(AsrEngineName::HkAliyun);
        let opts = CommandOptions::Benchmark(BenchmarkOptions {
            common,
            asr_engine: AsrEngineName::RevAi,
            wor: true.into(),
            merge_abbrev: false.into(),
        });

        assert!(!should_preflight(ReleasedCommand::Benchmark, Some(&opts)));
    }
}
