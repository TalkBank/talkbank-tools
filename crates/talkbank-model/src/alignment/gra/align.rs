//! `%mor` chunk to `%gra` relation alignment.
//!
//! References:
//! - <https://talkbank.org/0info/manuals/CHAT.html#Morphological_Tier>
//! - <https://talkbank.org/0info/manuals/CHAT.html#GrammaticalRelations_Tier>

use crate::model::{GraTier, MorTier};
use crate::{ErrorCode, ErrorContext, ParseError, Severity, SourceLocation};

use super::super::format::format_positional_mismatch;
use super::super::helpers::{AlignableItem, to_chat_display_string as to_string};
use super::types::{GraAlignment, GraAlignmentPair};

/// Align %mor tier to %gra tier
///
/// Performs 1-1 alignment validation between `%mor` chunks and `%gra` relations.
/// Continues alignment even on mismatch, creating error placeholders.
///
/// # Important: Chunks vs Items
///
/// %gra aligns with %mor **chunks**, not items!
/// - A single %mor item can produce multiple chunks due to clitics
/// - Example: `pro|it~v|be&PRES` → 2 chunks (pre-clitic + main)
/// - Use `MorTier::count_chunks()` to get the correct count
///
/// # Algorithm
///
/// 1. Count %mor tier chunks (including clitics)
/// 2. Count %gra tier relations
/// 3. Pair up indices 1-1 up to min(mor_chunks, gra_relations)
/// 4. Create error placeholders for extras
/// 5. Return alignment with collected errors
pub fn align_mor_to_gra(mor: &MorTier, gra: &GraTier) -> GraAlignment {
    let mut alignment = GraAlignment::new();

    // Count chunks (NOT items!) - clitics create additional chunks
    let mor_chunk_count = mor.count_chunks();
    let gra_count = gra.len();

    // Create 1-1 pairs for the common range
    let min_len = mor_chunk_count.min(gra_count);
    for i in 0..min_len {
        alignment = alignment.with_pair(GraAlignmentPair::new(Some(i), Some(i)));
    }

    // Handle length mismatch
    if mor_chunk_count > gra_count {
        // %mor has more chunks - %gra tier too short
        let mor_items = extract_mor_chunk_items(mor);
        let gra_items = extract_gra_relation_items(gra);
        let detailed_message =
            format_positional_mismatch("%mor chunks", "%gra relations", &mor_items, &gra_items);

        let mut error = ParseError::new(
            ErrorCode::GraInvalidWordIndex,
            Severity::Error,
            SourceLocation::new(mor.span),
            ErrorContext::new("", mor.span, ""),
            detailed_message,
        )
        .with_suggestion("Each %mor chunk (including pre/post-clitics) needs a %gra relation");
        if !mor.span.is_dummy() {
            error
                .labels
                .push(crate::ErrorLabel::new(mor.span, "%mor tier"));
        }
        if !gra.span.is_dummy() {
            error
                .labels
                .push(crate::ErrorLabel::new(gra.span, "%gra tier"));
        }

        alignment = alignment.with_error(error);

        // Add placeholders for extra %mor chunks
        for i in gra_count..mor_chunk_count {
            alignment = alignment.with_pair(GraAlignmentPair::new(Some(i), None));
        }
    } else if gra_count > mor_chunk_count {
        // %gra tier has more relations - %mor tier too short
        let mor_items = extract_mor_chunk_items(mor);
        let gra_items = extract_gra_relation_items(gra);
        let detailed_message =
            format_positional_mismatch("%mor chunks", "%gra relations", &mor_items, &gra_items);

        let mut error = ParseError::new(
            ErrorCode::GraInvalidHeadIndex,
            Severity::Error,
            SourceLocation::new(mor.span),
            ErrorContext::new("", mor.span, ""),
            detailed_message,
        )
        .with_suggestion("Remove extra %gra relations or add corresponding %mor chunks");
        if !mor.span.is_dummy() {
            error
                .labels
                .push(crate::ErrorLabel::new(mor.span, "%mor tier"));
        }
        if !gra.span.is_dummy() {
            error
                .labels
                .push(crate::ErrorLabel::new(gra.span, "%gra tier"));
        }

        alignment = alignment.with_error(error);

        // Add placeholders for extra %gra relations
        for i in mor_chunk_count..gra_count {
            alignment = alignment.with_pair(GraAlignmentPair::new(None, Some(i)));
        }
    }

    // Validate explicit relation indices when cardinalities match.
    // If lengths already mismatch, length diagnostics above are clearer and non-duplicative.
    if mor_chunk_count == gra_count {
        let max_index = mor_chunk_count;
        for relation in gra.relations.iter() {
            if relation.index == 0 || relation.index > max_index {
                alignment = alignment.with_error(
                    ParseError::new(
                        ErrorCode::GraInvalidWordIndex,
                        Severity::Error,
                        SourceLocation::new(gra.span),
                        ErrorContext::new("", gra.span, ""),
                        format!(
                            "%gra relation word index {} is out of bounds for %mor chunk count {}",
                            relation.index, mor_chunk_count
                        ),
                    )
                    .with_suggestion(format!(
                        "Use word indices in range 1..={} (0 is reserved for ROOT head only)",
                        mor_chunk_count
                    )),
                );
            }

            if relation.head > max_index {
                alignment = alignment.with_error(
                    ParseError::new(
                        ErrorCode::GraInvalidHeadIndex,
                        Severity::Error,
                        SourceLocation::new(gra.span),
                        ErrorContext::new("", gra.span, ""),
                        format!(
                            "%gra relation head index {} is out of bounds for %mor chunk count {}",
                            relation.head, mor_chunk_count
                        ),
                    )
                    .with_suggestion(format!(
                        "Use head indices in range 0..={} (0 marks ROOT)",
                        mor_chunk_count
                    )),
                );
            }
        }
    }

    alignment
}

/// Extract %mor chunks as individual AlignableItems for diagnostic display.
///
/// Expands each `Mor` item into its constituent chunks (main + post-clitics),
/// plus the terminator if present. Each chunk becomes one item in the output.
fn extract_mor_chunk_items(mor: &MorTier) -> Vec<AlignableItem> {
    let mut items = Vec::new();
    for mor_item in mor.items.iter() {
        items.push(AlignableItem {
            text: to_string(&mor_item.main),
            description: None,
        });
        for clitic in &mor_item.post_clitics {
            items.push(AlignableItem {
                text: to_string(clitic),
                description: Some("post-clitic".to_string()),
            });
        }
    }
    if let Some(term) = &mor.terminator {
        items.push(AlignableItem {
            text: term.to_string(),
            description: Some("terminator".to_string()),
        });
    }
    items
}

/// Extract %gra relations as AlignableItems for diagnostic display.
fn extract_gra_relation_items(gra: &GraTier) -> Vec<AlignableItem> {
    gra.relations
        .iter()
        .map(|rel| AlignableItem {
            text: to_string(rel),
            description: None,
        })
        .collect()
}
