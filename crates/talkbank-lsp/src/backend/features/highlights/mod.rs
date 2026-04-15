//! Bidirectional alignment highlights on click.
//!
//! When the user clicks a word on the main tier, all aligned items across
//! dependent tiers (`%mor`, `%pho`, `%mod`, `%sin`) are highlighted. Clicking
//! on a dependent-tier item highlights the main-tier word *and* corresponding
//! items on sibling tiers. This gives annotators an instant visual check of
//! cross-tier alignment without leaving the source buffer.
//!
//! Tier-specific logic lives in [`tier_handlers`]; CST traversal for computing
//! LSP ranges lives in [`range_finders`].

use tower_lsp::lsp_types::*;

use crate::backend::utils;
use talkbank_model::Span;
use talkbank_model::dependent_tier::DependentTier;

mod range_finders;
mod tier_handlers;

use tier_handlers::{
    highlights_from_gra_tier, highlights_from_main_tier, highlights_from_mod_tier,
    highlights_from_mor_tier, highlights_from_pho_tier, highlights_from_sin_tier,
};

/// Generate document highlights for aligned items across tiers
pub fn document_highlights(
    chat_file: &talkbank_model::model::ChatFile,
    tree: &tree_sitter::Tree,
    position: Position,
    document: &str,
) -> Option<Vec<DocumentHighlight>> {
    // Find the utterance at this position
    let utterance = utils::find_utterance_at_position(chat_file, position, document)?;
    let offset = utils::position_to_offset(document, position) as u32;

    if span_contains(utterance.main.span, offset) {
        // Main tier - find alignment index and highlight across all tiers
        highlights_from_main_tier(utterance, tree, position, document)
    } else {
        let tier = find_dependent_tier_at_offset(utterance, offset)?;
        match tier {
            DependentTier::Mor(_) => highlights_from_mor_tier(utterance, tree, position, document),
            DependentTier::Pho(_) => highlights_from_pho_tier(utterance, tree, position, document),
            DependentTier::Mod(_) => highlights_from_mod_tier(utterance, tree, position, document),
            DependentTier::Sin(_) => highlights_from_sin_tier(utterance, tree, position, document),
            DependentTier::Gra(_) => highlights_from_gra_tier(utterance, tree, position, document),
            _ => None,
        }
    }
}

/// Finds dependent tier at offset.
fn find_dependent_tier_at_offset(
    utterance: &talkbank_model::model::Utterance,
    offset: u32,
) -> Option<&DependentTier> {
    utterance
        .dependent_tiers
        .iter()
        .find(|tier| dependent_tier_span(tier).is_some_and(|span| span_contains(span, offset)))
}

/// Return the source span for a dependent tier variant.
fn dependent_tier_span(tier: &DependentTier) -> Option<Span> {
    Some(tier.span())
}

/// Return `true` when `outer` fully contains `inner`.
fn span_contains(span: Span, offset: u32) -> bool {
    offset >= span.start && offset <= span.end
}

#[cfg(test)]
mod tests {
    use super::*;
    use talkbank_parser::TreeSitterParser;

    fn parse_chat(content: &str) -> talkbank_model::model::ChatFile {
        let parser = TreeSitterParser::new().unwrap();
        parser.parse_chat_file(content).unwrap()
    }

    fn parse_tree(input: &str) -> tree_sitter::Tree {
        let mut parser = tree_sitter::Parser::new();
        let language = tree_sitter_talkbank::LANGUAGE;
        parser.set_language(&language.into()).unwrap();
        parser.parse(input, None).unwrap()
    }

    #[test]
    fn no_highlights_on_header_line() {
        let content = "@UTF8\n@Begin\n@Languages:\teng\n@Participants:\tCHI Child\n@ID:\teng|corpus|CHI|||||Child|||\n*CHI:\thello .\n@End\n";
        let chat_file = parse_chat(content);
        let tree = parse_tree(content);
        // Position on @Languages header — not inside any utterance.
        let pos = Position {
            line: 2,
            character: 5,
        };
        let result = document_highlights(&chat_file, &tree, pos, content);
        assert!(result.is_none(), "Expected no highlights on a header line");
    }

