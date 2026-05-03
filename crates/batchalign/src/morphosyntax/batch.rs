//! Secondary L2 dispatch for @s words.

use std::collections::HashMap;

use super::worker::infer_batch;
use crate::chat_ops::morphosyntax_ops::BatchItemWithPosition;
use crate::chat_ops::morphosyntax_ops::l2;
use crate::chat_ops::{ChatFile, LanguageCode};
use crate::pipeline::PipelineServices;

// ---------------------------------------------------------------------------
// Experimental: secondary L2 dispatch for @s words
// ---------------------------------------------------------------------------

/// Dispatch @s words to secondary language workers and splice results back.
///
/// This function:
/// 1. Groups deferred positions into contiguous spans by target language
/// 2. For each supported target language, builds a minimal `BatchItemWithPosition`
///    and dispatches it to a secondary Stanza worker via `infer_batch`
/// 3. Runs the structural merge algorithm (primary structural + secondary lexical)
/// 4. Splices merged morphology back into the ChatFile, replacing L2|xxx
/// 5. Falls back to L2|xxx for unsupported languages or dispatch failures
pub(crate) async fn dispatch_secondary_l2(
    chat_file: &mut ChatFile,
    deferred: &[l2::L2DeferredPosition],
    services: PipelineServices<'_>,
    filename: &str,
) {
    use crate::chat_ops::morphosyntax_ops::MorphosyntaxBatchItem;

    // Pre-extract word texts once per unique utterance.
    let word_cache = build_word_text_cache(chat_file, deferred);

    // Group into per-utterance contiguous spans. Each span becomes one
    // "sentence" for the secondary Stanza model, preserving within-span
    // context (e.g., "los niños" stays together, not sent as two isolated words).
    let dispatch_spans = l2::group_deferred_into_dispatch_spans(deferred, &word_cache);

    // Group spans by target language for batched dispatch.
    let mut by_lang: HashMap<LanguageCode, Vec<&l2::DispatchSpan>> = HashMap::new();
    for span in &dispatch_spans {
        by_lang
            .entry(span.target_lang.clone())
            .or_default()
            .push(span);
    }

    tracing::info!(
        filename = %filename,
        deferred = deferred.len(),
        spans = dispatch_spans.len(),
        languages = by_lang.len(),
        "L2 morphotag: dispatching @s words to secondary workers"
    );

    let mut merged_results: Vec<Option<l2::MergedL2Morphology>> = vec![None; deferred.len()];

    for (target_lang, lang_spans) in &by_lang {
        let lang3 = match crate::api::LanguageCode3::try_new(target_lang.as_ref()) {
            Ok(l) => l,
            Err(_) => {
                tracing::warn!(lang = %target_lang, "L2 morphotag: invalid language code");
                continue;
            }
        };

        let supported = if let Some(reg) = services.pool.stanza_registry() {
            reg.supports_morphosyntax(target_lang.as_ref())
        } else {
            crate::chat_ops::morphosyntax_ops::is_stanza_supported(target_lang)
        };

        if !supported {
            let total_words: usize = lang_spans.iter().map(|s| s.words.len()).sum();
            tracing::info!(lang = %lang3, words = total_words, "L2 morphotag: unsupported language");
            continue;
        }

        // Each span becomes one BatchItemWithPosition (one Stanza "sentence").
        let batch_items: Vec<BatchItemWithPosition> = lang_spans
            .iter()
            .map(|span| {
                let num_words = span.words.len();
                (
                    0, // line_idx placeholder
                    0, // utt_ordinal placeholder
                    MorphosyntaxBatchItem {
                        words: span.words.clone(),
                        terminator: talkbank_model::Terminator::Period {
                            span: talkbank_model::Span::DUMMY,
                        },
                        special_forms: vec![(None, None); num_words],
                        lang: target_lang.clone(),
                    },
                    Vec::new(), // no extracted words needed
                )
            })
            .collect();

        let empty_mwt: std::collections::BTreeMap<String, Vec<String>> =
            std::collections::BTreeMap::new();

        match infer_batch(services.pool, &batch_items, &lang3, &empty_mwt, true, None).await {
            Ok(responses) => {
                let mapping_ctx = crate::chat_ops::nlp::MappingContext {
                    lang: target_lang.clone(),
                };
                for (span, ud_resp) in lang_spans.iter().zip(responses.iter()) {
                    if let Some(sentence) = ud_resp.sentences.first() {
                        // Use map_ud_sentence to handle MWT Range tokens.
                        // This collapses contractions (it's → pron|it~aux|be)
                        // and produces 1 Mor per original word.
                        let (mors, gra_relations) =
                            match crate::chat_ops::nlp::map_ud_sentence(sentence, &mapping_ctx) {
                                Ok((mors, gras)) => (mors, gras),
                                Err(e) => {
                                    tracing::warn!(
                                        lang = %lang3,
                                        error = %e,
                                        "L2 morphotag: map_ud_sentence failed"
                                    );
                                    continue;
                                }
                            };

                        // Phrasal-verb recognition needs the full UD
                        // sentence, but only when the UD tokens line up
                        // 1:1 with the mapped Mors. MWT-collapsed cases
                        // (e.g., "it's" → pron|it~aux|be) do not line up,
                        // and phrasal verbs do not involve MWT, so we skip
                        // context for those spans.
                        let pass_context = sentence.words.len() == mors.len();

                        // NEW: Calculate the span's external anchor.
                        // Find the word in this L2 span that points to a head OUTSIDE
                        // the span in the original primary analysis. This "Span Head"
                        // is used as the anchor for Stanza's root result.
                        let mut span_external_head = None;
                        for &global_idx in &span.global_indices {
                            let primary_head = deferred[global_idx].primary.head;
                            let head_in_span = primary_head > 0
                                && span
                                    .global_indices
                                    .iter()
                                    .any(|&gi| deferred[gi].word_idx + 1 == primary_head);
                            if !head_in_span {
                                span_external_head = Some(primary_head);
                                break;
                            }
                        }

                        let mut chunk_offset = 0usize;
                        for (idx, global_idx) in span.global_indices.iter().enumerate() {
                            if let Some(mor) = mors.get(idx) {
                                let chunk_count = mor.count_chunks();
                                let item_gras = if chunk_offset + chunk_count <= gra_relations.len()
                                {
                                    gra_relations[chunk_offset..chunk_offset + chunk_count].to_vec()
                                } else {
                                    Vec::new()
                                };
                                chunk_offset += chunk_count;

                                let ctx = if pass_context {
                                    Some(l2::SecondaryUdContext {
                                        sentence,
                                        word_position: idx,
                                    })
                                } else {
                                    None
                                };
                                let merged = l2::merge_primary_secondary_with_context(
                                    &deferred[*global_idx].primary,
                                    mor.clone(),
                                    item_gras,
                                    target_lang,
                                    span_external_head,
                                    ctx.as_ref(),
                                );
                                merged_results[*global_idx] = Some(merged);
                            }
                        }
                    }
                }
                let total_words: usize = lang_spans.iter().map(|s| s.words.len()).sum();
                tracing::info!(
                    lang = %lang3,
                    spans = lang_spans.len(),
                    words = total_words,
                    "L2 morphotag: secondary dispatch succeeded"
                );
            }
            Err(e) => {
                tracing::warn!(lang = %lang3, error = %e, "L2 morphotag: secondary dispatch failed");
            }
        }
    }

    let outcome = l2::splice_l2_into_chat(chat_file, deferred, &merged_results);
    tracing::info!(
        filename = %filename,
        spliced = outcome.spliced,
        fallback = outcome.fallback,
        gra_upgraded = outcome.gra_upgraded,
        "L2 morphotag: splice complete"
    );
}

