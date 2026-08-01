//! Contiguous span grouping for secondary dispatch.
//!
//! Groups @s words into per-utterance contiguous spans by target language,
//! preserving within-span context for Stanza (e.g., "los niños" stays
//! together rather than being sent as two isolated words).

use talkbank_model::model::LanguageCode;
use talkbank_model::validation::LanguageResolution;

/// A contiguous span of @s words sharing the same target language.
///
/// Used by `group_l2_spans` for per-utterance analysis of @s patterns.
#[derive(Debug, Clone, PartialEq)]
pub struct L2Span {
    /// Indices into the utterance's word list (0-based).
    pub word_indices: Vec<usize>,
    /// Resolved target language for secondary dispatch.
    pub target_lang: LanguageCode,
    /// Word texts extracted for the secondary batch. Each is a
    /// provenance-sealed `ChatCleanedText` derived from the upstream
    /// typed AST.
    pub words: Vec<talkbank_model::ChatCleanedText>,
}

/// Resolve the dispatch target language from a `LanguageResolution`.
fn resolve_dispatch_lang(resolution: &LanguageResolution) -> Option<LanguageCode> {
    resolution.languages().first().cloned()
}

/// Group @s words into contiguous spans by target language.
///
/// Consecutive @s words with the same resolved target language are merged
/// into a single [`L2Span`]. Non-contiguous @s words or words with
/// different target languages produce separate spans.
///
/// Words with `Unresolved` language resolution are skipped (they will
/// fall back to `L2|xxx`).
pub fn group_l2_spans(
    special_forms: &[(
        Option<talkbank_model::model::FormType>,
        Option<LanguageResolution>,
    )],
    word_texts: &[talkbank_model::ChatCleanedText],
) -> Vec<L2Span> {
    let mut spans: Vec<L2Span> = Vec::new();

    for (idx, (_form_type, lang_res)) in special_forms.iter().enumerate() {
        let target_lang = match lang_res {
            Some(res) => match resolve_dispatch_lang(res) {
                Some(lang) => lang,
                None => continue,
            },
            None => continue,
        };

        let extends = spans.last().is_some_and(|last: &L2Span| {
            last.target_lang == target_lang
                && last
                    .word_indices
                    .last()
                    .is_some_and(|&prev| prev + 1 == idx)
        });

        if extends {
            if let Some(last) = spans.last_mut() {
                last.word_indices.push(idx);
                last.words.push(word_texts[idx].clone());
            }
        } else {
            spans.push(L2Span {
                word_indices: vec![idx],
                target_lang,
                words: vec![word_texts[idx].clone()],
            });
        }
    }

    spans
}
