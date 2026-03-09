//! CHECK — CHAT file validation.
//!
//! Validates CHAT files for structural correctness, checking headers, tier
//! formatting, bracket matching, bullet consistency, and more.
//!
//! # CLAN Equivalence
//!
//! | CLAN command                | Rust equivalent                              |
//! |-----------------------------|----------------------------------------------|
//! | `check file.cha`            | `chatter clan check file.cha`                |
//! | `check +c0 file.cha`        | `chatter clan check --bullets 0 file.cha`    |
//! | `check +e file.cha`         | `chatter clan check --list-errors file.cha`  |
//! | `check +e6 file.cha`        | `chatter clan check --error 6 file.cha`      |
//! | `check -e6 file.cha`        | `chatter clan check --exclude-error 6 file.cha` |
//! | `check +g2 file.cha`        | `chatter clan check --check-target file.cha` |
//! | `check +g4 file.cha`        | `chatter clan check --check-id file.cha`     |
//! | `check +g5 file.cha`        | `chatter clan check --check-unused file.cha` |
//! | `check +u file.cha`         | `chatter clan check --check-ud file.cha`     |
//!
//! # Differences from CLAN
//!
//! - **Parsing**: Uses tree-sitter grammar for CHAT parsing, which is more
//!   rigorous and consistent than CLAN's hand-written character-by-character
//!   parser. Many of CHECK's 161 error numbers correspond to parse errors that
//!   our parser catches structurally.
//! - **Error numbering**: CLAN CHECK uses a flat numbering system (1–161).
//!   We map our typed error codes to CHECK's numbers where a clear
//!   correspondence exists. Errors without a CHECK equivalent get number 0.
//! - **Two-pass architecture**: CLAN CHECK uses two passes: `check_OverAll`
//!   (basic structure) and `check_CheckRest` (semantic validation). Our parser
//!   combines both passes into a single streaming parse+validate pipeline.
//! - **depfile.cut**: CLAN CHECK reads `depfile.cut` for tier/code templates.
//!   We validate against the CHAT specification directly without external
//!   template files.
//! - **Output format**: In default mode, we emit CLAN-compatible output:
//!   `*** File "path": line N.` followed by the tier text and error message
//!   with `(N)` error number suffix.
//! - **Bug fixes**: Several CHECK errors in the original are unreachable or
//!   duplicate (e.g., errors 51, 96 are commented out). We skip those.
//! - **`+g1` (prosodic delimiters)**: Currently a no-op — our parser always
//!   recognizes prosodic delimiters.
//! - **`+g3` (word detail checks)**: Partially implemented through our
//!   existing word validation (illegal characters, digits in words).

use std::collections::BTreeSet;
use std::fmt::Write;
use std::path::Path;

use serde::Serialize;
use talkbank_model::ParseValidateOptions;
use talkbank_model::{ChatFile, ChatOptionFlag, Header, Line};
use talkbank_model::{ErrorCollector, ParseError};

use crate::framework::CommandOutput;

mod error_map;
pub use error_map::check_error_number;

// ---------------------------------------------------------------------------
// Configuration
// ---------------------------------------------------------------------------

/// Configuration for the CHECK command.
#[derive(Debug, Clone)]
pub struct CheckConfig {
    /// Bullet consistency check level: `None` = skip, `Some(0)` = full,
    /// `Some(1)` = missing bullets only.
    pub bullets: Option<u8>,
    /// Only report errors with these numbers (empty = report all).
    pub include_errors: BTreeSet<u16>,
    /// Exclude errors with these numbers.
    pub exclude_errors: BTreeSet<u16>,
    /// If true, list all error numbers and their messages, then exit.
    pub list_errors: bool,
    /// `+g2`: Check that CHI has Target_Child role.
    pub check_target_child: bool,
    /// `+g4`: Check for missing @ID tiers.
    pub check_missing_id: bool,
    /// `+g5`: Check for unused speakers.
    pub check_unused_speakers: bool,
    /// `+u`: Validate UD features on %mor tier.
    pub check_ud_features: bool,
}

impl Default for CheckConfig {
    fn default() -> Self {
        Self {
            bullets: None,
            include_errors: BTreeSet::new(),
            exclude_errors: BTreeSet::new(),
            list_errors: false,
            check_target_child: false,
            check_missing_id: true, // CLAN default: +g4 is on
            check_unused_speakers: false,
            check_ud_features: false,
        }
    }
}

