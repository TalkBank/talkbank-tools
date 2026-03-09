//! Parse isolated main-tier lines via synthetic-file wrapping.
//!
//! CHAT reference anchors:
//! - <https://talkbank.org/0info/manuals/CHAT.html#Main_Tier>

use super::TreeSitterParser;
use super::helpers::{
    MINIMAL_CHAT_PREFIX, MINIMAL_CHAT_SUFFIX, find_main_tier_node_in_tree, parse_tree,
};
use crate::error::{
    ErrorCode, ErrorContext, ParseError, ParseErrors, ParseResult, Severity, SourceLocation,
};
use crate::model::MainTier;
use crate::parser::tree_parsing::main_tier::structure::{
    collect_main_tier_errors, convert_main_tier_node,
};

/// Parse one main tier line into `MainTier`.
pub(super) fn parse_main_tier(parser: &TreeSitterParser, input: &str) -> ParseResult<MainTier> {
    // Wrap in minimal valid CHAT file
    let wrapped = format!("{}{}\n{}", MINIMAL_CHAT_PREFIX, input, MINIMAL_CHAT_SUFFIX);

    // Calculate offset where original input starts in wrapped source
    let offset = MINIMAL_CHAT_PREFIX.len();

    // Parse with tree-sitter
    let tree = parse_tree(parser, input, &wrapped)?;

    // Find main_tier node
    let main_tier_node = find_main_tier_node_in_tree(&tree, &wrapped)?;

    // Check for parse errors within the main tier
    if main_tier_node.has_error() {
        let mut errors = ParseErrors::new();
        collect_main_tier_errors(main_tier_node, &wrapped, input, offset, &mut errors);
        if !errors.is_empty() {
            return Err(errors);
        }
    }

    // Convert to domain model (uses error recovery)
    let errors_sink = crate::error::ErrorCollector::new();
    let main_tier =
        convert_main_tier_node(main_tier_node, &wrapped, input, &errors_sink).into_option();

    // Check if there are actual errors (not just warnings)
    let tier_errors = errors_sink.into_vec();
    let has_actual_errors = tier_errors
        .iter()
        .any(|e| matches!(e.severity, Severity::Error));
    if has_actual_errors || main_tier.is_none() {
        let mut errors = ParseErrors::new();
        errors.errors.extend(tier_errors);
        if main_tier.is_none() && errors.is_empty() {
            errors.push(ParseError::new(
                ErrorCode::MissingMainTier,
                Severity::Error,
                SourceLocation::from_offsets(0, input.len()),
                ErrorContext::new(input, 0..input.len(), input),
                "Failed to build main tier from parse tree",
            ));
        }
        return Err(errors);
    }

    match main_tier {
        Some(main_tier) => Ok(main_tier),
        None => {
            let mut errors = ParseErrors::new();
            errors.push(ParseError::new(
                ErrorCode::MissingMainTier,
                Severity::Error,
                SourceLocation::from_offsets(0, input.len()),
                ErrorContext::new(input, 0..input.len(), input),
                "Failed to build main tier from parse tree",
            ));
            Err(errors)
        }
    }
}