/// Pre-extract word texts for all deferred positions, walking each utterance
/// at most once. Returns a map from `(line_idx, word_idx)` to word text.
fn build_word_text_cache(
    chat_file: &ChatFile,
    deferred: &[l2::L2DeferredPosition],
) -> HashMap<(usize, usize), talkbank_model::ChatCleanedText> {
    use crate::chat_ops::Line;
    use talkbank_transform::extract;

    // Collect unique line indices to avoid re-walking the same utterance.
    let mut lines_needed: HashMap<usize, Vec<usize>> = HashMap::new();
    for def in deferred {
        lines_needed
            .entry(def.line_idx)
            .or_default()
            .push(def.word_idx);
    }

    let mut cache: HashMap<(usize, usize), talkbank_model::ChatCleanedText> = HashMap::new();
    for (line_idx, word_indices) in &lines_needed {
        let utt = match &chat_file.lines[*line_idx] {
            Line::Utterance(u) => u,
            _ => continue,
        };
        let mut words = Vec::new();
        extract::collect_utterance_content(
            &utt.main.content.content,
            crate::chat_ops::TierDomain::Mor,
            &mut words,
        );
        for &widx in word_indices {
            if let Some(w) = words.get(widx) {
                // `w.text` is provenance-sealed `ChatCleanedText` from a
                // typed AST source. Clone preserves the type discipline
                // through this cache and into the L2 dispatch path —
                // the previous `as_str().to_string()` round-trip dropped
                // the seal and was the candidate BUG-009 leak site
                // (architectural review Debt 11). Now sealed.
                cache.insert((*line_idx, widx), w.text.clone());
            }
        }
    }
    cache
}
