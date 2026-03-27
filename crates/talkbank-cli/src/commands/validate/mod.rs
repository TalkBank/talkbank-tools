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
            // TEMPORARY: %xmodsyl/%xphosyl/%xphoaln cross-tier alignment (PhonTalk-generated).
            // E725: %modsyl vs %mod, E726: %phosyl vs %pho,
            // E727: %phoaln vs %mod, E728: %phoaln vs %pho.
            // Remove once Greg fixes the PhonTalk data quality issues.
            "xphon" => codes.extend(["E725", "E726", "E727", "E728"].map(String::from)),
            _ => codes.push(item.to_uppercase()),
        }
    }
    codes
}

#[cfg(test)]
mod tests {
    use super::expand_suppress_groups;

    #[test]
    fn xphon_expands_to_all_phon_cross_tier_codes() {
        let result = expand_suppress_groups(vec!["xphon".to_string()]);
        // E725: %modsyl vs %mod
        // E726: %phosyl vs %pho
        // E727: %phoaln vs %mod
        // E728: %phoaln vs %pho
        assert!(
            result.contains(&"E725".to_string()),
            "missing E725 (modsyl/mod)"
        );
        assert!(
            result.contains(&"E726".to_string()),
            "missing E726 (phosyl/pho)"
        );
        assert!(
            result.contains(&"E727".to_string()),
            "missing E727 (phoaln/mod)"
        );
        assert!(
            result.contains(&"E728".to_string()),
            "missing E728 (phoaln/pho)"
        );
        assert_eq!(result.len(), 4);
    }

    #[test]
    fn literal_codes_pass_through_uppercased() {
        let result = expand_suppress_groups(vec!["e316".to_string()]);
        assert_eq!(result, vec!["E316"]);
    }

    #[test]
    fn mixed_groups_and_codes() {
        let result = expand_suppress_groups(vec!["xphon".to_string(), "E316".to_string()]);
        assert_eq!(result.len(), 5);
        assert!(result.contains(&"E725".to_string()));
        assert!(result.contains(&"E316".to_string()));
    }
}

/// Execute one top-level `chatter validate` invocation.
///
/// Accepts one or more paths. Each path can be a file or directory.
/// Multiple files are validated individually. A single directory uses
/// the parallel directory validation pipeline.
pub fn run_validate_command(paths: Vec<PathBuf>, options: ValidateCommandOptions) {
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

    // Classify paths into files and directories
    let mut files: Vec<PathBuf> = Vec::new();
    let mut dirs: Vec<PathBuf> = Vec::new();
    for p in &paths {
        if p.is_file() {
            files.push(p.clone());
        } else if p.is_dir() {
            dirs.push(p.clone());
        } else {
            eprintln!("Error: {:?} is not a file or directory", p);
            std::process::exit(1);
        }
    }

    let mut had_errors = false;

    // Validate individual files
    for file_path in &files {
        validate_file(
            file_path,
            format,
            alignment,
            cache_refresh,
            quiet,
            interface,
            theme.clone(),
            &suppress,
        );
    }

    // Validate directories (use parallel pipeline for each)
    for dir_path in &dirs {
        let stats = validate_directory_parallel(
            dir_path,
            ValidateDirectoryOptions {
                rules: ValidationRules {
                    alignment,
                    roundtrip,
                    parser_kind,
                },
                traversal: ValidationTraversalMode::Recursive,
                execution: ValidationExecution {
                    cache_refresh,
                    jobs,
                    max_errors,
                },
                presentation: match &audit_output {
                    Some(output_path) => ValidationPresentation::Audit {
                        output_path: output_path.clone(),
                    },
                    None => ValidationPresentation::Streaming(StreamingValidationOutput {
                        format,
                        quiet,
                        interface,
                        theme: theme.clone(),
                    }),
                },
                suppress: suppress.clone(),
            },
        );

        if stats.invalid_files > 0 || stats.parse_errors > 0 {
            had_errors = true;
        }
    }

    if had_errors {
        std::process::exit(1);
    }
}
