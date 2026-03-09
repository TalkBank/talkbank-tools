//! Test module for helpers in `talkbank-chat`.
//!
//! These tests document expected behavior and regressions.

use talkbank_direct_parser::DirectParser;
use talkbank_parser::TreeSitterParser;
use talkbank_parser_tests::test_error::TestError;

/// Enum wrapper for testing both parser implementations
pub enum ParserImpl {
    TreeSitter(TreeSitterParser),
    Direct(DirectParser),
}

impl ParserImpl {
    /// Short backend label for snapshot names and failure messages.
    pub fn name(&self) -> &'static str {
        match self {
            ParserImpl::TreeSitter(_) => "tree-sitter",
            ParserImpl::Direct(_) => "direct",
        }
    }
}

/// Returns both parser implementations for round-trip testing
pub fn parser_suite() -> Result<Vec<ParserImpl>, TestError> {
    let tree_sitter =
        TreeSitterParser::new().map_err(|err| TestError::ParserInit(err.to_string()))?;
    let direct = DirectParser::new().map_err(|err| TestError::ParserInit(err.to_string()))?;
    Ok(vec![
        ParserImpl::TreeSitter(tree_sitter),
        ParserImpl::Direct(direct),
    ])
}

/// Legacy helper - returns TreeSitterParser only (for backwards compatibility)
pub fn parser() -> Result<TreeSitterParser, TestError> {
    TreeSitterParser::new().map_err(|err| TestError::ParserInit(err.to_string()))
}
