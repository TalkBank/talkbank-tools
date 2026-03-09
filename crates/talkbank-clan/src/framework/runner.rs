//! [`AnalysisRunner`] -- file loading, filtering, and command dispatch.
//!
//! The runner is the orchestrator that loads CHAT files, applies
//! [`FilterConfig`] criteria, and feeds matching utterances to the command's
//! [`process_utterance()`](super::AnalysisCommand::process_utterance) method.
//! It replaces CUTT's main loop from the original CLAN C framework.
//!
//! Two execution modes are supported:
//! - [`run()`](AnalysisRunner::run) -- aggregated mode: all files share one
//!   state, producing a single combined output
//! - [`run_per_file()`](AnalysisRunner::run_per_file) -- per-file mode: each
//!   file gets its own fresh state and produces independent output
//!
//! Parse/validation failures for individual files are logged as warnings and
//! skipped (non-fatal), following CLAN's behavior of continuing through file
//! errors.

use std::path::PathBuf;

use talkbank_model::ParseValidateOptions;
use talkbank_transform::{PipelineError, parse_file_and_validate};
use tracing::{debug, warn};

use super::command::{AnalysisCommand, FileContext};
use super::filter::{FilterConfig, update_active_gems};

/// Error type for analysis runner operations.
#[derive(Debug, thiserror::Error)]
pub enum RunnerError {
    /// Failed to parse or validate a CHAT file.
    #[error("Failed to process {path}: {source}")]
    Pipeline {
        /// Path to the file that failed.
        path: PathBuf,
        /// The underlying pipeline error.
        source: PipelineError,
    },
    /// No input files were provided.
    #[error("No input files provided")]
    NoFiles,
}

/// Orchestrates file loading, filtering, and command execution.
///
/// The runner handles:
/// 1. Loading and parsing CHAT files via talkbank-transform
/// 2. Tracking @BG/@EG gem boundaries
/// 3. Applying filter criteria (speakers, gems, etc.)
/// 4. Dispatching matching utterances to the command
///
/// # Example
///
/// ```no_run
/// use std::path::PathBuf;
/// use talkbank_clan::framework::{AnalysisRunner, FilterConfig};
/// use talkbank_clan::commands::freq::FreqCommand;
///
/// let runner = AnalysisRunner::with_filter(FilterConfig::default());
/// let command = FreqCommand::default();
/// let result = runner.run(&command, &[PathBuf::from("file.cha")]);
/// ```
pub struct AnalysisRunner {
    /// Filter criteria applied to each utterance
    filter: FilterConfig,
}

impl AnalysisRunner {
    /// Create a runner with default (no-op) filtering.
    pub fn new() -> Self {
        Self {
            filter: FilterConfig::default(),
        }
    }

    /// Create a runner with the given filter configuration.
    pub fn with_filter(filter: FilterConfig) -> Self {
        Self { filter }
    }

    /// Run an analysis command across the given files, aggregating results.
    ///
    /// # Lifecycle
    ///
    /// For each file:
    ///   1. Parse and validate the file
    ///   2. For each utterance, update gem tracking from preceding headers
    ///   3. Apply filter criteria (speaker, gem, word, utterance range)
    ///   4. Call `command.process_utterance()` for matching utterances
    ///   5. Call `command.end_file()` after all utterances
    ///
    /// After all files: call `command.finalize()` to produce output.
    ///
    /// # Errors
    ///
    /// Returns `RunnerError::NoFiles` if no files are provided.
    /// Parse/validation failures for individual files are logged as warnings
    /// and skipped (non-fatal), following CLAN's behavior of continuing
    /// through file errors.
    pub fn run<C: AnalysisCommand>(
        &self,
        command: &C,
        files: &[PathBuf],
    ) -> Result<C::Output, RunnerError> {
        if files.is_empty() {
            return Err(RunnerError::NoFiles);
        }

        let mut state = C::State::default();
        self.process_files(command, files, &mut state);
        Ok(command.finalize(state))
    }

    /// Run an analysis command per file, returning separate results for each.
    ///
    /// Each file gets its own fresh `State`, processed independently, and
    /// finalized into its own `Output`. This corresponds to CLAN's per-file
    /// output mode.
    ///
    /// # Errors
    ///
    /// Returns `RunnerError::NoFiles` if no files are provided.
    pub fn run_per_file<C: AnalysisCommand>(
        &self,
        command: &C,
        files: &[PathBuf],
    ) -> Result<Vec<(PathBuf, C::Output)>, RunnerError> {
        if files.is_empty() {
            return Err(RunnerError::NoFiles);
        }

        let mut results = Vec::new();
        for path in files {
            let mut state = C::State::default();
            self.process_files(command, std::slice::from_ref(path), &mut state);
            results.push((path.clone(), command.finalize(state)));
        }
        Ok(results)
    }

    /// Process files into accumulator state, applying filters and dispatching
    /// matching utterances to the command.
    fn process_files<C: AnalysisCommand>(
        &self,
        command: &C,
        files: &[PathBuf],
        state: &mut C::State,
    ) {
        let options = ParseValidateOptions::default().with_validation();

        for path in files {
            debug!(path = %path.display(), "Processing file");

            let chat_file = match parse_file_and_validate(path, options.clone()) {
                Ok(f) => f,
                Err(e) => {
                    warn!(path = %path.display(), error = %e, "Skipping file due to parse error");
                    continue;
                }
            };

            let filename = path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("unknown");

            let file_ctx = FileContext {
                path,
                chat_file: &chat_file,
                filename,
                line_map: chat_file.line_map.as_ref(),
            };

            // Track @BG/@EG boundaries and utterance index per file
            let mut active_gems: Vec<String> = Vec::new();
            let mut utterance_index: usize = 0;

            for utterance in chat_file.utterances() {
                // Update gem tracking from preceding headers
                update_active_gems(&utterance.preceding_headers, &mut active_gems);

                // 1-based utterance index for range filtering
                utterance_index += 1;

                // Apply filters (speaker, gem, word, range)
                if !self
                    .filter
                    .matches(utterance, &active_gems, utterance_index)
                {
                    continue;
                }

                command.process_utterance(utterance, &file_ctx, state);
            }

            command.end_file(&file_ctx, state);
        }
    }
}

impl Default for AnalysisRunner {
    /// Construct a runner with pass-through filtering (`FilterConfig::default()`).
    fn default() -> Self {
        Self::new()
    }
}