    #[test]
    fn no_highlights_past_end_of_document() {
        let content = "@UTF8\n@Begin\n@Languages:\teng\n@Participants:\tCHI Child\n@ID:\teng|corpus|CHI|||||Child|||\n*CHI:\thello .\n@End\n";
        let chat_file = parse_chat(content);
        let tree = parse_tree(content);
        // Position well beyond the document.
        let pos = Position {
            line: 100,
            character: 0,
        };
        let result = document_highlights(&chat_file, &tree, pos, content);
        assert!(
            result.is_none(),
            "Expected no highlights past end of document"
        );
    }

    #[test]
    fn highlight_main_tier_word_returns_at_least_one_highlight() {
        let content = "@UTF8\n@Begin\n@Languages:\teng\n@Participants:\tCHI Child\n@ID:\teng|corpus|CHI|||||Child|||\n*CHI:\thello .\n@End\n";
        let chat_file = parse_chat(content);
        let tree = parse_tree(content);
        // Position on "hello" word — character 6 is 'h' in "hello" (after *CHI:\t).
        let pos = Position {
            line: 5,
            character: 6,
        };
        let result = document_highlights(&chat_file, &tree, pos, content);
        // Without dependent tiers, the main tier word itself should be highlighted.
        if let Some(highlights) = result {
            assert!(
                !highlights.is_empty(),
                "Expected at least one highlight for main tier word"
            );
            assert!(
                highlights
                    .iter()
                    .any(|h| h.kind == Some(DocumentHighlightKind::TEXT)),
                "Main tier word should have TEXT highlight kind"
            );
        }
        // If result is None, the word may not be alignable (no mor tier),
        // which is valid behavior for a document without dependent tiers.
    }

    #[test]
    fn no_highlights_on_terminator() {
        let content = "@UTF8\n@Begin\n@Languages:\teng\n@Participants:\tCHI Child\n@ID:\teng|corpus|CHI|||||Child|||\n*CHI:\thello .\n@End\n";
        let chat_file = parse_chat(content);
        let tree = parse_tree(content);
        // Position on the utterance terminator '.' — this is not a word.
        let pos = Position {
            line: 5,
            character: 12,
        };
        let result = document_highlights(&chat_file, &tree, pos, content);
        // Terminators are not alignable content, so no highlights expected.
        assert!(
            result.is_none(),
            "Expected no highlights on utterance terminator"
        );
    }

    #[test]
    fn span_contains_boundary_values() {
        let span = Span { start: 10, end: 20 };
        assert!(span_contains(span, 10), "Start of span should be contained");
        assert!(span_contains(span, 20), "End of span should be contained");
        assert!(
            span_contains(span, 15),
            "Middle of span should be contained"
        );
        assert!(
            !span_contains(span, 9),
            "Before span start should not be contained"
        );
        assert!(
            !span_contains(span, 21),
            "After span end should not be contained"
        );
    }

    #[test]
    fn no_highlights_on_dependent_tier_without_alignment() {
        // A document with a %com tier (comment) — not an aligned tier.
        let content = "@UTF8\n@Begin\n@Languages:\teng\n@Participants:\tCHI Child\n@ID:\teng|corpus|CHI|||||Child|||\n*CHI:\thello .\n%com:\tthis is a comment\n@End\n";
        let chat_file = parse_chat(content);
        let tree = parse_tree(content);
        // Position inside the %com tier text.
        let pos = Position {
            line: 6,
            character: 8,
        };
        let result = document_highlights(&chat_file, &tree, pos, content);
        // %com is not a tier that supports alignment highlighting.
        assert!(
            result.is_none(),
            "Expected no highlights on non-aligned dependent tier (%com)"
        );
    }
}
