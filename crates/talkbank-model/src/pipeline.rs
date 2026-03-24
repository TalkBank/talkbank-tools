//! Pipeline options and helpers for CHAT file processing workflows.
//!
//! This module is a thin configuration layer that sits between the parser crate
//! (`talkbank-parser`) and the orchestration layer (`talkbank-transform`).
//! It defines the option
//! structs that control *what happens after parsing* -- specifically, which
//! validation phases to run.
//!
//! # Architecture
//!
//! ```text
//!   Parser crate              talkbank-model::pipeline  talkbank-transform
//!   ┌──────────────┐         ┌──────────────────┐      ┌──────────────────┐
//!   │ tree-sitter  │─parse──▶│ ParseValidateOpts│─────▶│ parse_and_validate│
//!   │ direct       │         │ validate_chat_... │      │ (orchestrator)   │
//!   └──────────────┘         └──────────────────┘      └──────────────────┘
//! ```
//!
//! ## Builder pattern
//!
//! [`ParseValidateOptions`] uses a builder pattern with `with_*` methods that
//! return `Self`, enabling fluent configuration chains. The default is
//! parse-only (no validation):
//!
//! | Configuration | `validate` | `alignment` | Effect |
//! |---------------|-----------|-------------|--------|
//! | `default()` | `false` | `false` | Parse only, skip all validation |
//! | `with_validation()` | `true` | `false` | Structural validation (headers, speakers) |
//! | `with_alignment()` | `true` | `true` | Full validation including cross-tier alignment |
//!
//! # Examples
//!
//! Parse-only (fastest, used by roundtrip re-parse):
//!
//! ```
//! use talkbank_model::ParseValidateOptions;
//!
//! let options = ParseValidateOptions::default();
//! assert!(!options.should_validate());
//! ```
//!
//! Structural validation without alignment (for quick checks):
//!
//! ```
//! use talkbank_model::ParseValidateOptions;
//!
//! let options = ParseValidateOptions::default().with_validation();
//! assert!(options.should_validate());
//! assert!(!options.alignment);
//! ```
//!
//! Full validation with cross-tier alignment (the standard pipeline):
//!
//! ```
//! use talkbank_model::ParseValidateOptions;
//!
//! // with_alignment() implies with_validation()
//! let options = ParseValidateOptions::default().with_alignment();
//! assert!(options.should_validate());
//! assert!(options.alignment);
//! ```

use crate::ParseError;

use crate::ChatFile;

/// Options for parse-and-validate pipeline.
#[derive(Debug, Clone, Default)]
pub struct ParseValidateOptions {
    /// Run structural/data-model validation after parse.
    pub validate: bool,
    /// Run cross-tier alignment validation (`%mor`, `%gra`, `%pho`, `%wor`).
    ///
    /// This implies `validate = true`.
    pub alignment: bool,
}

impl ParseValidateOptions {
    /// Enable structural/data-model validation.
    pub fn with_validation(mut self) -> Self {
        self.validate = true;
        self
    }

    /// Enable alignment validation and its prerequisite structural validation.
    pub fn with_alignment(mut self) -> Self {
        self.alignment = true;
        self.validate = true; // alignment implies validation
        self
    }

    /// Return `true` if any validation phase should run.
    pub fn should_validate(&self) -> bool {
        self.validate || self.alignment
    }
}

/// Validate a parsed ChatFile according to options.
///
/// This helper function validates a ChatFile according to the options,
/// returning validation errors if any.
///
/// # Arguments
///
/// * `chat_file` - Parsed CHAT model.
///   Alignment validation may annotate alignment state, so `&mut` is required.
/// * `options` - Validation options
///
/// # Returns
///
/// * `Ok(())` - Validation passed or was skipped.
/// * `Err(Vec<ParseError>)` - At least one validation error/warning was emitted.
pub fn validate_chat_file_with_options(
    chat_file: &mut ChatFile,
    options: &ParseValidateOptions,
) -> Result<(), Vec<ParseError>> {
    use crate::ErrorCollector;

    if !options.should_validate() {
        return Ok(());
    }

    let errors = ErrorCollector::new();
    if options.alignment {
        chat_file.validate_with_alignment(&errors, None);
    } else {
        chat_file.validate(&errors, None);
    }

    let error_vec = errors.into_vec();
    if !error_vec.is_empty() {
        Err(error_vec)
    } else {
        Ok(())
    }
}
