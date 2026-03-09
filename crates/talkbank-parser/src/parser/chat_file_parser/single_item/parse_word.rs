//! Parse isolated CHAT words via synthetic-file wrapping.
//!
//! CHAT reference anchors:
//! - <https://talkbank.org/0info/manuals/CHAT.html#Words>

use super::TreeSitterParser;
use super::helpers::{
    MINIMAL_CHAT_PREFIX, MINIMAL_CHAT_SUFFIX, find_main_tier_node_in_tree, parse_tree,
};
use crate::error::{
    ErrorCode, ErrorContext, ParseError, ParseErrors, ParseResult, Severity, SourceLocation,
};
use crate::model::{UtteranceContent, Word};
use crate::parser::tree_parsing::main_tier::structure::{
    collect_main_tier_errors, convert_main_tier_node,
};

/// Parse a single word token and project it from wrapped main-tier content.
pub(super) fn parse_word(parser: &TreeSitterParser, input: &str) -> ParseResult<Word> {
    // Wrap word in minimal valid CHAT file with one word utterance
    let wrapped = format!(
        "{}*CHI:\t{} .\n{}",
        MINIMAL_CHAT_PREFIX, input, MINIMAL_CHAT_SUFFIX
    );

    // Calculate offset where original input starts in wrapped source
    let offset = MINIMAL_CHAT_PREFIX.len() + "*CHI:\t".len();

    // Parse with tree-sitter
    let tree = parse_tree(parser, input, &wrapped)?;

    // Find the main_tier node
    let main_tier_node = find_main_tier_node_in_tree(&tree, &wrapped)?;

    // Check for parse errors
    if main_tier_node.has_error() {
        let mut errors = ParseErrors::new();
        collect_main_tier_errors(main_tier_node, &wrapped, input, offset, &mut errors);
        if !errors.is_empty() {
            return Err(errors);
        }
    }

    // Convert the main_tier node to MainTier model
    let errors_sink = crate::error::ErrorCollector::new();
    let main_tier =
        convert_main_tier_node(main_tier_node, &wrapped, input, &errors_sink).into_option();

    // Check if there are actual errors
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

    let main_tier = match main_tier {
        Some(main_tier) => main_tier,
        None => {
            let mut errors = ParseErrors::new();
            errors.push(ParseError::new(
                ErrorCode::MissingMainTier,
                Severity::Error,
                SourceLocation::from_offsets(0, input.len()),
                ErrorContext::new(input, 0..input.len(), input),
                "Failed to build main tier from parse tree",
            ));
            return Err(errors);
        }
    };

    if main_tier.content.content.is_empty() {
        let mut errors = ParseErrors::new();
        errors.push(ParseError::new(
            ErrorCode::InvalidWordFormat,
            Severity::Error,
            SourceLocation::from_offsets(0, input.len()),
            ErrorContext::new(input, 0..input.len(), input),
            "No words parsed from input",
        ));
        return Err(errors);
    }

    // Extract the word from the first content element
    match &main_tier.content.content[0] {
        UtteranceContent::Word(word) => Ok((**word).clone()),
        UtteranceContent::AnnotatedWord(annotated) => Ok(annotated.inner.clone()),
        UtteranceContent::ReplacedWord(replaced) => Ok(replaced.word.clone()),
        _ => {
            let mut errors = ParseErrors::new();
            errors.push(ParseError::new(
                ErrorCode::InvalidWordFormat,
                Severity::Error,
                SourceLocation::from_offsets(0, input.len()),
                ErrorContext::new(input, 0..input.len(), input),
                "First content element is not a word",
            ));
            Err(errors)
        }
    }
}
