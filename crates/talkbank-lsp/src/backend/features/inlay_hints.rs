//! Inlay hint annotations for alignment mismatches near `%mor`/`%gra`.
//!
//! Generates inline annotations that the editor renders alongside the source
//! text, surfacing alignment count mismatches (e.g. 3 main-tier words vs 4
//! `%mor` items) without requiring the user to hover or run a separate command.
//! Each hint carries a tooltip explaining the mismatch.
//!
//! # Related CHAT Manual Sections
//!
//! - <https://talkbank.org/0info/manuals/CHAT.html#Dependent_Tiers>

use tower_lsp::lsp_types::*;

use crate::backend::utils::LineIndex;

/// Generates alignment hints.
pub fn generate_alignment_hints(
    chat_file: &talkbank_model::model::ChatFile,
    text: &str,
    range: Range,
) -> Vec<InlayHint> {
    let mut hints = Vec::new();
    let index = LineIndex::new(text);

    for utterance in chat_file.utterances() {
        let main_line = index
            .offset_to_position(text, utterance.main.span.start)
            .line;
        if main_line < range.start.line || main_line > range.end.line {
            continue;
        }

        // Check for %mor tier and show alignment hints
        if let Some(mor_metadata) = utterance.alignments.as_ref().and_then(|m| m.mor.as_ref()) {
            // Show alignment count hint at end of main tier line
            let mor_tier = utterance.mor_tier();
            let mor_count = match mor_tier {
                Some(tier) => tier.items.len(),
                None => 0,
            };
            let main_count = mor_metadata.pairs.len();

            // Only show hint if counts don't match (indicating potential alignment issue)
            if mor_count != main_count {
                let main_end = index.offset_to_position(text, utterance.main.span.end);
                hints.push(InlayHint {
                    position: main_end,
                    label: InlayHintLabel::String(format!(
                        " [alignment: {} main ↔ {} mor]",
                        main_count, mor_count
                    )),
                    kind: Some(InlayHintKind::PARAMETER),
                    text_edits: None,
                    tooltip: Some(InlayHintTooltip::String(
                        "Main tier and %mor tier item counts differ. This may indicate an alignment error.".to_string()
                    )),
                    padding_left: Some(true),
                    padding_right: None,
                    data: None,
                });
            }
        }

        // Check for %gra tier alignment
        if utterance
            .alignments
            .as_ref()
            .and_then(|m| m.gra.as_ref())
            .is_some()
            && let Some(gra_tier) = utterance.gra_tier()
        {
            let gra_line = index.offset_to_position(text, gra_tier.span.start).line;
            if gra_line < range.start.line || gra_line > range.end.line {
                continue;
            }

            let gra_count = gra_tier.relations.len();
            let mor_count = match utterance.mor_tier() {
                Some(tier) => tier.items.len(),
                None => 0,
            };

            // Show hint if %gra doesn't align with %mor
            if gra_count != mor_count {
                let gra_end = index.offset_to_position(text, gra_tier.span.end);
                hints.push(InlayHint {
                    position: gra_end,
                    label: InlayHintLabel::String(format!(
                        " [alignment: {} gra ↔ {} mor]",
                        gra_count, mor_count
                    )),
                    kind: Some(InlayHintKind::PARAMETER),
                    text_edits: None,
                    tooltip: Some(InlayHintTooltip::String(
                        "%gra and %mor tier item counts differ. Each %mor item should have a corresponding %gra relation.".to_string()
                    )),
                    padding_left: Some(true),
                    padding_right: None,
                    data: None,
                });
            }
        }
    }

    hints
}

#[cfg(test)]
mod tests {
    use super::*;
    use talkbank_model::model::Line;
    use talkbank_parser::TreeSitterParser;

    fn parse_and_align(input: &str) -> talkbank_model::model::ChatFile {
        let parser = TreeSitterParser::new().unwrap();
        let mut chat_file = parser.parse_chat_file(input).unwrap();
        for line in &mut chat_file.lines {
            if let Line::Utterance(utterance) = line {
                utterance.compute_alignments_default();
            }
        }
        chat_file
    }

    fn full_range() -> Range {
        Range {
            start: Position {
                line: 0,
                character: 0,
            },
            end: Position {
                line: 1000,
                character: 0,
            },
        }
    }

    #[test]
    fn no_hints_when_aligned() {
        let input = "@UTF8\n@Begin\n@Languages:\teng\n@Participants:\tCHI Child\n@ID:\teng|corpus|CHI|||||Child|||\n*CHI:\thello world .\n%mor:\tn|hello n|world .\n@End\n";
        let chat_file = parse_and_align(input);
        let hints = generate_alignment_hints(&chat_file, input, full_range());
        // When counts match, no hint should be generated
        assert!(hints.is_empty());
    }

    #[test]
    fn hint_on_mor_count_mismatch() {
        // 3 main-tier words but only 2 mor items → mismatch
        let input = "@UTF8\n@Begin\n@Languages:\teng\n@Participants:\tCHI Child\n@ID:\teng|corpus|CHI|||||Child|||\n*CHI:\thello big world .\n%mor:\tn|hello n|world .\n@End\n";
        let chat_file = parse_and_align(input);
        let hints = generate_alignment_hints(&chat_file, input, full_range());
        assert_eq!(hints.len(), 1);
        if let InlayHintLabel::String(label) = &hints[0].label {
            assert!(label.contains("3 main"));
            assert!(label.contains("2 mor"));
        } else {
            panic!("Expected string label");
        }
    }

    #[test]
    fn hints_filtered_by_range() {
        let input = "@UTF8\n@Begin\n@Languages:\teng\n@Participants:\tCHI Child\n@ID:\teng|corpus|CHI|||||Child|||\n*CHI:\thello world .\n%mor:\tn|hello n|world adj|big .\n@End\n";
        let chat_file = parse_and_align(input);
        // Request only lines 0-1 — utterance is further down
        let narrow = Range {
            start: Position {
                line: 0,
                character: 0,
            },
            end: Position {
                line: 1,
                character: 0,
            },
        };
        let hints = generate_alignment_hints(&chat_file, input, narrow);
        assert!(hints.is_empty()); // Utterance is outside the requested range
    }
}
