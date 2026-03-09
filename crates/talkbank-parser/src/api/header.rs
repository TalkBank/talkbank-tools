//! Header parsing functions
//!
//! Functions for parsing CHAT file headers.
//!
//! CHAT reference anchors:
//! - <https://talkbank.org/0info/manuals/CHAT.html#File_Headers>
//! - <https://talkbank.org/0info/manuals/CHAT.html#Languages_Header>
//! - <https://talkbank.org/0info/manuals/CHAT.html#Participants_Header>
//! - <https://talkbank.org/0info/manuals/CHAT.html#ID_Header>

use crate::error::ParseResult;
use crate::model::Header;
use crate::parser::ParserInitError;
use crate::parser::TreeSitterParser;
use talkbank_model::{ErrorCode, ErrorContext, ParseError, ParseErrors, Severity, SourceLocation};

/// Parse one header line.
///
/// Header lines start with `@` and contain metadata about the transcript.
///
/// # Examples
///
/// ```ignore
/// use talkbank_parser::parse_header;
/// use talkbank_model::model::Header;
///
/// let input = "@Begin";
/// let header = parse_header(input)?;
/// // Phase 2: Header is now a strongly-typed enum
/// assert!(matches!(header, Header::Begin));
/// # Ok::<(), anyhow::Error>(())
/// ```
pub fn parse_header(input: &str) -> ParseResult<Header> {
    let parser = TreeSitterParser::new().map_err(parser_init_errors)?;
    parser.parse_header(input)
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
