//! Word parsing from tree-sitter CST — Phase 2 delegation to direct parser.
//!
//! After grammar coarsening (Phase 2), `standalone_word` is a single opaque token.
//! All internal word structure (prefix, body, CA markers, shortenings, form/language
//! suffixes, POS tags) is parsed by the direct parser's `parse_word_impl()`.
//!
//! # Related CHAT Manual Sections
//!
//! - <https://talkbank.org/0info/manuals/CHAT.html#Words>
//! - <https://talkbank.org/0info/manuals/CHAT.html#Word_Tier>

use crate::error::{ErrorCode, ErrorContext, ErrorSink, ParseError, Severity, SourceLocation};
use crate::model::Word;
use crate::parser::tree_parsing::parser_helpers::extract_utf8_text;
use talkbank_model::ParseOutcome;
use tree_sitter::Node;

/// Convert a tree-sitter `standalone_word` node into the typed `Word` model.
///
/// After Phase 2 coarsening, each word is represented as a single opaque token and all internal
/// CHAT word structure (pronunciation, CA markers, shortenings, affixes) is parsed by the direct
/// parser. This helper extracts the UTF-8 text, ensures the node is not `MISSING`, and delegates to
/// `talkbank_direct_parser::word::parse_word_impl`, the canonical parser described in the
/// CHAT Words/Word Tier sections that handles detailed word forms.
pub fn convert_word_node(node: Node, source: &str, errors: &impl ErrorSink) -> ParseOutcome<Word> {
    // CRITICAL: Check for MISSING nodes - tree-sitter error recovery inserts these
    // as placeholders. A MISSING standalone_word is an internal error.
    if node.is_missing() {
        errors.report(ParseError::new(
            ErrorCode::MalformedWordContent,
            Severity::Error,
            SourceLocation::from_offsets(node.start_byte(), node.end_byte()),
            ErrorContext::new(source, node.start_byte()..node.end_byte(), ""),
            format!(
                "Internal error: attempted to convert MISSING tree-sitter node at byte {}",
                node.start_byte()
            ),
        ));
        return ParseOutcome::rejected();
    }

    let text = extract_utf8_text(node, source, errors, "standalone_word", "");
    let offset = node.start_byte();

    // Delegate to direct parser — single source of truth for word structure
    talkbank_direct_parser::word::parse_word_impl(text, offset, errors)
}
