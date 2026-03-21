//! Main-tier and word-level parsing helpers.
//!
//! Like other `api/*` wrappers, these functions create a parser per call.
//! Prefer crate-level pooled helpers when parser construction overhead matters.
//!
//! CHAT reference anchors:
//! - <https://talkbank.org/0info/manuals/CHAT.html#Main_Tier>
//! - <https://talkbank.org/0info/manuals/CHAT.html#Words>

use crate::error::ParseResult;
use crate::model::{MainTier, Word};
use crate::parser::ParserInitError;
use crate::parser::TreeSitterParser;
use talkbank_model::{ChatParser, FragmentSemanticContext};
use talkbank_model::{ErrorCode, ErrorContext, ParseError, ParseErrors, Severity, SourceLocation};

/// Parse one main tier line (without dependent tiers).
///
/// **Important:** this is a legacy synthetic tree-sitter fragment helper. It
/// does not define direct-parser fragment semantics.
///
/// # Examples
///
/// ```ignore
/// use talkbank_parser::synthetic_fragments::parse_main_tier;
///
/// let input = "*CHI:\thello world .";
/// let tier = parse_main_tier(input)?;
/// # Ok::<(), anyhow::Error>(())
/// ```
pub fn parse_main_tier(input: &str) -> ParseResult<MainTier> {
    let parser = TreeSitterParser::new().map_err(parser_init_errors)?;
    ChatParser::parse_main_tier(&parser, input, 0, &talkbank_model::ErrorCollector::new())
        .ok_or_else(parser_rejected_errors)
}

/// Parse one main tier line with explicit fragment semantics.
///
/// Use this when the fragment depends on file-level meaning such as
/// `@Options: CA`.
///
/// **Important:** this remains a synthetic wrapper-based tree-sitter path.
///
/// # Examples
///
/// ```ignore
/// use talkbank_model::ChatOptionFlag;
/// use talkbank_model::FragmentSemanticContext;
/// use talkbank_parser::synthetic_fragments::parse_main_tier_with_context;
///
/// let input = "*CHI:\t(word) .";
/// let context = FragmentSemanticContext::new().with_option_flag(ChatOptionFlag::Ca);
/// let tier = parse_main_tier_with_context(input, &context)?;
/// # Ok::<(), anyhow::Error>(())
/// ```
pub fn parse_main_tier_with_context(
    input: &str,
    context: &FragmentSemanticContext,
) -> ParseResult<MainTier> {
    let parser = TreeSitterParser::new().map_err(parser_init_errors)?;
    ChatParser::parse_main_tier_with_context(
        &parser,
        input,
        0,
        context,
        &talkbank_model::ErrorCollector::new(),
    )
    .ok_or_else(parser_rejected_errors)
}

/// Parse a single CHAT word token with its inline annotations.
///
/// **Important:** this helper extracts the word from a synthetic wrapped
/// utterance. It is useful for legacy compatibility checks, but not the
/// semantic source of truth for isolated fragment parsing.
///
/// # Examples
///
/// ```ignore
/// use talkbank_parser::synthetic_fragments::parse_word;
///
/// let input = "hello@b";
/// let word = parse_word(input)?;
/// # Ok::<(), anyhow::Error>(())
/// ```
pub fn parse_word(input: &str) -> ParseResult<Word> {
    let parser = TreeSitterParser::new().map_err(parser_init_errors)?;
    parser.parse_word(input)
}

/// Convert parser rejection into a consistent parse error.
fn parser_rejected_errors() -> ParseErrors {
    ParseErrors::from(vec![ParseError::new(
        ErrorCode::TreeParsingError,
        Severity::Error,
        SourceLocation::from_offsets(0, 0),
        ErrorContext::new("", 0..0, "TreeSitterParser::parse_main_tier_with_context"),
        "Fragment parser rejected input".to_string(),
    )])
}

/// Convert parser-construction failure into a consistent parse error.
fn parser_init_errors(err: ParserInitError) -> ParseErrors {
    ParseErrors::from(vec![ParseError::new(
        ErrorCode::TreeParsingError,
        Severity::Error,
        SourceLocation::from_offsets(0, 0),
        ErrorContext::new("", 0..0, "TreeSitterParser::new"),
        err.to_string(),
    )])
}
