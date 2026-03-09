//! Test module for helpers in `talkbank-chat`.
//!
//! These tests document expected behavior and regressions.

use crate::test_utils::parser_suite::{
    ParserImpl, ParserSuiteError, parser_suite as shared_parser_suite,
};
use talkbank_model::ErrorCollector;
use talkbank_parser::{ParserInitError, TreeSitterParser};

/// Enum variants for TestError.
#[derive(Debug, thiserror::Error)]
pub enum TestError {
    #[error("Failed to create TreeSitterParser: {source}")]
    TreeSitterInit { source: ParserInitError },
    #[error("Failed to create DirectParser: {message}")]
    DirectParserInit { message: DirectParserInitMessage },
    #[error("Parse failed for {parser}")]
    ParseErrors {
        parser: &'static str,
        errors: talkbank_model::ParseErrors,
    },
    #[error("Expected parse to fail for parser {parser}, but it succeeded")]
    UnexpectedParseSuccess { parser: &'static str },
    #[error("Missing @Options header in CA sample")]
    MissingOptionsHeader,
}

/// Type representing DirectParserInitMessage.
#[derive(Debug, thiserror::Error)]
#[error("{0}")]
pub struct DirectParserInitMessage(String);

impl ParserImpl {
    /// Parses chat file result.
    pub fn parse_chat_file_result(
        &self,
        content: &str,
    ) -> Result<talkbank_model::ChatFile, TestError> {
        let result = match self {
            ParserImpl::TreeSitter(p) => p.parse_chat_file(content),
            ParserImpl::Direct(p) => p.parse_chat_file(content),
        };
        result.map_err(|errors| TestError::ParseErrors {
            parser: self.name(),
            errors,
        })
    }
}

/// Returns both parser implementations for testing
pub fn parser_suite() -> Result<Vec<ParserImpl>, TestError> {
    shared_parser_suite().map_err(map_parser_suite_error)
}

fn map_parser_suite_error(error: ParserSuiteError) -> TestError {
    match error {
        ParserSuiteError::TreeSitterInit { source } => TestError::TreeSitterInit { source },
        ParserSuiteError::DirectParserInit { message } => TestError::DirectParserInit {
            message: DirectParserInitMessage(message),
        },
    }
}

/// Parses and validate.
pub fn parse_and_validate(content: &str) -> Result<Vec<talkbank_model::ParseError>, TestError> {
    let parser = TreeSitterParser::new().map_err(|source| TestError::TreeSitterInit { source })?;
    let chat_file = parser
        .parse_chat_file(content)
        .map_err(|errors| TestError::ParseErrors {
            parser: "tree-sitter",
            errors,
        })?;
    let errors = ErrorCollector::new();
    chat_file.validate(&errors, None);
    Ok(errors.into_vec())
}

/// Parses only.
pub fn parse_only(content: &str) -> Result<talkbank_model::ChatFile, TestError> {
    let parser = TreeSitterParser::new().map_err(|source| TestError::TreeSitterInit { source })?;
    parser
        .parse_chat_file(content)
        .map_err(|errors| TestError::ParseErrors {
            parser: "tree-sitter",
            errors,
        })
}