impl CheckConfig {
    /// Returns true if the given CHECK error number should be reported.
    fn should_report(&self, error_num: u16) -> bool {
        if !self.include_errors.is_empty() {
            return self.include_errors.contains(&error_num);
        }
        if self.exclude_errors.contains(&error_num) {
            return false;
        }
        true
    }
}

// ---------------------------------------------------------------------------
// Result
// ---------------------------------------------------------------------------

/// A single CHECK error in CLAN-compatible format.
#[derive(Debug, Clone, Serialize)]
pub struct CheckError {
    /// CHECK error number (1–161), or 0 for unmapped errors.
    pub error_number: u16,
    /// Line number in the file (1-based), or 0 for file-level errors.
    pub line: usize,
    /// The error message text.
    pub message: String,
    /// The tier/line text where the error occurred (if available).
    pub context: String,
    /// Our internal error code (e.g., "E502").
    pub error_code: String,
}

/// Output of the CHECK command.
#[derive(Debug, Clone, Serialize)]
pub struct CheckResult {
    /// Path to the file that was checked.
    pub file: String,
    /// All errors found (after filtering).
    pub errors: Vec<CheckError>,
    /// True if any errors were found (including filtered ones).
    pub has_errors: bool,
}

impl CommandOutput for CheckResult {
    fn render_text(&self) -> String {
        let mut out = String::new();
        if self.errors.is_empty() && !self.has_errors {
            return out;
        }
        for err in &self.errors {
            let _ = writeln!(out, "*** File \"{}\": line {}.", self.file, err.line);
            if !err.context.is_empty() {
                let _ = writeln!(out, "{}", err.context);
            }
            if err.error_number > 0 {
                let _ = writeln!(out, "{}({})", err.message, err.error_number);
            } else {
                let _ = writeln!(out, "{} [{}]", err.message, err.error_code);
            }
        }
        out
    }

    fn render_clan(&self) -> String {
        self.render_text()
    }
}

// ---------------------------------------------------------------------------
// Error listing
// ---------------------------------------------------------------------------

/// Returns the full list of CHECK error numbers and their messages.
///
/// This corresponds to `check +e` which lists all error numbers.
pub fn list_all_errors() -> String {
    let mut out = String::new();
    for n in 1..=161u16 {
        let msg = error_map::check_error_message(n);
        let _ = writeln!(out, "{n:3}: {msg}");
    }
    out
}

// ---------------------------------------------------------------------------
// Running CHECK
// ---------------------------------------------------------------------------

/// Run CHECK on a single file, returning the result.
pub fn run_check(path: &Path, content: &str, config: &CheckConfig) -> CheckResult {
    let options = ParseValidateOptions::default().with_alignment();
    let sink = ErrorCollector::new();

    let parse_result = talkbank_transform::parse_and_validate_streaming(content, options, &sink);

    let collected = sink.into_vec();
    let has_errors = !collected.is_empty() || parse_result.is_err();

    let mut errors = convert_errors(content, &collected, config);

    // Additional checks that require the parsed AST
    if let Ok(file) = &parse_result {
        run_additional_checks(file, config, &mut errors);
    }

    CheckResult {
        file: path.display().to_string(),
        errors,
        has_errors,
    }
}

/// Convert our ParseErrors to CHECK-compatible errors, applying filtering.
fn convert_errors(content: &str, errors: &[ParseError], config: &CheckConfig) -> Vec<CheckError> {
    let lines: Vec<&str> = content.lines().collect();

    errors
        .iter()
        .filter_map(|err| {
            let error_num = check_error_number(&err.code);
            if !config.should_report(error_num) {
                return None;
            }

            // Use line from SourceLocation if available, else compute from span
            let line = err
                .location
                .line
                .unwrap_or_else(|| byte_offset_to_line(content, err.location.span.start as usize));

            let context = if line > 0 && line <= lines.len() {
                lines[line - 1].to_string()
            } else {
                String::new()
            };

            Some(CheckError {
                error_number: error_num,
                line,
                message: err.message.clone(),
                context,
                error_code: err.code.as_str().to_string(),
            })
        })
        .collect()
}

/// Additional checks that run on the parsed AST (not covered by parse+validate).
fn run_additional_checks(file: &ChatFile, config: &CheckConfig, errors: &mut Vec<CheckError>) {
    if config.check_target_child {
        check_target_child(file, config, errors);
    }
    if config.check_unused_speakers {
        check_unused_speakers(file, config, errors);
    }
}

