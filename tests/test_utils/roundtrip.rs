//! Shared roundtrip testing logic - DO NOT DUPLICATE
//!
//! Single source of truth for how roundtrip tests should work.

#![allow(dead_code)] // Some utilities are for future batch workflows

use talkbank_model::ParseValidateOptions;
use talkbank_model::model::ChatFile;
use talkbank_model::{ChatParser, ParseOutcome};
use talkbank_model::{ErrorCollector, ParseErrors, Severity};
use talkbank_parser::TreeSitterParser;
use talkbank_transform::{PipelineError, parse_and_validate, parse_and_validate_with_parser};

/// Parse and validate a CHAT file with consistent error handling.
///
/// This is the ONLY way roundtrip tests should parse files.
///
/// Returns:
/// - Ok(ChatFile) if parsing succeeds (with or without warnings)
/// - Err(PipelineError) if parsing or validation fails
pub fn parse_for_roundtrip(
    content: &str,
    check_alignment: bool,
) -> Result<ChatFile, PipelineError> {
    let options = if check_alignment {
        ParseValidateOptions::default().with_alignment()
    } else {
        ParseValidateOptions::default().with_validation()
    };

    match parse_and_validate(content, options) {
        Ok(file) => Ok(file),
        Err(PipelineError::Validation(errors)) => {
            // Only fail if there were actual errors (not warnings)
            let has_error = errors.iter().any(|e| e.severity == Severity::Error);

            if has_error {
                Err(PipelineError::Validation(errors))
            } else {
                parse_and_validate(content, ParseValidateOptions::default())
            }
        }
        Err(e) => Err(e),
    }
}

/// Parser-reuse variant of parse_for_roundtrip for batch workflows.
pub fn parse_for_roundtrip_with_parser(
    parser: &TreeSitterParser,
    content: &str,
    check_alignment: bool,
) -> Result<ChatFile, PipelineError> {
    let options = if check_alignment {
        ParseValidateOptions::default().with_alignment()
    } else {
        ParseValidateOptions::default().with_validation()
    };

    match parse_and_validate_with_parser(parser, content, options) {
        Ok(file) => Ok(file),
        Err(PipelineError::Validation(errors)) => {
            // Only fail if there were actual errors (not warnings)
            let has_error = errors.iter().any(|e| e.severity == Severity::Error);

            if has_error {
                Err(PipelineError::Validation(errors))
            } else {
                parse_and_validate_with_parser(parser, content, ParseValidateOptions::default())
            }
        }
        Err(e) => Err(e),
    }
}

/// Parse and validate a CHAT file using any ChatParser implementation.
///
/// Preconditions:
/// - `content` is full CHAT file content.
/// - `parser` is ready to parse CHAT content.
/// - `check_alignment` determines whether alignment validation runs.
///
/// Postconditions:
/// - Returns `Ok(ChatFile)` if parsing succeeds and validation passes.
/// - Returns `Err(PipelineError)` when parsing fails or validation has errors.
/// - Validation warnings do not fail the result (falls back to parse-only).
///
/// Invariants:
/// - Does not mutate external state beyond parser internals.
///
/// Complexity:
/// - Time: O(n) in the size of `content`.
/// - Space: O(n) for the parsed model and error collection.
pub fn parse_for_roundtrip_with_chat_parser<P: ChatParser>(
    parser: &P,
    content: &str,
    check_alignment: bool,
) -> Result<ChatFile, PipelineError> {
    let options = if check_alignment {
        ParseValidateOptions::default().with_alignment()
    } else {
        ParseValidateOptions::default().with_validation()
    };

    match parse_and_validate_with_chat_parser(parser, content, options) {
        Ok(file) => Ok(file),
        Err(PipelineError::Validation(errors)) => {
            let has_error = errors.iter().any(|e| e.severity == Severity::Error);

            if has_error {
                Err(PipelineError::Validation(errors))
            } else {
                parse_and_validate_with_chat_parser(
                    parser,
                    content,
                    ParseValidateOptions::default(),
                )
            }
        }
        Err(e) => Err(e),
    }
}

/// Parse and validate using a ChatParser implementation.
///
/// Preconditions:
/// - `content` is full CHAT file content.
/// - `parser` is ready to parse CHAT content.
///
/// Postconditions:
/// - Returns `Ok(ChatFile)` on successful parse and validation.
/// - Returns `Err(PipelineError::Parse)` if parse errors occur.
/// - Returns `Err(PipelineError::Validation)` if validation errors occur.
///
/// Invariants:
/// - Validation is only executed when requested in `options`.
///
/// Complexity:
/// - Time: O(n) in the size of `content`.
/// - Space: O(n) for the parsed model and error collection.
fn parse_and_validate_with_chat_parser<P: ChatParser>(
    parser: &P,
    content: &str,
    options: ParseValidateOptions,
) -> Result<ChatFile, PipelineError> {
    let parse_errors = ErrorCollector::new();
    let chat_file = ChatParser::parse_chat_file(parser, content, 0, &parse_errors);

    let parse_error_vec = parse_errors.into_vec();
    let has_actual_errors = parse_error_vec
        .iter()
        .any(|e| e.severity == Severity::Error);
    if has_actual_errors {
        return Err(PipelineError::Parse(ParseErrors {
            errors: parse_error_vec,
        }));
    }

    let mut chat_file = match chat_file {
        ParseOutcome::Parsed(file) => file,
        ParseOutcome::Rejected => {
            return Err(PipelineError::Parse(ParseErrors {
                errors: parse_error_vec,
            }));
        }
    };

    if options.validate || options.alignment {
        let validation_errors = ErrorCollector::new();

        if options.alignment {
            chat_file.validate_with_alignment(&validation_errors, None);
        } else {
            chat_file.validate(&validation_errors, None);
        }

        let validation_error_vec = validation_errors.into_vec();
        if !validation_error_vec.is_empty() {
            return Err(PipelineError::Validation(validation_error_vec));
        }
    }

    Ok(chat_file)
}
