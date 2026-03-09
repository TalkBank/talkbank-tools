//! Alignment sidecar document builder.
//!
//! Produces a JSON sidecar describing per-utterance alignment data
//! (main-tier words, `%mor`/`%gra` links, timing) for the VS Code extension's
//! alignment visualization panel.

use serde::Serialize;
use talkbank_model::Span;
use talkbank_model::model::{ChatFile, UtteranceContent};
use tower_lsp::lsp_types::*;

use crate::alignment::finders::get_alignable_content_by_index;
use crate::alignment::formatters::format_content_item;
use crate::backend::utils;

// ---------------------------------------------------------------------------
// Sidecar types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize)]
pub(crate) struct AlignmentSidecarDocument {
    pub schema_version: u8,
    pub uri: String,
    pub utterances: Vec<AlignmentSidecarUtterance>,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct AlignmentSidecarUtterance {
    pub utterance_index: usize,
    pub speaker: String,
    pub range: Option<Range>,
    pub timing: Option<SidecarTiming>,
    pub main_units: Vec<AlignmentSidecarMainUnit>,
    pub alignments: AlignmentSidecarLinks,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct AlignmentSidecarMainUnit {
    pub alignment_index: usize,
    pub text: String,
    pub range: Option<Range>,
    pub word_id: Option<String>,
    pub inline_timing: Option<SidecarTiming>,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct AlignmentSidecarLinks {
    pub mor: Vec<AlignmentPairView>,
    pub gra: Vec<AlignmentPairView>,
    pub pho: Vec<AlignmentPairView>,
    pub wor: Vec<AlignmentPairView>,
    pub mod_: Vec<AlignmentPairView>,
    pub sin: Vec<AlignmentPairView>,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct AlignmentPairView {
    pub source_index: Option<usize>,
    pub target_index: Option<usize>,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct SidecarTiming {
    pub start_ms: u64,
    pub end_ms: u64,
    pub skip: bool,
}

// ---------------------------------------------------------------------------
// Builder
// ---------------------------------------------------------------------------

pub(crate) fn build_alignment_sidecar(
    uri: &Url,
    text: &str,
    chat_file: &ChatFile,
) -> AlignmentSidecarDocument {
    let index = utils::LineIndex::new(text);
    let utterances = chat_file
        .utterances()
        .enumerate()
        .map(|(utterance_index, utterance)| {
            let main_unit_count = utterance
                .alignments
                .as_ref()
                .map(|a| a.units.main_mor.len())
                .unwrap_or_else(|| count_main_units(utterance));

            let mut main_units = Vec::new();
            for idx in 0..main_unit_count {
                if let Some(content) =
                    get_alignable_content_by_index(&utterance.main.content.content, idx)
                {
                    let (word_id, inline_timing) = content_word_metadata(content);
                    main_units.push(AlignmentSidecarMainUnit {
                        alignment_index: idx,
                        text: format_content_item(content),
                        range: content_span(content)
                            .and_then(|span| span_to_range(span, text, &index)),
                        word_id,
                        inline_timing,
                    });
                }
            }

            let alignments = utterance.alignments.as_ref();
            let links = AlignmentSidecarLinks {
                mor: alignments
                    .and_then(|a| a.mor.as_ref())
                    .map(|a| collect_alignment_pairs(&a.pairs))
                    .unwrap_or_default(),
                gra: alignments
                    .and_then(|a| a.gra.as_ref())
                    .map(|a| collect_gra_alignment_pairs(&a.pairs))
                    .unwrap_or_default(),
                pho: alignments
                    .and_then(|a| a.pho.as_ref())
                    .map(|a| collect_alignment_pairs(&a.pairs))
                    .unwrap_or_default(),
                wor: alignments
                    .and_then(|a| a.wor.as_ref())
                    .map(|a| collect_alignment_pairs(&a.pairs))
                    .unwrap_or_default(),
                mod_: alignments
                    .and_then(|a| a.mod_.as_ref())
                    .map(|a| collect_alignment_pairs(&a.pairs))
                    .unwrap_or_default(),
                sin: alignments
                    .and_then(|a| a.sin.as_ref())
                    .map(|a| collect_alignment_pairs(&a.pairs))
                    .unwrap_or_default(),
            };

            AlignmentSidecarUtterance {
                utterance_index,
                speaker: utterance.main.speaker.to_string(),
                range: span_to_range(utterance.main.span, text, &index),
                timing: utterance.main.content.bullet.as_ref().map(bullet_to_timing),
                main_units,
                alignments: links,
            }
        })
        .collect();

    AlignmentSidecarDocument {
        schema_version: 1,
        uri: uri.to_string(),
        utterances,
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn count_main_units(utterance: &talkbank_model::model::Utterance) -> usize {
    let mut count = 0usize;
    while get_alignable_content_by_index(&utterance.main.content.content, count).is_some() {
        count += 1;
    }
    count
}

fn collect_alignment_pairs(
    pairs: &[talkbank_model::alignment::AlignmentPair],
) -> Vec<AlignmentPairView> {
    pairs
        .iter()
        .map(|pair| AlignmentPairView {
            source_index: pair.source_index,
            target_index: pair.target_index,
        })
        .collect()
}

fn collect_gra_alignment_pairs(
    pairs: &[talkbank_model::alignment::GraAlignmentPair],
) -> Vec<AlignmentPairView> {
    pairs
        .iter()
        .map(|pair| AlignmentPairView {
            source_index: pair.mor_chunk_index,
            target_index: pair.gra_index,
        })
        .collect()
}

fn span_to_range(span: Span, text: &str, index: &utils::LineIndex) -> Option<Range> {
    if span.is_dummy() {
        return None;
    }
    Some(Range {
        start: index.offset_to_position(text, span.start),
        end: index.offset_to_position(text, span.end),
    })
}

fn bullet_to_timing(bullet: &talkbank_model::model::Bullet) -> SidecarTiming {
    SidecarTiming {
        start_ms: bullet.timing.start_ms,
        end_ms: bullet.timing.end_ms,
        skip: bullet.skip,
    }
}

fn content_word_metadata(content: &UtteranceContent) -> (Option<String>, Option<SidecarTiming>) {
    match content {
        UtteranceContent::Word(word) => (
            word.word_id.as_ref().map(std::string::ToString::to_string),
            word.inline_bullet.as_ref().map(bullet_to_timing),
        ),
        UtteranceContent::AnnotatedWord(annotated) => (
            annotated
                .inner
                .word_id
                .as_ref()
                .map(std::string::ToString::to_string),
            annotated.inner.inline_bullet.as_ref().map(bullet_to_timing),
        ),
        UtteranceContent::ReplacedWord(replaced) => (
            replaced
                .word
                .word_id
                .as_ref()
                .map(std::string::ToString::to_string),
            replaced.word.inline_bullet.as_ref().map(bullet_to_timing),
        ),
        _ => (None, None),
    }
}

/// Return the source span for a main-tier content item.
fn content_span(content: &UtteranceContent) -> Option<Span> {
    match content {
        UtteranceContent::Word(word) => Some(word.span),
        UtteranceContent::AnnotatedWord(annotated) => Some(annotated.span),
        UtteranceContent::ReplacedWord(replaced) => Some(replaced.span),
        UtteranceContent::Group(group) => Some(group.span),
        UtteranceContent::AnnotatedGroup(annotated) => Some(annotated.span),
        UtteranceContent::PhoGroup(_) => None,
        UtteranceContent::SinGroup(_) => None,
        UtteranceContent::Quotation(_) => None,
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use talkbank_model::model::Line;
    use talkbank_parser::TreeSitterParser;

    fn parse_chat_with_alignments(input: &str) -> std::result::Result<ChatFile, String> {
        let parser =
            TreeSitterParser::new().map_err(|err| format!("Failed to create parser: {err}"))?;
        let mut chat_file = parser
            .parse_chat_file(input)
            .map_err(|err| format!("Failed to parse chat file: {err}"))?;
        for line in &mut chat_file.lines {
            if let Line::Utterance(utterance) = line {
                utterance.compute_alignments_default();
            }
        }
        Ok(chat_file)
    }

    #[test]
    fn test_build_alignment_sidecar_with_alignments() -> std::result::Result<(), String> {
        let input = "@UTF8\n@Begin\n*CHI:\tmore cookie .\n%mor:\tqn|more n|cookie .\n%gra:\t1|2|DET 2|0|ROOT 3|2|OBJ 4|2|PUNCT\n@End\n";
        let chat_file = parse_chat_with_alignments(input)?;
        let uri = Url::parse("file:///tmp/sample.cha").map_err(|err| err.to_string())?;

        let sidecar = build_alignment_sidecar(&uri, input, &chat_file);
        assert_eq!(sidecar.schema_version, 1);
        assert_eq!(sidecar.utterances.len(), 1);

        let utterance = &sidecar.utterances[0];
        assert_eq!(utterance.speaker, "CHI");
        assert!(utterance.range.is_some());
        assert!(!utterance.main_units.is_empty());
        assert!(!utterance.alignments.mor.is_empty());
        assert!(!utterance.alignments.gra.is_empty());
        Ok(())
    }

    #[test]
    fn test_build_alignment_sidecar_without_alignments() -> std::result::Result<(), String> {
        let input = "@UTF8\n@Begin\n*CHI:\tmore cookie .\n@End\n";
        let parser =
            TreeSitterParser::new().map_err(|err| format!("Failed to create parser: {err}"))?;
        let chat_file = parser
            .parse_chat_file(input)
            .map_err(|err| format!("Failed to parse chat file: {err}"))?;
        let uri = Url::parse("file:///tmp/no-align.cha").map_err(|err| err.to_string())?;

        let sidecar = build_alignment_sidecar(&uri, input, &chat_file);
        assert_eq!(sidecar.utterances.len(), 1);
        assert!(!sidecar.utterances[0].main_units.is_empty());
        assert!(sidecar.utterances[0].alignments.mor.is_empty());
        assert!(sidecar.utterances[0].alignments.gra.is_empty());
        Ok(())
    }
}