/// Check that CHI has Target_Child role (+g2).
fn check_target_child(file: &ChatFile, config: &CheckConfig, errors: &mut Vec<CheckError>) {
    if !config.should_report(68) {
        return;
    }

    let has_chi_target = file
        .participants
        .iter()
        .any(|(code, p)| code.as_str() == "CHI" && p.role.as_str() == "Target_Child");

    // Check for @Options: notarget
    let notarget = file.lines.iter().any(|line| {
        if let Line::Header { header, .. } = line
            && let Header::Options { options } = header.as_ref()
        {
            return options
                .iter()
                .any(|o| matches!(o, ChatOptionFlag::Unsupported(s) if s == "notarget"));
        }
        false
    });

    if !notarget && !has_chi_target {
        errors.push(CheckError {
            error_number: 68,
            line: 0,
            message: "PARTICIPANTS TIER IS MISSING \"CHI Target_Child\".".to_string(),
            context: String::new(),
            error_code: String::new(),
        });
    }
}

/// Check for speakers declared in @Participants but never used (+g5).
fn check_unused_speakers(file: &ChatFile, config: &CheckConfig, errors: &mut Vec<CheckError>) {
    let declared: BTreeSet<&str> = file.participants.keys().map(|code| code.as_str()).collect();

    let mut used: BTreeSet<&str> = BTreeSet::new();
    for line in file.lines.iter() {
        if let Line::Utterance(utt) = line {
            used.insert(utt.main.speaker.as_str());
        }
    }

    for speaker in declared.difference(&used) {
        if !config.should_report(0) {
            continue;
        }
        errors.push(CheckError {
            error_number: 0,
            line: 3, // CLAN reports this at line 3 (participants line)
            message: format!("Speaker \"{speaker}\" is not used in this file."),
            context: String::new(),
            error_code: String::new(),
        });
    }
}

/// Convert a byte offset to a 1-based line number.
fn byte_offset_to_line(content: &str, offset: usize) -> usize {
    content[..offset.min(content.len())]
        .bytes()
        .filter(|&b| b == b'\n')
        .count()
        + 1
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_list_all_errors() {
        let listing = list_all_errors();
        assert!(listing.contains("  6:"));
        assert!(listing.contains("  7:"));
        assert!(listing.contains("161:"));
        assert_eq!(listing.lines().count(), 161);
    }

    #[test]
    fn test_config_error_filtering() {
        let mut config = CheckConfig::default();
        assert!(config.should_report(6));
        assert!(config.should_report(7));

        config.include_errors.insert(6);
        assert!(config.should_report(6));
        assert!(!config.should_report(7));

        let mut config2 = CheckConfig::default();
        config2.exclude_errors.insert(6);
        assert!(!config2.should_report(6));
        assert!(config2.should_report(7));
    }

    #[test]
    fn test_byte_offset_to_line() {
        let content = "line1\nline2\nline3\n";
        assert_eq!(byte_offset_to_line(content, 0), 1);
        assert_eq!(byte_offset_to_line(content, 5), 1);
        assert_eq!(byte_offset_to_line(content, 6), 2);
        assert_eq!(byte_offset_to_line(content, 12), 3);
    }

    #[test]
    fn test_check_valid_file() {
        let content = "\u{FEFF}@UTF8\n@Begin\n@Languages:\teng\n\
            @Participants:\tCHI Target_Child\n\
            @ID:\teng|test|CHI|2;0.||||Target_Child|||\n\
            *CHI:\tdog .\n@End\n";
        let config = CheckConfig::default();
        let result = run_check(Path::new("test.cha"), content, &config);
        assert!(
            result.errors.is_empty(),
            "Expected no errors, got: {:?}",
            result.errors
        );
    }

    #[test]
    fn test_check_missing_end() {
        let content = "\u{FEFF}@UTF8\n@Begin\n@Languages:\teng\n\
            @Participants:\tCHI Target_Child\n\
            @ID:\teng|test|CHI|2;0.||||Target_Child|||\n\
            *CHI:\tdog .\n";
        let config = CheckConfig::default();
        let result = run_check(Path::new("test.cha"), content, &config);
        assert!(result.has_errors);
        assert!(
            result.errors.iter().any(|e| e.error_number == 7),
            "Expected error 7 (missing @End), got: {:?}",
            result.errors
        );
    }
}
