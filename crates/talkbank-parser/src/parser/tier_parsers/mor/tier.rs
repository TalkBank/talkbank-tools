//! Tier-level `%mor` parsing.
//!
//! This file parses one morphology tier line, then delegates each item to
//! `parse_mor_content` and records any terminator token carried in the tier.
//!
//! # Related CHAT Manual Sections
//!
//! - <https://talkbank.org/0info/manuals/CHAT.html#Morphological_Tier>
//! - <https://talkbank.org/0info/manuals/CHAT.html#MOR_Format>

use crate::node_types as kind;
use talkbank_model::ParseOutcome;
use talkbank_model::model::{Mor, MorTier, MorTierType};
use talkbank_model::{
    ErrorCode, ErrorContext, ErrorSink, ParseError, Severity, SourceLocation, Span,
};
use tree_sitter::Node;

use super::item::parse_mor_content;
use crate::parser::tree_parsing::helpers::unexpected_node_error;
use crate::parser::tree_parsing::parser_helpers::{
    check_not_missing, expect_child_at, is_terminator,
};

/// Converts `%mor` tier content into a `MorTier`.
///
/// **Grammar Rule:**
/// ```text
/// mor_dependent_tier: $ => seq(
///     $.mor_tier_prefix,   // Position 0
///     $.tier_sep,          // Position 1
///     $.mor_contents,      // Position 2
///     $.newline            // Position 3
/// )
/// ```
pub fn parse_mor_tier_inner(
    node: Node,
    source: &str,
    tier_type: MorTierType,
    errors: &impl ErrorSink,
) -> MorTier {
    let span = Span::new(node.start_byte() as u32, node.end_byte() as u32);

    // Position 2: mor_contents (required by grammar)
    let mor_contents = match expect_child_at(node, 2, source, errors, "mor_dependent_tier") {
        ParseOutcome::Parsed(contents) => contents,
        ParseOutcome::Rejected => {
            return MorTier::new(tier_type, Vec::new()).with_span(span);
        }
    };

    let child_count = mor_contents.child_count();
    let mut items: Vec<Mor> = Vec::with_capacity(child_count / 2 + 1);
    let mut terminator: Option<smol_str::SmolStr> = None;
    let mut idx = 0;

    while idx < child_count {
        if let Some(child) = mor_contents.child(idx as u32) {
            // CRITICAL: Check for MISSING nodes before processing
            if !check_not_missing(child, source, errors, "mor_contents") {
                idx += 1;
                continue;
            }

            let kind = child.kind();
            match kind {
                kind::MOR_CONTENT => {
                    if let ParseOutcome::Parsed(mor) = parse_mor_content(child, source, errors) {
                        items.push(mor);
                    }
                }
                _ if is_terminator(kind) || kind == kind::TERMINATOR => {
                    // Extract terminator as optional field, not as item
                    match child.utf8_text(source.as_bytes()) {
                        Ok(text) if !text.is_empty() => {
                            terminator = Some(text.into());
                        }
                        Ok(_) => {
                            errors.report(ParseError::new(
                                ErrorCode::TreeParsingError,
                                Severity::Error,
                                SourceLocation::from_offsets(child.start_byte(), child.end_byte()),
                                ErrorContext::new(
                                    source,
                                    child.start_byte()..child.end_byte(),
                                    "mor_terminator",
                                ),
                                "Empty terminator node in %mor contents".to_string(),
                            ));
                        }
                        Err(err) => {
                            errors.report(ParseError::new(
                                ErrorCode::TreeParsingError,
                                Severity::Error,
                                SourceLocation::from_offsets(child.start_byte(), child.end_byte()),
                                ErrorContext::new(
                                    source,
                                    child.start_byte()..child.end_byte(),
                                    "mor_terminator",
                                ),
                                format!("UTF-8 decoding error in %mor terminator: {err}"),
                            ));
                        }
                    }
                }
                kind::WHITESPACES => {}
                _ => {
                    errors.report(unexpected_node_error(child, source, "mor_contents"));
                }
            }
        }
        idx += 1;
    }

    MorTier::new(tier_type, items)
        .with_terminator(terminator)
        .with_span(span)
}
