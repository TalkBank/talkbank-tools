//! Shared parser-suite utilities for root integration tests.

use talkbank_parser::TreeSitterParser;
use talkbank_model::{ChatFile, ChatParser, ErrorSink};
use talkbank_parser::ParserInitError;

/// Shared initialization failure for test parser suites.
#[derive(Debug, thiserror::Error)]
pub enum ParserSuiteError {
    #[error("Failed to create TreeSitterParser: {source}")]
    TreeSitterInit { source: ParserInitError },
    #[error("Failed to create TreeSitterParser: {message}")]
    ParserInit { message: String },
}

/// Enum wrapper for testing both parser implementations.
pub enum ParserImpl {
    TreeSitter(TreeSitterParser),
    Direct(TreeSitterParser),
}

impl ParserImpl {
    /// Returns the display name used in assertions.
    pub fn name(&self) -> &'static str {
        match self {
            ParserImpl::TreeSitter(_) => "tree-sitter",
            ParserImpl::Direct(_) => "direct",
        }
    }

    /// Parse a CHAT file through the streaming API.
    pub fn parse_chat_file_streaming(
        &self,
        input: &str,
        errors: &impl ErrorSink,
    ) -> Option<ChatFile> {
        match self {
            ParserImpl::TreeSitter(parser) => {
                ChatParser::parse_chat_file(parser, input, 0, errors).into()
            }
            ParserImpl::Direct(parser) => {
                ChatParser::parse_chat_file(parser, input, 0, errors).into()
            }
        }
    }
}

/// Build the standard root-test parser suite.
pub fn parser_suite() -> Result<Vec<ParserImpl>, ParserSuiteError> {
    let tree_sitter =
        TreeSitterParser::new().map_err(|source| ParserSuiteError::TreeSitterInit { source });
    let direct =
        TreeSitterParser::new().map_err(|message| ParserSuiteError::ParserInit { message: message.to_string() });

    match (tree_sitter, direct) {
        (Ok(tree_sitter), Ok(direct)) => Ok(vec![
            ParserImpl::TreeSitter(tree_sitter),
            ParserImpl::Direct(direct),
        ]),
        (Err(error), _) | (_, Err(error)) => Err(error),
    }
}
