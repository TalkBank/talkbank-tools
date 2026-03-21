//! File-level parsing helpers.
//!
//! These functions construct a fresh [`TreeSitterParser`] per call. They are
//! straightforward API entry points for library consumers. For high-throughput
//! batch usage, prefer the crate-level `parse_*` helpers, which reuse a
//! thread-local parser instance.
//!
//! CHAT reference anchors:
//! - <https://talkbank.org/0info/manuals/CHAT.html#File_Headers>
//! - <https://talkbank.org/0info/manuals/CHAT.html#Main_Tier>

use crate::error::ParseResult;
use crate::model::{ChatFile, Utterance};
use crate::parser::ParserInitError;
use crate::parser::TreeSitterParser;
use talkbank_model::{ChatParser, FragmentSemanticContext};
use talkbank_model::{ErrorCode, ErrorContext, ParseError, ParseErrors, Severity, SourceLocation};

/// Parse a complete CHAT file.
///
/// Handles:
/// - File headers (@Begin, @End, @Participants, etc.)
/// - Main tier lines (speaker utterances)
/// - Dependent tier lines (morphology, grammar, etc.)
/// - Tier alignment and validation
///
/// # Examples
///
/// ```ignore
/// use talkbank_parser::parse_chat_file;
///
/// // Parses complete CHAT files with headers, utterances, and dependent tiers
/// let file = parse_chat_file(complete_chat_file_content)?;
/// # Ok::<(), anyhow::Error>(())
/// ```
pub fn parse_chat_file(input: &str) -> ParseResult<ChatFile> {
    let parser = TreeSitterParser::new().map_err(parser_init_errors)?;
    parser.parse_chat_file(input)
}

/// Parse a single utterance (main tier plus attached dependent tiers).
///
/// The input is treated as one utterance unit, not a full CHAT file.
///
/// **Important:** this is still a synthetic tree-sitter fragment helper. It
/// may wrap the input in a minimal CHAT file before parsing.
///
/// # Examples
///
/// ```ignore
/// use talkbank_parser::synthetic_fragments::parse_utterance;
///
/// let input = "*CHI:\thello .";
/// let utterance = parse_utterance(input)?;
/// # Ok::<(), anyhow::Error>(())
/// ```
pub fn parse_utterance(input: &str) -> ParseResult<Utterance> {
    let parser = TreeSitterParser::new().map_err(parser_init_errors)?;
    ChatParser::parse_utterance(&parser, input, 0, &talkbank_model::ErrorCollector::new())
        .ok_or_else(parser_rejected_errors)
}

/// Parse a single utterance with explicit fragment semantics.
///
/// Use this when the utterance depends on file-level meaning such as
/// `@Options: CA`.
///
/// **Important:** this remains a synthetic wrapper-based tree-sitter path.
pub fn parse_utterance_with_context(
    input: &str,
    context: &FragmentSemanticContext,
) -> ParseResult<Utterance> {
    let parser = TreeSitterParser::new().map_err(parser_init_errors)?;
    ChatParser::parse_utterance_with_context(
        &parser,
        input,
        0,
        context,
        &talkbank_model::ErrorCollector::new(),
    )
    .ok_or_else(parser_rejected_errors)
}

/// Convert parser rejection into a consistent parse error.
fn parser_rejected_errors() -> ParseErrors {
    ParseErrors::from(vec![ParseError::new(
        ErrorCode::TreeParsingError,
        Severity::Error,
        SourceLocation::from_offsets(0, 0),
        ErrorContext::new("", 0..0, "TreeSitterParser::parse_utterance_with_context"),
        "Fragment parser rejected input".to_string(),
    )])
}

/// Convert parser-construction failure into a consistent parse error.
///
/// API callers expect `ParseErrors`, so initialization failures are normalized
/// into a synthetic `TreeParsingError`.
fn parser_init_errors(err: ParserInitError) -> ParseErrors {
    ParseErrors::from(vec![ParseError::new(
        ErrorCode::TreeParsingError,
        Severity::Error,
        SourceLocation::from_offsets(0, 0),
        ErrorContext::new("", 0..0, "TreeSitterParser::new"),
        err.to_string(),
    )])
}
