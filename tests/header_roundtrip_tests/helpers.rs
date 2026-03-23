//! Test module for helpers in `talkbank-chat`.
//!
//! These tests document expected behavior and regressions.

use crate::test_utils::parser_suite::{
    ParserImpl, ParserSuiteError, parser_suite as shared_parser_suite,
};
use talkbank_model::ErrorCollector;
use talkbank_model::{ChatParser, ParseOutcome};
use talkbank_parser::ParserInitError;

/// Enum variants for TestError.
#[derive(Debug, thiserror::Error)]
pub enum TestError {
    #[error("Failed to create TreeSitterParser: {source}")]
    TreeSitterInit { source: ParserInitError },
    #[error("Failed to create TreeSitterParser: {message}")]
    ParserInit { message: ParserInitMessage },
    #[error("Header parse failed for {parser}")]
    ParseFailed { parser: &'static str },
    #[error("Header parse errors for {parser}")]
    ParseErrors {
        parser: &'static str,
        errors: talkbank_model::ParseErrors,
    },
}

/// Type representing ParserInitMessage.
#[derive(Debug, thiserror::Error)]
#[error("{0}")]
pub struct ParserInitMessage(String);

impl ParserImpl {
    /// Parses header.
    pub fn parse_header(&self, input: &str) -> Result<talkbank_model::Header, TestError> {
        let errors = ErrorCollector::new();
        let header = match self {
            ParserImpl::TreeSitter(p) => ChatParser::parse_header(p, input, 0, &errors),
            ParserImpl::Direct(p) => ChatParser::parse_header(p, input, 0, &errors),
        };

        if let ParseOutcome::Parsed(header) = header {
            if errors.is_empty() {
                Ok(header)
            } else {
                Err(TestError::ParseErrors {
                    parser: self.name(),
                    errors: talkbank_model::ParseErrors {
                        errors: errors.into_vec(),
                    },
                })
            }
        } else if errors.is_empty() {
            Err(TestError::ParseFailed {
                parser: self.name(),
            })
        } else {
            Err(TestError::ParseErrors {
                parser: self.name(),
                errors: talkbank_model::ParseErrors {
                    errors: errors.into_vec(),
                },
            })
        }
    }
}

/// Returns both parser implementations for testing
pub fn parser_suite() -> Result<Vec<ParserImpl>, TestError> {
    shared_parser_suite().map_err(map_parser_suite_error)
}

fn map_parser_suite_error(error: ParserSuiteError) -> TestError {
    match error {
        ParserSuiteError::TreeSitterInit { source } => TestError::TreeSitterInit { source },
        ParserSuiteError::ParserInit { message } => TestError::ParserInit {
            message: ParserInitMessage(message),
        },
    }
}

/// Test header roundtrip for both parsers
pub fn assert_header_roundtrip(input: &str) -> Result<(), TestError> {
    for parser in parser_suite()? {
        let header = parser.parse_header(input)?;
        let output = header.to_chat();
        assert_eq!(output, input, "[{}] Header should roundtrip", parser.name());
    }

    Ok(())
}
