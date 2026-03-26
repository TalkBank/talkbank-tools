//! Validation commands for CHAT files.
//!
//! This module exposes the low-level `validate_file` entrypoint plus formatting helpers
//! and utilities (audit reporting, output formatting). It is the landing point for CLI `validate`
//! subcommands (single file, directory, TUI) and orchestrates caching, alignment toggles, and
//! structured outputs.
//!
//! # Related CHAT Manual Sections
//!
//! - <https://talkbank.org/0info/manuals/CHAT.html#File_Format>
//! - <https://talkbank.org/0info/manuals/CHAT.html#File_Headers>
//! - <https://talkbank.org/0info/manuals/CHAT.html#Main_Tier>
//! - <https://talkbank.org/0info/manuals/CHAT.html#Dependent_Tiers>

pub mod audit_reporter;
pub(crate) mod cache;
mod file;
mod output;

use std::path::PathBuf;

use crate::cli::OutputFormat;
use crate::ui::Theme;
use talkbank_transform::validation_runner::ParserKind;

use super::validate_parallel::{
    AlignmentValidationMode, CacheRefreshMode, RoundtripValidationMode, StreamingValidationOutput,
    ValidateDirectoryOptions, ValidationExecution, ValidationInterface, ValidationPresentation,
    ValidationRules, ValidationTraversalMode, validate_directory_parallel,
};

pub use file::validate_file;

/// Typed options for the top-level `chatter validate` command.
#[derive(Clone, Debug)]
pub struct ValidateCommandRules {
    /// Alignment validation policy.
    pub alignment: AlignmentValidationMode,
    /// Roundtrip validation policy.
    pub roundtrip: RoundtripValidationMode,
    /// Parser backend selection.
    pub parser_kind: ParserKind,
}

/// Execution settings for the top-level `chatter validate` command.
#[derive(Clone, Debug)]
pub struct ValidateCommandExecution {
    /// Cache refresh policy for the target path.
    pub cache_refresh: CacheRefreshMode,
    /// Optional parallel worker count.
    pub jobs: Option<usize>,
    /// Optional global error cap for directory validation.
    pub max_errors: Option<usize>,
}

/// Output and interaction settings for the top-level `chatter validate` command.
#[derive(Clone, Debug)]
pub struct ValidateCommandPresentation {
    /// Output format for file or directory validation.
    pub format: OutputFormat,
    /// Whether to suppress success output.
    pub quiet: bool,
    /// Optional audit JSONL output path.
    pub audit_output: Option<PathBuf>,
    /// Interactive presentation surface to use.
    pub interface: ValidationInterface,
    /// Loaded theme for TUI validation.
    pub theme: Theme,
}

/// Typed options for the top-level `chatter validate` command.
#[derive(Clone, Debug)]
pub struct ValidateCommandOptions {
    /// Validation rules and parser choices.
    pub rules: ValidateCommandRules,
    /// Cache, worker-count, and failure-limit settings.
    pub execution: ValidateCommandExecution,
    /// Output, audit, and TUI settings.
    pub presentation: ValidateCommandPresentation,
    /// Error codes to suppress (e.g., ["E726", "E727", "E728"]).
    /// Suppressed errors are not reported and do not affect the exit code.
    pub suppress: Vec<String>,
}

/// Expand named suppress groups into concrete error codes.
///
/// Named groups provide user-friendly shorthand for sets of related error codes.
/// Unknown names are treated as literal error codes (e.g., "E726").
fn expand_suppress_groups(raw: Vec<String>) -> Vec<String> {
    let mut codes = Vec::new();
    for item in raw {
        match item.to_lowercase().as_str() {
            // TEMPORARY: %xphosyl/%xphoaln/%xmodsyl cross-tier alignment (PhonTalk-generated).
            // Remove once Greg fixes the PhonTalk data quality issues.
            "xphon" => codes.extend(["E726", "E727", "E728"].map(String::from)),
            _ => codes.push(item.to_uppercase()),
        }
    }
    codes
}

/// Execute one top-level `chatter validate` invocation.
pub fn run_validate_command(path: PathBuf, options: ValidateCommandOptions) {
    let ValidateCommandOptions {
        rules,
        execution,
        presentation,
        suppress: raw_suppress,
    } = options;
    let suppress = expand_suppress_groups(raw_suppress);
    let ValidateCommandRules {
        alignment,
        roundtrip,
        parser_kind,
    } = rules;
    let ValidateCommandExecution {
        cache_refresh,
        jobs,
        max_errors,
    } = execution;
    let ValidateCommandPresentation {
        format,
        quiet,
        audit_output,
        interface,
        theme,
    } = presentation;

    let traversal = if path.is_file() {
        ValidationTraversalMode::from_recursive(false)
    } else if path.is_dir() {
        ValidationTraversalMode::from_recursive(true)
    } else {
        eprintln!("Error: {:?} is not a file or directory", path);
        std::process::exit(1);
    };

    match traversal {
        ValidationTraversalMode::SingleFile => {
            validate_file(
                &path,
                format,
                alignment,
                cache_refresh,
                quiet,
                interface,
                theme,
                &suppress,
            );
        }
        ValidationTraversalMode::Recursive => {
            let stats = validate_directory_parallel(
                &path,
                ValidateDirectoryOptions {
                    rules: ValidationRules {
                        alignment,
                        roundtrip,
                        parser_kind,
                    },
                    traversal,
                    execution: ValidationExecution {
                        cache_refresh,
                        jobs,
                        max_errors,
                    },
                    presentation: match audit_output {
                        Some(output_path) => ValidationPresentation::Audit { output_path },
                        None => ValidationPresentation::Streaming(StreamingValidationOutput {
                            format,
                            quiet,
                            interface,
                            theme,
                        }),
                    },
                    suppress: suppress.iter().map(|s| s.to_uppercase()).collect(),
                },
            );

            if stats.invalid_files > 0 || stats.parse_errors > 0 {
                std::process::exit(1);
            }
        }
    }
}
