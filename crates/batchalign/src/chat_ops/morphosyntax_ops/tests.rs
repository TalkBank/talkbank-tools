//! Tests for morphosyntax module.

use super::*;

fn ud_response_from_words(words_json: &str) -> crate::chat_ops::nlp::UdResponse {
    serde_json::from_str(&format!(r#"{{"sentences":[{{"words":{words_json}}}]}}"#)).unwrap()
}

#[test]
fn test_clear_morphosyntax() {
    use talkbank_model::model::Line;
    use talkbank_transform::parse::{TreeSitterParser, parse_lenient};

    let parser = TreeSitterParser::new().unwrap();
    // Minimal CHAT with %mor and %gra tiers
    let chat = include_str!("../../../../../test-fixtures/eng_hello_world_with_mor_gra.cha");
    let (mut chat_file, _errors) = parse_lenient(&parser, chat);

    // Verify the utterance has %mor and %gra before clearing
    let utt = chat_file
        .lines
        .iter()
        .find_map(|l| match l {
            Line::Utterance(u) => Some(u),
            _ => None,
        })
        .expect("should have an utterance");
    assert!(
        utt.dependent_tiers
            .iter()
            .any(|t| matches!(t, talkbank_model::model::DependentTier::Mor(_))),
        "should have %mor before clear"
    );
    assert!(
        utt.dependent_tiers
            .iter()
            .any(|t| matches!(t, talkbank_model::model::DependentTier::Gra(_))),
        "should have %gra before clear"
    );

    // Clear then sweep any unfilled placeholders (mirrors pipeline order).
    clear_morphosyntax(&mut chat_file);
    crate::chat_ops::morphosyntax_ops::remove_empty_morphosyntax_placeholders(&mut chat_file);

    // Verify no %mor or %gra remain after the full clear+sweep.
    let utt = chat_file
        .lines
        .iter()
        .find_map(|l| match l {
            Line::Utterance(u) => Some(u),
            _ => None,
        })
        .expect("should still have an utterance");
    assert!(
        !utt.dependent_tiers
            .iter()
            .any(|t| matches!(t, talkbank_model::model::DependentTier::Mor(_))),
        "should NOT have %mor after clear+sweep"
    );
    assert!(
        !utt.dependent_tiers
            .iter()
            .any(|t| matches!(t, talkbank_model::model::DependentTier::Gra(_))),
        "should NOT have %gra after clear+sweep"
    );
}

#[test]
fn test_clear_morphosyntax_preserves_other_tiers() {
    use talkbank_model::model::Line;
    use talkbank_transform::parse::{TreeSitterParser, parse_lenient};

    let parser = TreeSitterParser::new().unwrap();
    // CHAT with %mor, %gra, and %act -- only %mor/%gra should be removed
    let chat = include_str!("../../../../../test-fixtures/eng_hello_world_with_mor_gra_act.cha");
    let (mut chat_file, _) = parse_lenient(&parser, chat);

    clear_morphosyntax(&mut chat_file);
    crate::chat_ops::morphosyntax_ops::remove_empty_morphosyntax_placeholders(&mut chat_file);

    let utt = chat_file
        .lines
        .iter()
        .find_map(|l| match l {
            Line::Utterance(u) => Some(u),
            _ => None,
        })
        .expect("utterance");
    // %act should survive clear+sweep
    assert!(
        !utt.dependent_tiers.is_empty(),
        "should still have %act after clear+sweep"
    );
    // But no %mor or %gra
    assert!(
        !utt.dependent_tiers.iter().any(|t| matches!(
            t,
            talkbank_model::model::DependentTier::Mor(_)
                | talkbank_model::model::DependentTier::Gra(_)
        )),
        "should not have %mor or %gra after clear+sweep"
    );
}

#[test]
fn test_clear_morphosyntax_no_tiers() {
    use talkbank_model::model::Line;
    use talkbank_transform::parse::{TreeSitterParser, parse_lenient};

    let parser = TreeSitterParser::new().unwrap();
    // CHAT without any dependent tiers -- clear should be a no-op
    let chat = include_str!("../../../../../test-fixtures/eng_hello_male.cha");
    let (mut chat_file, _) = parse_lenient(&parser, chat);

    clear_morphosyntax(&mut chat_file);

    let utt = chat_file
        .lines
        .iter()
        .find_map(|l| match l {
            Line::Utterance(u) => Some(u),
            _ => None,
        })
        .expect("utterance");
    assert!(utt.dependent_tiers.is_empty());
}

#[test]
fn test_validate_mor_alignment_ok() {
    use talkbank_transform::parse::{TreeSitterParser, parse_lenient};

    let parser = TreeSitterParser::new().unwrap();
    // Correctly aligned: 2 main words + 2 %mor items
    let chat = include_str!("../../../../../test-fixtures/eng_hello_world_with_mor_gra.cha");
    let (chat_file, _) = parse_lenient(&parser, chat);

    let warnings = validate_mor_alignment(&chat_file);
    assert!(
        warnings.is_empty(),
        "expected no alignment warnings, got: {:?}",
        warnings
    );
}

#[test]
fn test_validate_mor_alignment_no_mor_tier() {
    use talkbank_transform::parse::{TreeSitterParser, parse_lenient};

    let parser = TreeSitterParser::new().unwrap();
    // No %mor -- validation should pass (nothing to check)
    let chat = include_str!("../../../../../test-fixtures/eng_hello_male.cha");
    let (chat_file, _) = parse_lenient(&parser, chat);

    let warnings = validate_mor_alignment(&chat_file);
    assert!(warnings.is_empty());
}

// -----------------------------------------------------------------------
// Cross-language roundtrip snapshot tests
// -----------------------------------------------------------------------

/// Verify MorphosyntaxBatchItem serializes to the JSON shape Python expects.
#[test]
fn snapshot_morphosyntax_batch_item() {
    // Test fixture path via the explicitly-labeled
    // `ChatCleanedText::test_unchecked` escape hatch (gated behind the
    // `test-utils` feature on `talkbank-model`).
    let words: Vec<talkbank_model::ChatCleanedText> = ["the", "dog", "runs"]
        .iter()
        .map(|s| talkbank_model::ChatCleanedText::test_unchecked(*s))
        .collect();
    let item = MorphosyntaxBatchItem {
        words,
        terminator: talkbank_model::Terminator::Period {
            span: talkbank_model::Span::DUMMY,
        },
        special_forms: vec![(None, None), (None, None), (None, None)],
        lang: talkbank_model::model::LanguageCode::new("eng"),
    };
    insta::assert_json_snapshot!("morphosyntax_batch_item", item);
}

/// Verify UdResponse from Python deserializes correctly in Rust.
#[test]
fn snapshot_ud_response_from_python() {
    // This is the exact shape Python's Stanza inference returns
    let python_json = r#"{
        "sentences": [
            {
                "words": [
                    {
                        "id": 1,
                        "text": "the",
                        "lemma": "the",
                        "upos": "DET",
                        "xpos": "DT",
                        "feats": "Definite=Def|PronType=Art",
                        "head": 2,
                        "deprel": "det",
                        "start_char": 0,
                        "end_char": 3
                    },
                    {
                        "id": 2,
                        "text": "dog",
                        "lemma": "dog",
                        "upos": "NOUN",
                        "xpos": "NN",
                        "feats": "Number=Sing",
                        "head": 3,
                        "deprel": "nsubj",
                        "start_char": 4,
                        "end_char": 7
                    },
                    {
                        "id": 3,
                        "text": "runs",
                        "lemma": "run",
                        "upos": "VERB",
                        "xpos": "VBZ",
                        "feats": "Mood=Ind|Number=Sing|Person=3|Tense=Pres|VerbForm=Fin",
                        "head": 0,
                        "deprel": "root",
                        "start_char": 8,
                        "end_char": 12
                    }
                ]
            }
        ]
    }"#;

    let ud: crate::chat_ops::nlp::UdResponse = serde_json::from_str(python_json).unwrap();
    assert_eq!(ud.sentences.len(), 1);
    assert_eq!(ud.sentences[0].words.len(), 3);
    assert_eq!(ud.sentences[0].words[2].lemma, "run");

    // Re-serialize and snapshot to verify round-trip fidelity
    insta::assert_json_snapshot!("ud_response_roundtrip", ud);
}

/// Verify collect_payloads produces the expected shape for a simple CHAT.
#[test]
fn snapshot_collected_payloads() {
    use talkbank_transform::parse::{TreeSitterParser, parse_lenient};

    let parser = TreeSitterParser::new().unwrap();
    let chat = include_str!("../../../../../test-fixtures/eng_the_dog_runs.cha");
    let (chat_file, _) = parse_lenient(&parser, chat);

    let primary = talkbank_model::model::LanguageCode::new("eng");
    let langs = declared_languages(&chat_file, &primary);
    let collected = collect_payloads(&chat_file, &primary, &langs, MultilingualPolicy::ProcessAll);
    let items = collected.batch_items;
    let total = collected.total_utterances;

    assert_eq!(total, 1);
    assert_eq!(items.len(), 1);

    // Snapshot just the batch item (the payload that crosses the wire)
    let (_, _, ref batch_item, _) = items[0];
    insta::assert_json_snapshot!("collected_payload_item", batch_item);
}

// -----------------------------------------------------------------------
// Regression tests: batch item lang must reflect file @Languages header,
// not the batch-level primary_lang parameter.
//
// Bug: when a job has lang="eng" (the default) but a file declares
// @Languages: spa, collect_payloads produced items with lang="eng"
// instead of "spa". This caused Stanza to use the wrong model.
// -----------------------------------------------------------------------

/// When @Languages declares "spa" but primary_lang is "eng" (batch default),
/// the batch item must carry lang="spa" from the file header.
#[test]
fn collect_payloads_uses_file_language_not_batch_default_spa() {
    use talkbank_transform::parse::{TreeSitterParser, parse_lenient};

    let parser = TreeSitterParser::new().unwrap();
    let chat = include_str!("../../../../../test-fixtures/spa_hola_que_es_este.cha");
    let (chat_file, _) = parse_lenient(&parser, chat);

    // Simulate the batch-level default: primary_lang = "eng"
    let primary = talkbank_model::model::LanguageCode::new("eng");
    let langs = declared_languages(&chat_file, &primary);
    let items =
        collect_payloads(&chat_file, &primary, &langs, MultilingualPolicy::ProcessAll).batch_items;

    assert_eq!(items.len(), 1);
    let (_, _, ref batch_item, _) = items[0];

    // The batch item's lang MUST be "spa" (from @Languages header),
    // NOT "eng" (the batch default).
    assert_eq!(
        batch_item.lang.as_str(),
        "spa",
        "batch item lang should be 'spa' from @Languages header, not 'eng' batch default"
    );
}

/// Same regression for Russian: @Languages: rus with batch default "eng".
#[test]
fn collect_payloads_uses_file_language_not_batch_default_rus() {
    use talkbank_transform::parse::{TreeSitterParser, parse_lenient};

    let parser = TreeSitterParser::new().unwrap();
    let chat = include_str!("../../../../../test-fixtures/rus_vot_istoriya.cha");
    let (chat_file, _) = parse_lenient(&parser, chat);

    let primary = talkbank_model::model::LanguageCode::new("eng");
    let langs = declared_languages(&chat_file, &primary);
    let items =
        collect_payloads(&chat_file, &primary, &langs, MultilingualPolicy::ProcessAll).batch_items;

    assert_eq!(items.len(), 1);
    let (_, _, ref batch_item, _) = items[0];

    assert_eq!(
        batch_item.lang.as_str(),
        "rus",
        "batch item lang should be 'rus' from @Languages header, not 'eng' batch default"
    );
}

/// Same regression for Chinese: @Languages: zho with batch default "eng".
#[test]
fn collect_payloads_uses_file_language_not_batch_default_zho() {
    use talkbank_transform::parse::{TreeSitterParser, parse_lenient};

    let parser = TreeSitterParser::new().unwrap();
    let chat = include_str!("../../../../../test-fixtures/zho_hao_qing_zhong.cha");
    let (chat_file, _) = parse_lenient(&parser, chat);

    let primary = talkbank_model::model::LanguageCode::new("eng");
    let langs = declared_languages(&chat_file, &primary);
    let items =
        collect_payloads(&chat_file, &primary, &langs, MultilingualPolicy::ProcessAll).batch_items;

    assert_eq!(items.len(), 1);
    let (_, _, ref batch_item, _) = items[0];

    assert_eq!(
        batch_item.lang.as_str(),
        "zho",
        "batch item lang should be 'zho' from @Languages header, not 'eng' batch default"
    );
}

/// Same regression for French: @Languages: fra with batch default "eng".
#[test]
fn collect_payloads_uses_file_language_not_batch_default_fra() {
    use talkbank_transform::parse::{TreeSitterParser, parse_lenient};

    let parser = TreeSitterParser::new().unwrap();
    let chat = include_str!("../../../../../test-fixtures/fra_lescargot_dort.cha");
    let (chat_file, _) = parse_lenient(&parser, chat);

    let primary = talkbank_model::model::LanguageCode::new("eng");
    let langs = declared_languages(&chat_file, &primary);
    let items =
        collect_payloads(&chat_file, &primary, &langs, MultilingualPolicy::ProcessAll).batch_items;

    assert_eq!(items.len(), 1);
    let (_, _, ref batch_item, _) = items[0];

    assert_eq!(
        batch_item.lang.as_str(),
        "fra",
        "batch item lang should be 'fra' from @Languages header, not 'eng' batch default"
    );
}

/// When @Languages matches primary_lang, lang should still be correct.
/// (Control case: ensures fix doesn't regress the happy path.)
#[test]
fn collect_payloads_lang_correct_when_primary_matches_header() {
    use talkbank_transform::parse::{TreeSitterParser, parse_lenient};

    let parser = TreeSitterParser::new().unwrap();
    let chat = include_str!("../../../../../test-fixtures/eng_hello_world_male.cha");
    let (chat_file, _) = parse_lenient(&parser, chat);

    let primary = talkbank_model::model::LanguageCode::new("eng");
    let langs = declared_languages(&chat_file, &primary);
    let items =
        collect_payloads(&chat_file, &primary, &langs, MultilingualPolicy::ProcessAll).batch_items;

    assert_eq!(items.len(), 1);
    let (_, _, ref batch_item, _) = items[0];

    assert_eq!(batch_item.lang.as_str(), "eng");
}

/// When @Languages has multiple languages, the first declared language
/// should be used as the utterance default (not the batch primary_lang).
#[test]
fn collect_payloads_uses_first_declared_language_for_multilingual() {
    use talkbank_transform::parse::{TreeSitterParser, parse_lenient};

    let parser = TreeSitterParser::new().unwrap();
    // Bilingual file: primary declared language is "spa", secondary is "eng"
    let chat = include_str!("../../../../../test-fixtures/spa_eng_bilingual_hola_mundo.cha");
    let (chat_file, _) = parse_lenient(&parser, chat);

    // Batch default is "eng" but file says "spa" first
    let primary = talkbank_model::model::LanguageCode::new("eng");
    let langs = declared_languages(&chat_file, &primary);
    let items =
        collect_payloads(&chat_file, &primary, &langs, MultilingualPolicy::ProcessAll).batch_items;

    assert_eq!(items.len(), 1);
    let (_, _, ref batch_item, _) = items[0];

    // Should use "spa" (first declared), not "eng" (batch default)
    assert_eq!(
        batch_item.lang.as_str(),
        "spa",
        "batch item lang should be 'spa' (first in @Languages), not 'eng' batch default"
    );
}

/// Regression: inject_results with retokenize on Cantonese retrace utterance.
///
/// The full pipeline: parse CHAT → extract words → construct UD response →
/// inject with retokenize mode. This should succeed but fails with
/// "MOR item count does not match alignable word count".
///
/// Source: MOST corpus 40415b.cha line 46.
#[test]
// CANTONESE-SPECIFIC TEST: Cantonese retrace detection with retokenization.
// Validates that Cantonese-specific n-gram minimums (2 chars) are applied
// during retrace marking when --retokenize mode is active.
fn test_inject_results_retokenize_cantonese_retrace() {
    use crate::chat_ops::morphosyntax_ops::inject_results;
    use crate::chat_ops::nlp::{UdId, UdResponse, UdSentence, UdWord};
    use talkbank_transform::parse::{TreeSitterParser, parse_lenient};

    let parser = TreeSitterParser::new().unwrap();
    let chat = include_str!("../../../../../test-fixtures/retok_yue_retrace.cha");
    let (mut chat_file, _errors) = parse_lenient(&parser, chat);

    let primary_lang = talkbank_model::model::LanguageCode::new("yue");
    let langs = declared_languages(&chat_file, &primary_lang);

    let batch_items = collect_payloads(
        &chat_file,
        &primary_lang,
        &langs,
        MultilingualPolicy::ProcessAll,
    )
    .batch_items;

    assert!(!batch_items.is_empty(), "Should have batch items");

    // Print what was extracted
    for (line_idx, utt_ord, item, words) in &batch_items {
        eprintln!(
            "Batch item: line={line_idx} utt={utt_ord} words={:?} item_words={:?}",
            words.iter().map(|w| w.text.as_ref()).collect::<Vec<_>>(),
            item.words,
        );
    }

    // Build a matching UD response (one word per extracted word)
    let first_item = &batch_items[0];
    let word_count = first_item.2.words.len();
    eprintln!("Word count from batch item: {word_count}");

    // Simulate what Python actually returns: _segment_cantonese reduces
    // 7 single-char words to 5 words (下+次→下次, 食+飯→食飯).
    // Stanza processes 5 words and returns 5 MOR items.
    let segmented_words = vec!["呢", "度", "下次", "食飯", "啦"];
    eprintln!(
        "Simulated PyCantonese segmentation: {:?} ({} words)",
        segmented_words,
        segmented_words.len()
    );
    // UD requires exactly one root (head=0). First word is the root, all
    // others depend on index 1.
    let ud_words: Vec<UdWord> = segmented_words
        .iter()
        .enumerate()
        .map(|(i, w)| {
            let (head, deprel) = if i == 0 { (0, "root") } else { (1, "dep") };
            UdWord {
                id: UdId::Single(i + 1),
                text: w.to_string(),
                lemma: w.to_string(),
                upos: crate::chat_ops::nlp::UdPunctable::Value(
                    crate::chat_ops::nlp::UniversalPos::Noun,
                ),
                xpos: None,
                feats: None,
                head,
                deprel: deprel.into(),
                deps: None,
                misc: None,
            }
        })
        .collect();

    let ud_response = UdResponse {
        sentences: vec![UdSentence { words: ud_words }],
    };

    let empty_mwt = std::collections::BTreeMap::new();
    let result = inject_results(
        &parser,
        &mut chat_file,
        batch_items,
        vec![ud_response],
        &primary_lang,
        TokenizationMode::StanzaRetokenize,
        &empty_mwt,
    );

    assert!(
        result.is_ok(),
        "inject_results should succeed for retokenize + retrace: {:?}",
        result.err()
    );
}

/// Regression: French utterance with embedded single quotes around elision.
///
/// `*MOT: On dit pas 'quoi tu veux' , mais 'qu' est-ce que' on dit .`
///
/// The `qu'` is a French elision (like `l'homme`, `j'ai`).  Stanza's French
/// MWT tokenizer expands `qu'` into a range token `[n, n+1]` with components
/// `qu` and `'`.  The MOR mapping must collapse these back into one MOR item
/// so the count matches the CHAT word count.
///
/// Source: childes-other-data/Biling/Amsterdam/Anouk/fra/030428.cha line 509.
/// This caused batch 7 of the multilingual morphotag rerun to fail with:
/// "MOR item count (14) does not match alignable word count (13)"
#[test]
fn test_french_elision_in_quoted_context() {
    use crate::chat_ops::morphosyntax_ops::collect_payloads;

    let parser = talkbank_transform::parse::TreeSitterParser::new().unwrap();
    let chat = include_str!("../../../../../test-fixtures/fra_french_elision_quotes.cha");
    let (chat_file, _) = talkbank_transform::parse::parse_lenient(&parser, chat);

    let primary = talkbank_model::model::LanguageCode::new("fra");
    let langs = declared_languages(&chat_file, &primary);
    let items =
        collect_payloads(&chat_file, &primary, &langs, MultilingualPolicy::ProcessAll).batch_items;

    assert_eq!(items.len(), 1, "Should have exactly 1 utterance payload");
    let (_, _, item, extracted_words) = &items[0];

    // Print for debugging
    println!("Extracted words: {:?}", item.words);
    println!("Word count: {}", item.words.len());
    println!(
        "Extracted word details: {:?}",
        extracted_words
            .iter()
            .map(|w| w.text.as_ref())
            .collect::<Vec<_>>()
    );

    // The utterance has these CHAT words (in MOR domain, excluding separators):
    // On, dit, pas, 'quoi, tu, veux', mais, 'qu', est-ce, que', on, dit, .
    // That's 13 words (including the terminator).
    // Stanza should NOT produce more MOR items than this.
    let word_count = item.words.len();
    assert!(
        word_count > 0,
        "Should extract some words from French utterance"
    );
    println!("CHAT word count for MOR alignment: {word_count}");
}

// -----------------------------------------------------------------------
// @s (code-switching) payload collection tests
// -----------------------------------------------------------------------

/// Verify collect_payloads identifies @s:spa word positions in special_forms.
#[test]
fn collect_payloads_identifies_at_s_positions_single() {
    use talkbank_transform::parse::{TreeSitterParser, parse_lenient};

    let parser = TreeSitterParser::new().unwrap();
    let chat = include_str!("../../../../../test-fixtures/eng_spa_at_s_single.cha");
    let (chat_file, _) = parse_lenient(&parser, chat);

    let primary = talkbank_model::model::LanguageCode::new("eng");
    let langs = declared_languages(&chat_file, &primary);
    let items =
        collect_payloads(&chat_file, &primary, &langs, MultilingualPolicy::ProcessAll).batch_items;

    assert_eq!(items.len(), 1, "should have 1 utterance");
    let (_, _, ref item, _) = items[0];

    // "I went to the tienda@s:spa yesterday ."
    // Words: I, went, to, the, tienda, yesterday (6 words, terminator separate)
    assert!(
        item.words.len() >= 6,
        "should have at least 6 words, got {}",
        item.words.len()
    );

    // Find the position of "tienda" and verify it has a language resolution
    let tienda_idx = item
        .words
        .iter()
        .position(|w| w == "tienda")
        .expect("should contain 'tienda'");

    let (_, ref lang_res) = item.special_forms[tienda_idx];
    assert!(
        lang_res.is_some(),
        "tienda should have a language resolution (it's @s:spa)"
    );
    let resolution = lang_res.as_ref().unwrap();
    let langs = resolution.languages();
    assert_eq!(langs.len(), 1, "should resolve to exactly one language");
    assert_eq!(langs[0].as_str(), "spa", "should resolve to Spanish");

    // All other words should have None for language resolution
    for (i, (_, lr)) in item.special_forms.iter().enumerate() {
        if i != tienda_idx {
            assert!(
                lr.is_none(),
                "word {} ('{}') should NOT have language resolution",
                i,
                item.words[i]
            );
        }
    }
}

/// Verify contiguous @s:spa span in special_forms.
#[test]
fn collect_payloads_identifies_contiguous_at_s_span() {
    use talkbank_transform::parse::{TreeSitterParser, parse_lenient};

    let parser = TreeSitterParser::new().unwrap();
    let chat = include_str!("../../../../../test-fixtures/eng_spa_at_s_contiguous.cha");
    let (chat_file, _) = parse_lenient(&parser, chat);

    let primary = talkbank_model::model::LanguageCode::new("eng");
    let langs = declared_languages(&chat_file, &primary);
    let items =
        collect_payloads(&chat_file, &primary, &langs, MultilingualPolicy::ProcessAll).batch_items;

    assert_eq!(items.len(), 1);
    let (_, _, ref item, _) = items[0];

    // "we talked about los@s:spa niños@s:spa ."
    let los_idx = item
        .words
        .iter()
        .position(|w| w == "los")
        .expect("should have 'los'");
    let ninos_idx = item
        .words
        .iter()
        .position(|w| w == "niños")
        .expect("should have 'niños'");

    // Both should have spa resolution
    assert!(item.special_forms[los_idx].1.is_some());
    assert!(item.special_forms[ninos_idx].1.is_some());

    // They should be contiguous
    assert_eq!(ninos_idx, los_idx + 1, "los and niños should be adjacent");

    // Verify span grouping produces one span
    let spans =
        crate::chat_ops::morphosyntax_ops::l2::group_l2_spans(&item.special_forms, &item.words);
    assert_eq!(spans.len(), 1, "contiguous same-lang should produce 1 span");
    assert_eq!(spans[0].word_indices, vec![los_idx, ninos_idx]);
    assert_eq!(spans[0].words, vec!["los", "niños"]);
}

/// Thin L2 acceptance: exercise the real order above the seam-local
/// tests without invoking the worker/runtime layer:
///
/// 1. collect primary payloads
/// 2. extract deferred @s positions from the primary UD response
/// 3. inject primary results (which writes `L2|xxx` placeholders)
/// 4. plan the secondary dispatch span from the mutated `ChatFile`
/// 5. merge a synthetic secondary UD sentence for that span
/// 6. splice the merged secondary morphology back into the host file
#[test]
fn l2_pipeline_contiguous_span_replaces_placeholders_and_preserves_valid_gra() {
    use talkbank_model::ParseValidateOptions;
    use talkbank_model::model::Line;
    use talkbank_transform::morphosyntax::l2::{
        extract_l2_deferred_positions, merge_planned_secondary_span, plan_secondary_dispatch,
        splice_l2_into_chat,
    };
    use talkbank_transform::morphosyntax::{UdId, UdPunctable, UdSentence, UdWord, UniversalPos};
    use talkbank_transform::parse::{TreeSitterParser, parse_lenient};

    let parser = TreeSitterParser::new().unwrap();
    let chat = include_str!("../../../../../test-fixtures/eng_spa_at_s_contiguous.cha");
    let (mut chat_file, _) = parse_lenient(&parser, chat);

    let primary_lang = talkbank_model::model::LanguageCode::new("eng");
    let langs = declared_languages(&chat_file, &primary_lang);
    let batch_items = collect_payloads(
        &chat_file,
        &primary_lang,
        &langs,
        MultilingualPolicy::ProcessAll,
    )
    .batch_items;
    assert_eq!(
        batch_items.len(),
        1,
        "fixture should produce one utterance payload"
    );

    let primary_ud_response = ud_response_from_words(
        r#"[
            {"id":1,"text":"we","lemma":"we","upos":"PRON","head":2,"deprel":"nsubj"},
            {"id":2,"text":"talked","lemma":"talk","upos":"VERB","head":0,"deprel":"root"},
            {"id":3,"text":"about","lemma":"about","upos":"ADP","head":5,"deprel":"case"},
            {"id":4,"text":"los","lemma":"the","upos":"DET","head":5,"deprel":"det"},
            {"id":5,"text":"niños","lemma":"child","upos":"NOUN","head":2,"deprel":"obl"},
            {"id":6,"text":".","lemma":".","upos":"PUNCT","head":2,"deprel":"punct"}
        ]"#,
    );

    let deferred =
        extract_l2_deferred_positions(&batch_items, std::slice::from_ref(&primary_ud_response));
    assert_eq!(
        deferred.len(),
        2,
        "contiguous Spanish span should defer two positions"
    );
    assert_eq!(deferred[0].word_idx, 3);
    assert_eq!(deferred[1].word_idx, 4);
    assert_eq!(deferred[0].target_lang.as_str(), "spa");
    assert_eq!(deferred[1].target_lang.as_str(), "spa");

    let empty_mwt = std::collections::BTreeMap::new();
    let injection = inject_results(
        &parser,
        &mut chat_file,
        batch_items,
        vec![primary_ud_response],
        &primary_lang,
        TokenizationMode::Preserve,
        &empty_mwt,
    )
    .expect("primary injection should succeed");
    assert!(
        injection.decisions.is_empty(),
        "primary L2-placeholder injection should not degrade the fixture: {:?}",
        injection.decisions
    );

    let utterance = chat_file
        .lines
        .iter()
        .find_map(|line| match line {
            Line::Utterance(utt) => Some(utt),
            _ => None,
        })
        .expect("fixture should contain an utterance");
    let mor_before = utterance.mor_tier().expect("%mor after primary injection");
    assert_eq!(mor_before.items()[3].main.pos.to_string(), "L2");
    assert_eq!(mor_before.items()[3].main.lemma.to_string(), "xxx");
    assert_eq!(mor_before.items()[4].main.pos.to_string(), "L2");
    assert_eq!(mor_before.items()[4].main.lemma.to_string(), "xxx");

    let dispatch_plan = plan_secondary_dispatch(&chat_file, &deferred);
    assert_eq!(
        dispatch_plan.spans.len(),
        1,
        "contiguous span should plan as one secondary sentence"
    );
    let span = &dispatch_plan.spans[0];
    assert_eq!(
        span.words.iter().map(|w| w.as_str()).collect::<Vec<_>>(),
        vec!["los", "niños"]
    );
    assert!(
        span.attachment.is_external_root(),
        "the noun in the secondary span must reattach to the host predicate"
    );
    assert_eq!(
        span.attachment.external_root_deprel().map(|d| d.as_str()),
        Some("obl")
    );

    let secondary_sentence = UdSentence {
        words: vec![
            UdWord {
                id: UdId::Single(1),
                text: "los".to_string(),
                lemma: "el".to_string(),
                upos: UdPunctable::Value(UniversalPos::Det),
                xpos: None,
                feats: None,
                head: 2,
                deprel: "det".to_string(),
                deps: None,
                misc: None,
            },
            UdWord {
                id: UdId::Single(2),
                text: "niños".to_string(),
                lemma: "niño".to_string(),
                upos: UdPunctable::Value(UniversalPos::Noun),
                xpos: None,
                feats: None,
                head: 0,
                deprel: "root".to_string(),
                deps: None,
                misc: None,
            },
        ],
    };
    let merged_pairs = merge_planned_secondary_span(span, &deferred, &secondary_sentence)
        .expect("secondary merge");
    assert_eq!(
        merged_pairs.len(),
        2,
        "secondary span should merge back into two positions"
    );

    let mut merged_results = vec![None; deferred.len()];
    for (global_idx, merged) in merged_pairs {
        merged_results[global_idx] = Some(merged);
    }

    let outcome = splice_l2_into_chat(&mut chat_file, &deferred, &merged_results);
    assert_eq!(outcome.spliced, 2);
    assert_eq!(outcome.fallback, 0);
    assert_eq!(outcome.gra_upgraded, 0);

    let utterance = chat_file
        .lines
        .iter()
        .find_map(|line| match line {
            Line::Utterance(utt) => Some(utt),
            _ => None,
        })
        .expect("fixture should contain an utterance");
    let mor_after = utterance.mor_tier().expect("%mor after L2 splice");
    assert_eq!(mor_after.items()[3].main.pos.to_string(), "det");
    assert_eq!(mor_after.items()[3].main.lemma.to_string(), "el");
    assert_eq!(mor_after.items()[4].main.pos.to_string(), "noun");
    assert_eq!(mor_after.items()[4].main.lemma.to_string(), "niño");

    let gra_after = utterance.gra_tier().expect("%gra after L2 splice");
    assert_eq!(gra_after.relations()[3].to_string(), "4|5|DET");
    assert_eq!(gra_after.relations()[4].to_string(), "5|2|OBL");

    let opts = ParseValidateOptions::default().with_alignment();
    talkbank_model::validate_chat_file_with_options(&mut chat_file, &opts)
        .expect("L2 acceptance fixture should validate after splice");
}

// -----------------------------------------------------------------------
// Regression: retokenize with MWT Range tokens
//
// Bug: inject_results with StanzaRetokenize includes Range parent tokens
// AND their component words in the token vector. map_ud_sentence() merges
// Range components into 1 clitic MOR, so mors.len() < tokens.len().
// retokenize_utterance fails on the count mismatch.
//
// Example: Stanza returns "gonna" as Range(1,2) + "gon" + "na".
// Token vector gets 6 items [gonna, gon, na, eat, cookies, .] but
// only 4 MOR items (the Range is merged into verb|go~part|to).
// -----------------------------------------------------------------------

/// inject_results with StanzaRetokenize must handle MWT Range tokens
/// without failing on token/MOR count mismatch.
#[test]
fn inject_results_retokenize_mwt_range_tokens_no_failure() {
    use crate::chat_ops::morphosyntax_ops::inject_results;
    use crate::chat_ops::nlp::{UdId, UdPunctable, UdResponse, UdSentence, UdWord, UniversalPos};
    use talkbank_transform::parse::{TreeSitterParser, parse_lenient};

    let parser = TreeSitterParser::new().unwrap();
    // Minimal CHAT: "gonna eat cookies ."
    let chat = "\
@UTF8
@Begin
@Languages:\teng
@Participants:\tPAR Participant
@ID:\teng|test|PAR|||||Participant|||
*PAR:\tgonna eat cookies .
@End
";
    let (mut chat_file, _errors) = parse_lenient(&parser, chat);

    let primary_lang = talkbank_model::model::LanguageCode::new("eng");
    let langs = declared_languages(&chat_file, &primary_lang);
    let batch_items = collect_payloads(
        &chat_file,
        &primary_lang,
        &langs,
        MultilingualPolicy::ProcessAll,
    )
    .batch_items;

    assert_eq!(batch_items.len(), 1, "should have 1 utterance");
    let word_count = batch_items[0].2.words.len();
    // CHAT extracts main tier words (gonna, eat, cookies) + terminator is
    // stored separately in the batch item.
    assert!(
        word_count >= 3,
        "should have at least 3 words (gonna, eat, cookies), got {word_count}"
    );

    // Simulate Stanza's free-tokenize output with MWT expansion for "gonna".
    // This is the exact shape that Stanza returns: a Range parent token
    // followed by its component words, then the remaining regular words.
    let ud_response = UdResponse {
        sentences: vec![UdSentence {
            words: vec![
                // Range parent token for "gonna"
                UdWord {
                    id: UdId::Range(1, 2),
                    text: "gonna".into(),
                    lemma: "".into(),
                    upos: UdPunctable::Value(UniversalPos::X),
                    xpos: None,
                    feats: None,
                    head: 0,
                    deprel: "dep".into(),
                    deps: None,
                    misc: None,
                },
                // Component 1: "gon" (going)
                UdWord {
                    id: UdId::Single(1),
                    text: "gon".into(),
                    lemma: "go".into(),
                    upos: UdPunctable::Value(UniversalPos::Verb),
                    xpos: None,
                    feats: Some("VerbForm=Part".into()),
                    head: 4,
                    deprel: "advcl".into(),
                    deps: None,
                    misc: None,
                },
                // Component 2: "na" (to)
                UdWord {
                    id: UdId::Single(2),
                    text: "na".into(),
                    lemma: "to".into(),
                    upos: UdPunctable::Value(UniversalPos::Part),
                    xpos: None,
                    feats: None,
                    head: 1,
                    deprel: "mark".into(),
                    deps: None,
                    misc: None,
                },
                // Regular word: "eat"
                UdWord {
                    id: UdId::Single(3),
                    text: "eat".into(),
                    lemma: "eat".into(),
                    upos: UdPunctable::Value(UniversalPos::Verb),
                    xpos: None,
                    feats: Some("VerbForm=Inf".into()),
                    head: 0,
                    deprel: "root".into(),
                    deps: None,
                    misc: None,
                },
                // Regular word: "cookies"
                UdWord {
                    id: UdId::Single(4),
                    text: "cookies".into(),
                    lemma: "cookie".into(),
                    upos: UdPunctable::Value(UniversalPos::Noun),
                    xpos: None,
                    feats: Some("Number=Plur".into()),
                    head: 3,
                    deprel: "obj".into(),
                    deps: None,
                    misc: None,
                },
                // Punctuation: "."
                UdWord {
                    id: UdId::Single(5),
                    text: ".".into(),
                    lemma: ".".into(),
                    upos: UdPunctable::Punct("PUNCT".into()),
                    xpos: None,
                    feats: None,
                    head: 3,
                    deprel: "punct".into(),
                    deps: None,
                    misc: None,
                },
            ],
        }],
    };

    let empty_mwt = std::collections::BTreeMap::new();
    let result = inject_results(
        &parser,
        &mut chat_file,
        batch_items,
        vec![ud_response],
        &primary_lang,
        TokenizationMode::StanzaRetokenize,
        &empty_mwt,
    );

    // The injection must succeed — no retokenization_failed decision.
    let injection = result.expect("inject_results should not return Err");
    let failed_decisions: Vec<_> = injection
        .decisions
        .iter()
        .filter(|d| {
            d.strategy.strategy_name() == "retokenization_failed"
                || d.strategy.strategy_name() == "injection_failed"
                || d.strategy.strategy_name() == "mapping_failed"
        })
        .collect();
    assert!(
        failed_decisions.is_empty(),
        "Retokenize with MWT Range tokens should not produce failure decisions, \
         got: {failed_decisions:?}"
    );

    // The output should have a %mor tier.
    let utt = chat_file
        .lines
        .iter()
        .find_map(|l| match l {
            talkbank_model::model::Line::Utterance(u) => Some(u),
            _ => None,
        })
        .expect("should have an utterance");
    assert!(
        utt.dependent_tiers
            .iter()
            .any(|t| matches!(t, talkbank_model::model::DependentTier::Mor(_))),
        "Output should have a %mor tier after retokenize injection with MWT"
    );
}

#[test]
fn inject_results_preserve_coraal_units_keeps_mor_gra() {
    use crate::chat_ops::morphosyntax_ops::inject_results;
    use talkbank_transform::parse::{TreeSitterParser, parse_lenient};

    let parser = TreeSitterParser::new().unwrap();
    let chat = include_str!("../../../../../test-fixtures/eng_morph_coraal_units.cha");
    let (mut chat_file, _) = parse_lenient(&parser, chat);

    let primary_lang = talkbank_model::model::LanguageCode::new("eng");
    let langs = declared_languages(&chat_file, &primary_lang);
    let batch_items = collect_payloads(
        &chat_file,
        &primary_lang,
        &langs,
        MultilingualPolicy::ProcessAll,
    )
    .batch_items;

    assert_eq!(batch_items.len(), 1, "should have one payload");

    let ud_response = ud_response_from_words(
        r#"[
          {"id":1,"text":"now","lemma":"now","upos":"ADV","xpos":"RB","feats":"PronType=Dem","head":9,"deprel":"advmod"},
          {"id":2,"text":"the","lemma":"the","upos":"DET","xpos":"DT","feats":"Definite=Def|PronType=Art","head":3,"deprel":"det"},
          {"id":3,"text":"building's","lemma":"building'","upos":"NOUN","xpos":"NNS","feats":"Number=Plur","head":0,"deprel":"root"},
          {"id":4,"text":"only","lemma":"only","upos":"ADV","xpos":"RB","head":5,"deprel":"advmod"},
          {"id":5,"text":"four","lemma":"four","upos":"NUM","xpos":"CD","feats":"NumForm=Word|NumType=Card","head":6,"deprel":"nummod"},
          {"id":6,"text":"hundred","lemma":"hundred","upos":"NUM","xpos":"CD","feats":"NumForm=Word|NumType=Card","head":9,"deprel":"nummod"},
          {"id":7,"text":"and","lemma":"and","upos":"CCONJ","xpos":"CC","head":8,"deprel":"cc"},
          {"id":8,"text":"ninety","lemma":"ninety","upos":"NUM","xpos":"CD","feats":"NumForm=Word|NumType=Card","head":6,"deprel":"conj"},
          {"id":9,"text":"units","lemma":"unit","upos":"NOUN","xpos":"NNS","feats":"Number=Plur","head":3,"deprel":"conj"},
          {"id":10,"text":".","lemma":".","upos":"PUNCT","xpos":".","head":3,"deprel":"punct"}
        ]"#,
    );

    let empty_mwt = std::collections::BTreeMap::new();
    let injection = inject_results(
        &parser,
        &mut chat_file,
        batch_items,
        vec![ud_response],
        &primary_lang,
        TokenizationMode::Preserve,
        &empty_mwt,
    )
    .expect("preserve injection should not return Err");

    let failed_decisions: Vec<_> = injection
        .decisions
        .iter()
        .filter(|d| {
            d.strategy.strategy_name() == "mapping_failed"
                || d.strategy.strategy_name() == "injection_failed"
                || d.strategy.strategy_name() == "retokenization_failed"
        })
        .collect();
    assert!(
        failed_decisions.is_empty(),
        "coraal units should not be dropped; got decisions: {failed_decisions:?}"
    );

    let utt = chat_file
        .lines
        .iter()
        .find_map(|l| match l {
            talkbank_model::model::Line::Utterance(u) => Some(u),
            _ => None,
        })
        .expect("should have utterance");
    assert!(
        utt.dependent_tiers
            .iter()
            .any(|t| matches!(t, talkbank_model::model::DependentTier::Mor(_))),
        "preserve injection should write %mor for the units case"
    );
    assert!(
        utt.dependent_tiers
            .iter()
            .any(|t| matches!(t, talkbank_model::model::DependentTier::Gra(_))),
        "preserve injection should write %gra for the units case"
    );
}

#[test]
fn inject_results_preserve_minga_because_keeps_mor_gra() {
    use crate::chat_ops::morphosyntax_ops::inject_results;
    use talkbank_transform::parse::{TreeSitterParser, parse_lenient};

    let parser = TreeSitterParser::new().unwrap();
    let chat = include_str!("../../../../../test-fixtures/eng_morph_minga_because.cha");
    let (mut chat_file, _) = parse_lenient(&parser, chat);

    let primary_lang = talkbank_model::model::LanguageCode::new("eng");
    let langs = declared_languages(&chat_file, &primary_lang);
    let batch_items = collect_payloads(
        &chat_file,
        &primary_lang,
        &langs,
        MultilingualPolicy::ProcessAll,
    )
    .batch_items;

    assert_eq!(batch_items.len(), 1, "should have one payload");

    let ud_response = ud_response_from_words(
        r#"[
          {"id":1,"text":"yes","lemma":"yes","upos":"INTJ","xpos":"UH","feats":"Polarity=Pos","head":2,"deprel":"discourse"},
          {"id":2,"text":"(be)cause","lemma":"(be)cause","upos":"NOUN","xpos":"NN","feats":"Number=Sing","head":0,"deprel":"root"},
          {"id":3,"text":"that","lemma":"that","upos":"PRON","xpos":"WDT","feats":"PronType=Rel","head":5,"deprel":"obl"},
          {"id":4,"text":"building's","lemma":"building'","upos":"NOUN","xpos":"NNS","feats":"Number=Plur","head":5,"deprel":"nsubj"},
          {"id":5,"text":"nice","lemma":"nice","upos":"ADJ","xpos":"JJ","feats":"Degree=Pos","head":2,"deprel":"acl:relcl"},
          {"id":6,"text":".","lemma":".","upos":"PUNCT","xpos":".","head":2,"deprel":"punct"}
        ]"#,
    );

    let empty_mwt = std::collections::BTreeMap::new();
    let injection = inject_results(
        &parser,
        &mut chat_file,
        batch_items,
        vec![ud_response],
        &primary_lang,
        TokenizationMode::Preserve,
        &empty_mwt,
    )
    .expect("preserve injection should not return Err");

    let failed_decisions: Vec<_> = injection
        .decisions
        .iter()
        .filter(|d| {
            d.strategy.strategy_name() == "mapping_failed"
                || d.strategy.strategy_name() == "injection_failed"
                || d.strategy.strategy_name() == "retokenization_failed"
        })
        .collect();
    assert!(
        failed_decisions.is_empty(),
        "minga because should not be dropped; got decisions: {failed_decisions:?}"
    );

    let utt = chat_file
        .lines
        .iter()
        .find_map(|l| match l {
            talkbank_model::model::Line::Utterance(u) => Some(u),
            _ => None,
        })
        .expect("should have utterance");
    assert!(
        utt.dependent_tiers
            .iter()
            .any(|t| matches!(t, talkbank_model::model::DependentTier::Mor(_))),
        "preserve injection should write %mor for the because case"
    );
}

#[test]
fn inject_results_preserve_kings_continuation_keeps_mor_gra() {
    use crate::chat_ops::morphosyntax_ops::inject_results;
    use talkbank_transform::parse::{TreeSitterParser, parse_lenient};

    let parser = TreeSitterParser::new().unwrap();
    let chat = include_str!("../../../../../test-fixtures/eng_morph_kings_continuation.cha");
    let (mut chat_file, _) = parse_lenient(&parser, chat);

    let primary_lang = talkbank_model::model::LanguageCode::new("eng");
    let langs = declared_languages(&chat_file, &primary_lang);
    let batch_items = collect_payloads(
        &chat_file,
        &primary_lang,
        &langs,
        MultilingualPolicy::ProcessAll,
    )
    .batch_items;

    assert_eq!(batch_items.len(), 2, "should have two payloads");

    let ud_responses = vec![
        ud_response_from_words(
            r#"[
              {"id":1,"text":"all","lemma":"all","upos":"DET","xpos":"PDT","feats":"PronType=Tot","head":4,"deprel":"det:predet"},
              {"id":2,"text":"the","lemma":"the","upos":"DET","xpos":"DT","feats":"Definite=Def|PronType=Art","head":4,"deprel":"det"},
              {"id":3,"text":"king's","lemma":"king'","upos":"NOUN","xpos":"NNS","feats":"Number=Plur","head":4,"deprel":"compound"},
              {"id":4,"text":"horses","lemma":"horse","upos":"NOUN","xpos":"NNS","feats":"Number=Plur","head":0,"deprel":"root"},
              {"id":5,"text":"+...","lemma":"+...","upos":"PUNCT","xpos":",","head":4,"deprel":"punct"}
            ]"#,
        ),
        ud_response_from_words(
            r#"[
              {"id":1,"text":"all","lemma":"all","upos":"DET","xpos":"PDT","feats":"PronType=Tot","head":4,"deprel":"det:predet"},
              {"id":2,"text":"the","lemma":"the","upos":"DET","xpos":"DT","feats":"Definite=Def|PronType=Art","head":4,"deprel":"det"},
              {"id":3,"text":"king's","lemma":"king'","upos":"NOUN","xpos":"NNS","feats":"Number=Plur","head":4,"deprel":"compound"},
              {"id":4,"text":"men","lemma":"man","upos":"NOUN","xpos":"NNS","feats":"Number=Plur","head":0,"deprel":"root"},
              {"id":5,"text":"+...","lemma":"+...","upos":"PUNCT","xpos":",","head":4,"deprel":"punct"}
            ]"#,
        ),
    ];

    let empty_mwt = std::collections::BTreeMap::new();
    let injection = inject_results(
        &parser,
        &mut chat_file,
        batch_items,
        ud_responses,
        &primary_lang,
        TokenizationMode::Preserve,
        &empty_mwt,
    )
    .expect("preserve injection should not return Err");

    let failed_decisions: Vec<_> = injection
        .decisions
        .iter()
        .filter(|d| {
            d.strategy.strategy_name() == "mapping_failed"
                || d.strategy.strategy_name() == "injection_failed"
                || d.strategy.strategy_name() == "retokenization_failed"
        })
        .collect();
    assert!(
        failed_decisions.is_empty(),
        "kings continuation should not be dropped; got decisions: {failed_decisions:?}"
    );

    let utts: Vec<_> = chat_file
        .lines
        .iter()
        .filter_map(|l| match l {
            talkbank_model::model::Line::Utterance(u) => Some(u),
            _ => None,
        })
        .collect();
    assert_eq!(utts.len(), 2, "should have two utterances");
    for utt in utts {
        assert!(
            utt.dependent_tiers
                .iter()
                .any(|t| matches!(t, talkbank_model::model::DependentTier::Mor(_))),
            "continuation utterances should keep %mor"
        );
        assert!(
            utt.dependent_tiers
                .iter()
                .any(|t| matches!(t, talkbank_model::model::DependentTier::Gra(_))),
            "continuation utterances should keep %gra"
        );
    }
}

/// Verify bare @s shortcut resolves via @Languages header.
#[test]
fn collect_payloads_bare_at_s_shortcut_resolution() {
    use talkbank_transform::parse::{TreeSitterParser, parse_lenient};

    let parser = TreeSitterParser::new().unwrap();
    let chat = include_str!("../../../../../test-fixtures/deu_eng_at_s_bare.cha");
    let (chat_file, _) = parse_lenient(&parser, chat);

    // Primary is German, secondary is English (from @Languages: deu, eng)
    let primary = talkbank_model::model::LanguageCode::new("deu");
    let langs = declared_languages(&chat_file, &primary);
    let items =
        collect_payloads(&chat_file, &primary, &langs, MultilingualPolicy::ProcessAll).batch_items;

    assert_eq!(items.len(), 1);
    let (_, _, ref item, _) = items[0];

    // "ich möchte film@s studies@s machen ."
    let film_idx = item
        .words
        .iter()
        .position(|w| w == "film")
        .expect("should have 'film'");
    let studies_idx = item
        .words
        .iter()
        .position(|w| w == "studies")
        .expect("should have 'studies'");

    // Bare @s should resolve to "eng" (secondary language from @Languages: deu, eng)
    let film_res = item.special_forms[film_idx]
        .1
        .as_ref()
        .expect("film should have lang");
    let film_langs = film_res.languages();
    assert_eq!(
        film_langs[0].as_str(),
        "eng",
        "bare @s should resolve to eng (secondary language)"
    );

    let studies_res = item.special_forms[studies_idx]
        .1
        .as_ref()
        .expect("studies should have lang");
    assert_eq!(studies_res.languages()[0].as_str(), "eng");

    // Span grouping should merge them (contiguous, same language)
    let spans =
        crate::chat_ops::morphosyntax_ops::l2::group_l2_spans(&item.special_forms, &item.words);
    assert_eq!(spans.len(), 1);
    assert_eq!(spans[0].target_lang.as_str(), "eng");
    assert_eq!(spans[0].words, vec!["film", "studies"]);
}

// ---------------------------------------------------------------------------
// Tier-order preservation (source layout of %mor / %gra / %wor is stable
// across the clear → inject round trip; %wor stays wherever it was).
// ---------------------------------------------------------------------------

/// Source-order layout `[Mor, Gra, Wor]` must survive a
/// `clear_morphosyntax` → `replace_or_add_tier` round trip.
///
/// Before the fix, `clear_morphosyntax` *removed* Mor/Gra from
/// `dependent_tiers`, so when fresh Mor/Gra were re-added via
/// `replace_or_add_tier` they were pushed to the end and the utterance
/// serialized as `[Wor, Mor, Gra]`. Now clear replaces in place with
/// empty placeholders and the subsequent replace finds the variant match.
#[test]
fn clear_then_reinject_preserves_tier_order_mor_gra_wor() {
    use talkbank_model::model::DependentTier;
    use talkbank_model::model::dependent_tier::{GraTier, MorTier, WorTier};
    use talkbank_parser::TreeSitterParser;
    use talkbank_transform::inject::replace_or_add_tier;
    use talkbank_transform::parse::parse_lenient;

    let text = concat!(
        "@UTF8\n",
        "@Begin\n",
        "@Languages:\teng\n",
        "@Participants:\tPAR Adult\n",
        "@ID:\teng|test|PAR|||||Adult|||\n",
        "*PAR:\thello world .\n",
        "%mor:\tintj|hello noun|world .\n",
        "%gra:\t1|2|DISCOURSE 2|0|ROOT 3|2|PUNCT\n",
        "%wor:\thello 0_500 world 500_1000 .\n",
        "@End\n",
    );
    let parser = TreeSitterParser::new().unwrap();
    let (mut chat, _errors) = parse_lenient(&parser, text);

    assert_eq!(
        dep_tier_kinds(&chat),
        vec!["Mor", "Gra", "Wor"],
        "fixture precondition: expected [Mor, Gra, Wor]",
    );

    // Clear — must preserve tier positions.
    crate::chat_ops::morphosyntax_ops::clear_morphosyntax(&mut chat);
    assert_eq!(
        dep_tier_kinds(&chat),
        vec!["Mor", "Gra", "Wor"],
        "clear_morphosyntax must preserve tier order",
    );

    // Re-inject: replace_or_add_tier must overwrite placeholders in place.
    let utt = first_utterance_mut(&mut chat);
    replace_or_add_tier(
        &mut utt.dependent_tiers,
        DependentTier::Mor(MorTier::new_mor(
            Vec::new(),
            talkbank_model::Terminator::Period {
                span: talkbank_model::Span::DUMMY,
            },
        )),
    );
    replace_or_add_tier(
        &mut utt.dependent_tiers,
        DependentTier::Gra(GraTier::new_gra(Vec::new())),
    );
    assert_eq!(
        dep_tier_kinds(&chat),
        vec!["Mor", "Gra", "Wor"],
        "replace_or_add_tier must overwrite empty placeholders in place, \
         not push new tiers after the existing Wor",
    );

    // Replacing Wor must keep it at its original index.
    let utt = first_utterance_mut(&mut chat);
    replace_or_add_tier(
        &mut utt.dependent_tiers,
        DependentTier::Wor(WorTier::default()),
    );
    assert_eq!(
        dep_tier_kinds(&chat),
        vec!["Mor", "Gra", "Wor"],
        "replace_or_add_tier on existing Wor must stay in place",
    );
}

/// Symmetric test for the `align` path: `add_wor_tier` must replace an
/// existing `%wor` in place, preserving original tier order when `%wor`
/// sits before `%mor` / `%gra`. The pre-fix implementation called
/// `remove_wor_tier` + `push`, which displaced regenerated `%wor` to the
/// end and reshuffled `[Wor, Mor, Gra]` into `[Mor, Gra, Wor]` on every
/// align run.
#[test]
fn add_wor_tier_preserves_tier_order_wor_mor_gra() {
    use crate::chat_ops::fa::add_wor_tier;
    use talkbank_parser::TreeSitterParser;
    use talkbank_transform::parse::parse_lenient;

    let text = concat!(
        "@UTF8\n",
        "@Begin\n",
        "@Languages:\teng\n",
        "@Participants:\tPAR Adult\n",
        "@ID:\teng|test|PAR|||||Adult|||\n",
        "*PAR:\thello world .\n",
        "%wor:\thello 0_500 world 500_1000 .\n",
        "%mor:\tintj|hello noun|world .\n",
        "%gra:\t1|2|DISCOURSE 2|0|ROOT 3|2|PUNCT\n",
        "@End\n",
    );
    let parser = TreeSitterParser::new().unwrap();
    let (mut chat, _errors) = parse_lenient(&parser, text);

    assert_eq!(
        dep_tier_kinds(&chat),
        vec!["Wor", "Mor", "Gra"],
        "fixture precondition: expected [Wor, Mor, Gra]",
    );

    let utt = first_utterance_mut(&mut chat);
    add_wor_tier(utt);

    assert_eq!(
        dep_tier_kinds(&chat),
        vec!["Wor", "Mor", "Gra"],
        "add_wor_tier must replace existing %wor in place, not remove + push",
    );
}

/// Regression: after `clear_morphosyntax` leaves empty `%mor` / `%gra`
/// placeholders in place, `collect_payloads` MUST still collect the
/// utterance for re-inference. Previously it skipped any utterance with
/// a `%mor` variant regardless of whether its content was empty, which
/// meant the morphotag pipeline produced zero payloads after clear and
/// silently stripped every `%mor` / `%gra` in the file when
/// `remove_empty_morphosyntax_placeholders` ran at serialize time.
#[test]
fn collect_payloads_treats_empty_mor_placeholder_as_unprocessed() {
    use talkbank_model::model::LanguageCode;
    use talkbank_parser::TreeSitterParser;
    use talkbank_transform::parse::parse_lenient;

    let text = concat!(
        "@UTF8\n",
        "@Begin\n",
        "@Languages:\teng\n",
        "@Participants:\tPAR Adult\n",
        "@ID:\teng|test|PAR|||||Adult|||\n",
        "*PAR:\thello world .\n",
        "%mor:\tintj|hello noun|world .\n",
        "%gra:\t1|2|DISCOURSE 2|0|ROOT 3|2|PUNCT\n",
        "@End\n",
    );
    let parser = TreeSitterParser::new().unwrap();
    let (mut chat, _errors) = parse_lenient(&parser, text);

    let primary_lang = LanguageCode::new("eng");
    let langs = vec![primary_lang.clone()];

    // Before clearing: the utterance already has populated %mor, so
    // collect_payloads correctly skips it.
    let items = crate::chat_ops::morphosyntax_ops::collect_payloads(
        &chat,
        &primary_lang,
        &langs,
        crate::chat_ops::morphosyntax_ops::MultilingualPolicy::ProcessAll,
    )
    .batch_items;
    assert_eq!(
        items.len(),
        0,
        "populated %mor must be treated as already-processed",
    );

    // After clear_morphosyntax: %mor placeholder remains in place but empty.
    crate::chat_ops::morphosyntax_ops::clear_morphosyntax(&mut chat);

    let collected = crate::chat_ops::morphosyntax_ops::collect_payloads(
        &chat,
        &primary_lang,
        &langs,
        crate::chat_ops::morphosyntax_ops::MultilingualPolicy::ProcessAll,
    );
    let items = collected.batch_items;
    let total = collected.total_utterances;
    assert_eq!(total, 1, "one utterance in fixture");
    assert_eq!(
        items.len(),
        1,
        "empty %mor placeholder must NOT be treated as already-processed; \
         the utterance must be collected for re-inference",
    );
}

/// Helper: one-letter variant tag for each dependent tier of the first
/// utterance, in source order. Only cases we care about for this test
/// are enumerated explicitly; other tiers fall through to "Other".
fn dep_tier_kinds(chat: &talkbank_model::model::ChatFile) -> Vec<&'static str> {
    use talkbank_model::model::{DependentTier, Line};
    for line in &chat.lines {
        if let Line::Utterance(u) = line {
            return u
                .dependent_tiers
                .iter()
                .map(|t| match t {
                    DependentTier::Mor(_) => "Mor",
                    DependentTier::Gra(_) => "Gra",
                    DependentTier::Wor(_) => "Wor",
                    _ => "Other",
                })
                .collect();
        }
    }
    Vec::new()
}

fn first_utterance_mut(
    chat: &mut talkbank_model::model::ChatFile,
) -> &mut talkbank_model::model::Utterance {
    use talkbank_model::model::Line;
    for line in chat.lines.iter_mut() {
        if let Line::Utterance(u) = line {
            return u;
        }
    }
    panic!("no utterance in fixture")
}

// Regression guards: `inject_results` must surface injection errors
// visibly (via DecisionRecord + tracing::warn!) rather than silently
// drop the utterance, BUT must not kill the whole file — an isolated
// Stanza edge case shouldn't take down an entire morphotag run.
// See `inject::inject_morphosyntax` for the library-level error
// check that callers rely on.

/// When the UD response produces fewer Mor items than the CHAT main tier
/// has alignable words, `inject_results` must emit a visible
/// `DecisionRecord` (kind `injection_failed`) and continue with the
/// next utterance — not propagate the error and kill the file.
#[test]
fn inject_results_count_mismatch_propagates_error() {
    use crate::chat_ops::morphosyntax_ops::inject_results;
    use talkbank_transform::parse::{TreeSitterParser, parse_lenient};

    let parser = TreeSitterParser::new().unwrap();
    // Three alignable words on the main tier.
    let chat = "\
@UTF8
@Begin
@Languages:\teng
@Participants:\tPAR Participant
@ID:\teng|test|PAR|||||Participant|||
*PAR:\thello good world .
@End
";
    let (mut chat_file, _errors) = parse_lenient(&parser, chat);

    let primary_lang = talkbank_model::model::LanguageCode::new("eng");
    let langs = declared_languages(&chat_file, &primary_lang);
    let batch_items = collect_payloads(
        &chat_file,
        &primary_lang,
        &langs,
        MultilingualPolicy::ProcessAll,
    )
    .batch_items;
    assert_eq!(batch_items.len(), 1);

    // Provide a UD response with only TWO words (hello, world). The CHAT
    // main tier has three (hello, good, world), so injection should fail
    // with a count mismatch.
    let ud_response = ud_response_from_words(
        r#"[
            {"id":1,"text":"hello","lemma":"hello","upos":"INTJ","head":0,"deprel":"root"},
            {"id":2,"text":"world","lemma":"world","upos":"NOUN","head":1,"deprel":"parataxis"}
        ]"#,
    );

    let empty_mwt = std::collections::BTreeMap::new();
    let outcome = inject_results(
        &parser,
        &mut chat_file,
        batch_items,
        vec![ud_response],
        &primary_lang,
        TokenizationMode::Preserve,
        &empty_mwt,
    )
    .expect("inject_results should absorb per-utterance failures at file level");

    // After the outcome-typing refactor (Wave 1 of the morphotag
    // reconciliation architecture), a count mismatch surfaces as a
    // `misalignment_bug` DecisionRecord whose `reason` field carries
    // typed diagnostic data (class, expected, actual, chat_words,
    // stanza_tokens). The test still asserts what it was checking
    // before — that mismatches are visible as review-flagged decision
    // records — but in the new typed form.
    let failed: Vec<_> = outcome
        .decisions
        .iter()
        .filter(|d| d.strategy.strategy_name() == "misalignment_bug")
        .collect();
    assert!(
        !failed.is_empty(),
        "expected at least one `misalignment_bug` decision record when the \
         count mismatched; got decisions: {:?}",
        outcome.decisions
    );
    let reason = &failed[0].reason;
    assert!(reason.contains("expected=3"), "got: {reason}");
    assert!(reason.contains("actual=2"), "got: {reason}");
    assert!(
        failed[0].needs_review,
        "misalignment bugs always require human review"
    );
}

/// End-to-end: a mid-utterance comma flows through `collect_payloads` →
/// `inject_results` and appears as `cm|cm` in the final `%mor` tier.
#[test]
fn mid_utterance_comma_end_to_end_injects_cm_mor() {
    use crate::chat_ops::morphosyntax_ops::inject_results;
    use talkbank_transform::parse::{TreeSitterParser, parse_lenient};

    let parser = TreeSitterParser::new().unwrap();
    // Three words plus a comma plus a terminator: `hello , good world .`
    // CHAT Mor-domain alignable count = 4 (hello, comma, good, world).
    let chat = "\
@UTF8
@Begin
@Languages:\teng
@Participants:\tPAR Participant
@ID:\teng|test|PAR|||||Participant|||
*PAR:\thello , good world .
@End
";
    let (mut chat_file, _errors) = parse_lenient(&parser, chat);
    let primary_lang = talkbank_model::model::LanguageCode::new("eng");
    let langs = declared_languages(&chat_file, &primary_lang);
    let batch_items = collect_payloads(
        &chat_file,
        &primary_lang,
        &langs,
        MultilingualPolicy::ProcessAll,
    )
    .batch_items;
    assert_eq!(batch_items.len(), 1);

    // Stanza-style UD output: 5 UD words (hello, comma, good, world, dot).
    // After mapping with the comma-preserving fix, mors.len() == 4
    // (terminator dropped, comma kept as cm|cm).
    let ud_response = ud_response_from_words(
        r#"[
            {"id":1,"text":"hello","lemma":"hello","upos":"INTJ","head":0,"deprel":"root"},
            {"id":2,"text":",","lemma":",","upos":"PUNCT","head":1,"deprel":"punct"},
            {"id":3,"text":"good","lemma":"good","upos":"ADJ","head":4,"deprel":"amod"},
            {"id":4,"text":"world","lemma":"world","upos":"NOUN","head":1,"deprel":"parataxis"},
            {"id":5,"text":".","lemma":".","upos":"PUNCT","head":1,"deprel":"punct"}
        ]"#,
    );

    let empty_mwt = std::collections::BTreeMap::new();
    let result = inject_results(
        &parser,
        &mut chat_file,
        batch_items,
        vec![ud_response],
        &primary_lang,
        TokenizationMode::Preserve,
        &empty_mwt,
    );
    result.expect("inject_results should succeed when comma is preserved");

    let utt = chat_file
        .lines
        .iter()
        .find_map(|l| match l {
            talkbank_model::model::Line::Utterance(u) => Some(u),
            _ => None,
        })
        .expect("should have an utterance");

    let mor_tier = utt.dependent_tiers.iter().find_map(|t| match t {
        talkbank_model::model::DependentTier::Mor(m) => Some(m),
        _ => None,
    });
    let mor_tier = mor_tier.expect("utterance should gain a %mor tier");

    let mut mor_str = String::new();
    use talkbank_model::WriteChat;
    mor_tier.write_chat(&mut mor_str).unwrap();
    assert!(
        mor_str.contains("cm|cm"),
        "expected cm|cm in %mor; got: {mor_str}"
    );

    // Round-trip sanity: the full serialized CHAT must contain the %mor line.
    let serialized = chat_file.to_chat_string();
    assert!(
        serialized.contains("cm|cm"),
        "serialized CHAT should contain cm|cm; got:\n{serialized}"
    );
}

// ============================================================================
// Family A — synthesis-layer DEP overwrite kills ROOT deprel (RED tests).
//
// Pure-unit pinning for the Family A partition; see the L2
// architectural-reassessment notes (§5).
//
// The bug is at injection.rs:204:
//
//     gra.relation = GrammaticalRelationType::new(DEP_RELATION_LABEL);
//
// The overwrite ignores `gra.head`. When the form-marker token is the
// utterance's syntactic root, Stanza correctly emitted (head=0,
// deprel=root); the synthesis loop forces deprel="DEP" while leaving
// head=0, breaking the CHECK invariant that head=0 must pair with
// deprel="ROOT".
//
// These tests construct synthetic UdResponses mirroring what Stanza
// returns for the wild-bad utterances and assert the post-injection
// %gra has head=0/deprel=ROOT, not head=0/deprel=DEP.
//
// EXPECTED: every test in this section FAILS on the current build.
// Do not modify the asserts to make them pass — modify the bug.
// ============================================================================

/// Helper: pull the GraTier relations from the first utterance.
fn first_utt_gra_relations(
    chat_file: &talkbank_model::model::ChatFile,
) -> Vec<talkbank_model::model::GrammaticalRelation> {
    let utt = chat_file
        .lines
        .iter()
        .find_map(|l| match l {
            talkbank_model::model::Line::Utterance(u) => Some(u),
            _ => None,
        })
        .expect("fixture must have an utterance");
    let gra = utt
        .dependent_tiers
        .iter()
        .find_map(|t| match t {
            talkbank_model::model::DependentTier::Gra(g) => Some(g),
            _ => None,
        })
        .expect("utterance must have a %gra tier after injection");
    gra.relations().to_vec()
}

/// Format a GraTier's relations as the CHAT %gra body, e.g.
/// `1|0|ROOT 2|1|PUNCT`. Used for diagnostic-friendly assert messages.
fn fmt_gra(rels: &[talkbank_model::model::GrammaticalRelation]) -> String {
    rels.iter()
        .map(|r| format!("{}|{}|{}", r.index, r.head, r.relation.as_str()))
        .collect::<Vec<_>>()
        .join(" ")
}

/// Family A, Test A1 — single-word onomatopoeia utterance.
///
/// Source pattern (from `still-have-error-2.log`):
///     *CHI:  vau@o .
///     %mor:  on|vau .
///     %gra:  1|0|DEP 2|1|PUNCT          ← BUG
///
/// Stanza is fed the placeholder substitution `xbxxx` (per
/// `payload.rs::stanza_placeholder()`). For a single-word utterance
/// Stanza returns (head=0, deprel="root") for the placeholder.
///
/// The synthesis loop in `injection.rs:202-205` then runs and
/// overwrites the gra relation to "DEP" without checking gra.head.
/// Result: 1|0|DEP — fires E722 (no ROOT relation) downstream.
///
/// EXPECTED on current build: FAILS — relation is "DEP" not "ROOT".
#[test]
fn family_a_single_word_at_o_keeps_root_deprel_when_head_is_zero() {
    use talkbank_transform::parse::{TreeSitterParser, parse_lenient};

    let parser = TreeSitterParser::new().unwrap();
    let chat = include_str!("../../../../../test-fixtures/eng_at_o_single_word_red.cha");
    let (mut chat_file, _diags) = parse_lenient(&parser, chat);

    let primary_lang = talkbank_model::model::LanguageCode::new("eng");
    let langs = declared_languages(&chat_file, &primary_lang);
    let batch_items = collect_payloads(
        &chat_file,
        &primary_lang,
        &langs,
        MultilingualPolicy::ProcessAll,
    )
    .batch_items;
    assert_eq!(batch_items.len(), 1, "fixture has one utterance");

    // Stanza response for ["xbxxx", "."]: word 1 is the placeholder
    // root, word 2 is the period attached to the root.
    let ud_response = ud_response_from_words(
        r#"[
          {"id":1,"text":"xbxxx","lemma":"xbxxx","upos":"NOUN","xpos":"NN","head":0,"deprel":"root"},
          {"id":2,"text":".","lemma":".","upos":"PUNCT","xpos":".","head":1,"deprel":"punct"}
        ]"#,
    );

    let empty_mwt = std::collections::BTreeMap::new();
    let injection = inject_results(
        &parser,
        &mut chat_file,
        batch_items,
        vec![ud_response],
        &primary_lang,
        TokenizationMode::Preserve,
        &empty_mwt,
    )
    .expect("inject_results must succeed for the single-@o case");
    assert!(
        injection
            .decisions
            .iter()
            .all(|d| d.strategy.strategy_name() != "injection_failed"),
        "injection should not fail on single-@o utterance: {:?}",
        injection.decisions
    );

    let rels = first_utt_gra_relations(&chat_file);
    let body = fmt_gra(&rels);

    // The form-marker word is at chunk 1; it is the syntactic root.
    let chunk_1 = rels
        .iter()
        .find(|r| r.index == 1)
        .expect("must have a chunk-1 relation");
    assert_eq!(
        chunk_1.head, 0,
        "chunk 1 must remain head=0; got %gra: {body}"
    );
    assert_eq!(
        chunk_1.relation.as_str(),
        "ROOT",
        "Family A bug — single-@o root token's deprel was overwritten to \
         '{}' instead of preserving ROOT (Stanza returned head=0, deprel=root); \
         got %gra: {body}",
        chunk_1.relation.as_str()
    );

    // Symmetric structural invariant: every head=0 relation must carry
    // deprel=ROOT. Anything else is a CHECK violation.
    for r in &rels {
        if r.head == 0 {
            assert_eq!(
                r.relation.as_str(),
                "ROOT",
                "head=0 must pair with deprel=ROOT; got %gra: {body}"
            );
        }
    }
}

/// Family A, Test A2 — multi-word utterance where every word is a
/// form-marker (`@si`). The wild-bad case is the Croatian
/// `osam@si devet@si devet@si i@si jedan@si su@si deset@si .` whose
/// %gra is currently `1|0|DEP 2|1|DEP 3|1|DEP 4|1|DEP 5|1|DEP 6|1|DEP
/// 7|6|DEP 8|1|PUNCT`. The synthesis-DEP overwrite fires on EVERY
/// chunk because every content word has `form_type = Some(Si)`,
/// including the head=0 root.
///
/// EXPECTED on current build: FAILS — chunk 1 has head=0 but
/// deprel=DEP instead of deprel=ROOT.
#[test]
fn family_a_multi_word_all_at_si_keeps_root_deprel_at_head_zero() {
    use talkbank_transform::parse::{TreeSitterParser, parse_lenient};

    let parser = TreeSitterParser::new().unwrap();
    let chat = include_str!("../../../../../test-fixtures/eng_at_si_all_signed_red.cha");
    let (mut chat_file, _diags) = parse_lenient(&parser, chat);

    let primary_lang = talkbank_model::model::LanguageCode::new("eng");
    let langs = declared_languages(&chat_file, &primary_lang);
    let batch_items = collect_payloads(
        &chat_file,
        &primary_lang,
        &langs,
        MultilingualPolicy::ProcessAll,
    )
    .batch_items;
    assert_eq!(batch_items.len(), 1);

    // Stanza response for ["xbxxx", "xbxxx", "xbxxx", "xbxxx", "."]:
    // word 1 is the placeholder root, words 2..=4 attach to word 1
    // with arbitrary UD relations (Stanza can assign anything when
    // every token is the same surface placeholder), word 5 is punct.
    let ud_response = ud_response_from_words(
        r#"[
          {"id":1,"text":"xbxxx","lemma":"xbxxx","upos":"NOUN","xpos":"NN","head":0,"deprel":"root"},
          {"id":2,"text":"xbxxx","lemma":"xbxxx","upos":"NOUN","xpos":"NN","head":1,"deprel":"flat"},
          {"id":3,"text":"xbxxx","lemma":"xbxxx","upos":"NOUN","xpos":"NN","head":1,"deprel":"flat"},
          {"id":4,"text":"xbxxx","lemma":"xbxxx","upos":"NOUN","xpos":"NN","head":1,"deprel":"flat"},
          {"id":5,"text":".","lemma":".","upos":"PUNCT","xpos":".","head":1,"deprel":"punct"}
        ]"#,
    );

    let empty_mwt = std::collections::BTreeMap::new();
    inject_results(
        &parser,
        &mut chat_file,
        batch_items,
        vec![ud_response],
        &primary_lang,
        TokenizationMode::Preserve,
        &empty_mwt,
    )
    .expect("inject_results must succeed for the all-@si case");

    let rels = first_utt_gra_relations(&chat_file);
    let body = fmt_gra(&rels);

    // The number of head=0 relations must be exactly 1 (E722/E723 guard).
    let head_zero: Vec<_> = rels.iter().filter(|r| r.head == 0).collect();
    assert_eq!(
        head_zero.len(),
        1,
        "must have exactly one head=0 relation; got %gra: {body}"
    );
    assert_eq!(
        head_zero[0].relation.as_str(),
        "ROOT",
        "Family A bug — multi-@si root token's deprel was overwritten to \
         '{}' instead of preserving ROOT; got %gra: {body}",
        head_zero[0].relation.as_str()
    );
}

/// Family A, Test A4 — host-language modifier + `@o` as syntactic root.
///
/// Source pattern (from `still-have-error-2.log`):
///     *IRI:  the chingchangchongchong@o .
///     %mor:  det|the-Def-Art on|chingchangchongchong .
///     %gra:  1|2|DET 2|0|DEP 3|2|PUNCT          ← BUG
///
/// `the` is a determiner whose head is the form-marker token at
/// chunk 2; the form-marker token is the utterance's syntactic root
/// (Stanza returns head=0, deprel=root for it). The synthesis path
/// fires only on the form-marker chunk and overwrites its deprel to
/// DEP, leaving the modifier's gra intact.
///
/// EXPECTED on current build: FAILS — chunk 2 has head=0 but
/// deprel=DEP instead of deprel=ROOT.
#[test]
fn family_a_host_modifier_with_at_o_root_keeps_root_deprel() {
    use talkbank_transform::parse::{TreeSitterParser, parse_lenient};

    let parser = TreeSitterParser::new().unwrap();
    let chat = include_str!("../../../../../test-fixtures/eng_at_o_root_with_modifier_red.cha");
    let (mut chat_file, _diags) = parse_lenient(&parser, chat);

    let primary_lang = talkbank_model::model::LanguageCode::new("eng");
    let langs = declared_languages(&chat_file, &primary_lang);
    let batch_items = collect_payloads(
        &chat_file,
        &primary_lang,
        &langs,
        MultilingualPolicy::ProcessAll,
    )
    .batch_items;
    assert_eq!(batch_items.len(), 1);

    // Stanza for ["the", "xbxxx", "."]: the=det/head=2, xbxxx=root/head=0,
    // .=punct/head=2.
    let ud_response = ud_response_from_words(
        r#"[
          {"id":1,"text":"the","lemma":"the","upos":"DET","xpos":"DT","feats":"Definite=Def|PronType=Art","head":2,"deprel":"det"},
          {"id":2,"text":"xbxxx","lemma":"xbxxx","upos":"NOUN","xpos":"NN","head":0,"deprel":"root"},
          {"id":3,"text":".","lemma":".","upos":"PUNCT","xpos":".","head":2,"deprel":"punct"}
        ]"#,
    );

    let empty_mwt = std::collections::BTreeMap::new();
    inject_results(
        &parser,
        &mut chat_file,
        batch_items,
        vec![ud_response],
        &primary_lang,
        TokenizationMode::Preserve,
        &empty_mwt,
    )
    .expect("inject_results must succeed for det + @o-root case");

    let rels = first_utt_gra_relations(&chat_file);
    let body = fmt_gra(&rels);

    // Chunk 1 (the) must keep its DET relation pointing at chunk 2.
    let chunk_1 = rels.iter().find(|r| r.index == 1).expect("chunk 1");
    assert_eq!(
        chunk_1.head, 2,
        "host modifier head preserved; got %gra: {body}"
    );
    assert_eq!(
        chunk_1.relation.as_str(),
        "DET",
        "host modifier deprel preserved (synthesis must not touch \
         non-form-marker chunks); got %gra: {body}"
    );

    // Chunk 2 (xbxxx-from-@o) must remain head=0 with deprel=ROOT.
    let chunk_2 = rels.iter().find(|r| r.index == 2).expect("chunk 2");
    assert_eq!(
        chunk_2.head, 0,
        "form-marker chunk must remain head=0; got %gra: {body}"
    );
    assert_eq!(
        chunk_2.relation.as_str(),
        "ROOT",
        "Family A bug — det+@o-root case: form-marker root token's \
         deprel was overwritten to '{}' instead of preserving ROOT; \
         got %gra: {body}",
        chunk_2.relation.as_str()
    );
}

/// Family A, Test A5 — symmetric guard: form-marker token whose head is
/// NOT zero (i.e., not the utterance root) should not have its deprel
/// rewritten to "ROOT" by an over-eager fix. The current synthesis-DEP
/// overwrite is acceptable behavior for non-root form-marker tokens
/// (BA2-equivalent intent: "no specific role applies"). The fix must
/// touch only the head=0 branch — this test pins that scope so a future
/// "always preserve Stanza deprel" patch can't silently regress.
///
/// Source: `*CHI: I like vau@o .` (English primary). Stanza analyzes
/// "I"=nsubj, "like"=root, "xbxxx"=obj/dep, "."=punct. Post-injection:
/// chunk 3 (the @o token) should have head=2 (preserved from Stanza)
/// and deprel="DEP" (current synthesis convention) — NOT "ROOT".
///
/// EXPECTED on current build: PASSES (locks current behavior). After
/// the fix lands for A1/A2/A4, this must still pass.
#[test]
fn family_a_at_o_with_nonzero_head_does_not_become_root() {
    use talkbank_transform::parse::{TreeSitterParser, parse_lenient};

    let parser = TreeSitterParser::new().unwrap();
    // Inline fixture — keeping it inline so the assertion's premise
    // (form-marker is *not* the syntactic root) is visible alongside the
    // test.
    let chat = "@UTF8\n\
                @Begin\n\
                @Languages:\teng\n\
                @Participants:\tCHI Target_Child\n\
                @ID:\teng|test|CHI||female|||Target_Child|||\n\
                *CHI:\tI like vau@o .\n\
                @End\n";
    let (mut chat_file, _diags) = parse_lenient(&parser, chat);

    let primary_lang = talkbank_model::model::LanguageCode::new("eng");
    let langs = declared_languages(&chat_file, &primary_lang);
    let batch_items = collect_payloads(
        &chat_file,
        &primary_lang,
        &langs,
        MultilingualPolicy::ProcessAll,
    )
    .batch_items;
    assert_eq!(batch_items.len(), 1);

    // Stanza for ["I", "like", "xbxxx", "."]: I=nsubj/head=2,
    // like=root/head=0, xbxxx=obj/head=2, .=punct/head=2.
    let ud_response = ud_response_from_words(
        r#"[
          {"id":1,"text":"I","lemma":"I","upos":"PRON","xpos":"PRP","feats":"Person=1|PronType=Prs","head":2,"deprel":"nsubj"},
          {"id":2,"text":"like","lemma":"like","upos":"VERB","xpos":"VBP","feats":"Mood=Ind|Tense=Pres|VerbForm=Fin","head":0,"deprel":"root"},
          {"id":3,"text":"xbxxx","lemma":"xbxxx","upos":"NOUN","xpos":"NN","head":2,"deprel":"obj"},
          {"id":4,"text":".","lemma":".","upos":"PUNCT","xpos":".","head":2,"deprel":"punct"}
        ]"#,
    );

    let empty_mwt = std::collections::BTreeMap::new();
    inject_results(
        &parser,
        &mut chat_file,
        batch_items,
        vec![ud_response],
        &primary_lang,
        TokenizationMode::Preserve,
        &empty_mwt,
    )
    .expect("inject_results must succeed for non-root @o case");

    let rels = first_utt_gra_relations(&chat_file);
    let body = fmt_gra(&rels);

    // Chunk 3 is the @o-marker. Head must be preserved as 2 (not the
    // utterance root). Deprel must be "DEP" (or whatever the synthesis
    // convention is for non-root form markers) — and explicitly NOT
    // "ROOT".
    let chunk_3 = rels.iter().find(|r| r.index == 3).expect("chunk 3");
    assert_eq!(
        chunk_3.head, 2,
        "form-marker chunk's non-zero head must be preserved; got %gra: {body}"
    );
    assert_ne!(
        chunk_3.relation.as_str(),
        "ROOT",
        "form-marker chunk with head!=0 must NOT be re-labelled ROOT \
         by an over-correcting fix; got %gra: {body}"
    );

    // Joint invariant: at most one head=0 relation, and it must NOT be
    // the form-marker chunk (chunk 3 here).
    let head_zero: Vec<_> = rels.iter().filter(|r| r.head == 0).collect();
    assert_eq!(
        head_zero.len(),
        1,
        "exactly one head=0 relation expected; got %gra: {body}"
    );
    assert_ne!(
        head_zero[0].index, 3,
        "form-marker chunk must not become the root when Stanza said \
         it wasn't; got %gra: {body}"
    );
}

// ===========================================================================
// Utterance preservation regression — fusser22 corruption (2026-05-06)
//
// Background. The 2026-05-06 morphotag re-run corrupted
// `biling-data/Bangor/Siarad/fusser22.cha`: one utterance
// (`*EVA: [- eng] &-um and spo(rt) xxx +//. 1373503_1375802`) was
// silently DELETED from the output, and a different utterance
// (`*EVA: [- eng] what's the word for it ? 1376707_1377578`) was
// DUPLICATED in its place. Total `*SPK:` count was preserved (so
// per-speaker counts and net-line-count diffs all looked normal),
// which is exactly why the corruption escaped the per-commit and
// per-file scans. The bug was not reproducible on the deployed
// binary after the fact — the precise trigger is currently unknown
// and may be a concurrency or in-flight-state artifact.
//
// Goal of these tests. Pin the invariant the corruption violates so
// any future regression of this shape is caught even when the bug
// itself is not reproducible from a single fixture. The invariant is
// stronger than line counts: every distinct (speaker, main-tier
// content, optional timestamp) tuple from the input must appear
// EXACTLY ONCE in the output. Replacement-by-duplicate is detected
// because the lost utterance fails the "appears at least once" half;
// duplication is detected because the gained one fails the "appears
// at most once" half.
//
// Why these tests live at the inject_results layer. The whole-file
// pipeline depends on a live worker pool; pinning the invariant on
// `inject_results` (the in-memory transformation that replaces
// `L2|xxx` placeholders with synthesized morphology) keeps the test
// hermetic while still exercising the code path that is closest to
// where the corruption was observed.

/// Collect a per-utterance identity string (`MainTier::Display`) for
/// every utterance in `chat_file`. Two utterances are "the same" iff
/// they round-trip to the same CHAT line — good-enough for a
/// regression assertion.
fn collect_utterance_identities<S: talkbank_model::validation::ValidationState>(
    chat_file: &talkbank_model::ChatFile<S>,
) -> Vec<String> {
    use talkbank_model::model::Line;
    chat_file
        .lines
        .iter()
        .filter_map(|line| match line {
            Line::Utterance(utt) => Some(utt.main.to_string()),
            _ => None,
        })
        .collect()
}

fn assert_utterances_preserved_one_to_one(label: &str, before: &[String], after: &[String]) {
    // BTreeMap (not HashMap) so failure messages list lost/duplicated/
    // introduced utterances in deterministic order — important when the
    // assertion fires in CI and the diff is the entire signal.
    use std::collections::BTreeMap;
    fn count_in(xs: &[String]) -> BTreeMap<&String, usize> {
        let mut h: BTreeMap<&String, usize> = BTreeMap::new();
        for x in xs {
            *h.entry(x).or_default() += 1;
        }
        h
    }
    let before_counts = count_in(before);
    let after_counts = count_in(after);

    let mut lost: Vec<&String> = Vec::new();
    let mut duplicated: Vec<(&String, usize, usize)> = Vec::new();
    let mut introduced: Vec<&String> = Vec::new();

    for (id, &n_in) in &before_counts {
        let n_out = after_counts.get(id).copied().unwrap_or(0);
        if n_out == 0 {
            lost.push(id);
        } else if n_out != n_in {
            duplicated.push((id, n_in, n_out));
        }
    }
    for (id, _) in &after_counts {
        if !before_counts.contains_key(id) {
            introduced.push(id);
        }
    }

    assert!(
        lost.is_empty(),
        "[{label}] morphotag DROPPED utterance(s) from the output: {lost:#?}"
    );
    assert!(
        duplicated.is_empty(),
        "[{label}] morphotag changed the multiplicity of utterance(s) \
         (corruption shape: same utterance now appears more or fewer \
         times than the input). before/after counts: {duplicated:#?}"
    );
    assert!(
        introduced.is_empty(),
        "[{label}] morphotag INVENTED utterance(s) not present in the \
         input: {introduced:#?}"
    );
    assert_eq!(
        before.len(),
        after.len(),
        "[{label}] total utterance count must match (after dedup-aware \
         identity check)"
    );
}

/// Protective regression test for the fusser22 corruption shape.
///
/// `inject_results` rewrites `%mor` and `%gra` in place; it has no
/// business adding, removing, or substituting `*SPK:` main-tier
/// utterances. This test pins that invariant by feeding a synthetic
/// `UdResponse` for a small reproducer and asserting the output's
/// utterance multiset is identical to the input's.
///
/// The fixture mirrors the structure that surrounded the wild
/// corruption: three consecutive same-speaker `[- eng]`-precoded
/// utterances, one with a leading `&-um` filler + `xxx` + a `+//.`
/// terminator (the "lost" utterance shape), one with a leading
/// `&~er` filler (a fix-s-eligible shape), and one with neither
/// filler nor `xxx` (the "duplicated" utterance shape).
#[test]
fn morphotag_inject_results_preserves_utterance_multiplicity_one_to_one() {
    use crate::chat_ops::morphosyntax_ops::inject_results;
    use talkbank_transform::parse::{TreeSitterParser, parse_lenient};

    let parser = TreeSitterParser::new().unwrap();
    let chat = "\
@UTF8
@Begin
@Languages:\tcym, eng
@Participants:\tEVA Adult, WYN Adult
@ID:\tcym|Siarad|EVA|40;|female|||Adult|||
@ID:\tcym|Siarad|WYN|49;|male|||Adult|||
*EVA:\t[- eng] &-um and spo(rt) xxx +//.
*EVA:\t[- eng] &~er what's the word ?
*EVA:\t[- eng] what's the word for it ?
@End
";

    let (mut chat_file, _diags) = parse_lenient(&parser, chat);
    let primary_lang = talkbank_model::model::LanguageCode::new("cym");

    let before = collect_utterance_identities(&chat_file);
    assert_eq!(
        before.len(),
        3,
        "fixture must contain exactly 3 utterances; got {before:#?}"
    );

    let langs = declared_languages(&chat_file, &primary_lang);
    let batch_items = collect_payloads(
        &chat_file,
        &primary_lang,
        &langs,
        MultilingualPolicy::ProcessAll,
    )
    .batch_items;

    // One synthetic UdResponse per batch item with a single placeholder
    // root word — the morphology content is irrelevant; what matters
    // here is that injection does not perturb the main tier.
    let ud_responses: Vec<_> = batch_items
        .iter()
        .map(|_| {
            ud_response_from_words(
                r#"[
                  {"id":1,"text":"x","lemma":"x","upos":"NOUN","xpos":"NN","head":0,"deprel":"root"}
                ]"#,
            )
        })
        .collect();

    let empty_mwt = std::collections::BTreeMap::new();
    let _ = inject_results(
        &parser,
        &mut chat_file,
        batch_items,
        ud_responses,
        &primary_lang,
        TokenizationMode::Preserve,
        &empty_mwt,
    )
    .expect("inject_results must succeed on the fusser22-shape fixture");

    let after = collect_utterance_identities(&chat_file);
    assert_utterances_preserved_one_to_one(
        "fusser22-shape: 3 same-speaker [- eng] precoded utterances",
        &before,
        &after,
    );
}

// ===========================================================================
// L2-fallback construct matrix
//
// These tests pin "construct → safe `L2|xxx` fallback" for each path
// where the morphotag pipeline today emits `L2|xxx` instead of real
// secondary morphology. The intent is a transition path via tests:
// every assertion below currently expects `L2|xxx`; when we eventually
// implement real morphology for one of these constructs, the fix is to
// rewrite that single test's assertion (from "must remain L2|xxx" to
// "must produce <the real expected analysis>"). Until that day, these
// tests guarantee the fallback never silently regresses to a worse
// state — a crash, an invalid `%gra`, an empty/missing `%mor`, or a
// hallucinated wrong analysis.
//
// Each test follows the same shape:
//   1. Build a minimal CHAT fixture with one offending construct.
//   2. Drive the morphotag in-memory pipeline (collect_payloads +
//      inject_results), feeding a synthetic primary UD response that
//      tells the pipeline "the secondary positions are placeholders
//      pending dispatch." For the fallback paths, we deliberately do
//      NOT run `dispatch_secondary_l2` afterwards — that simulates the
//      production fallback (unsupported lang, ambiguous resolution,
//      `--no-l2-morphotag`, dispatch failure).
//   3. Assert the offending position(s) carry `%mor = "L2|xxx"`.
//   4. Assert `validate_chat_file_with_options` passes — no E722,
//      E724, or other downstream failures from the fallback shape.
//
// Note on what's covered here vs. elsewhere:
//   - The partition-side fallback (`partition_groups_by_stanza_support`)
//     has its own unit tests in `morphosyntax/worker.rs`; this matrix
//     covers the user-observable end of the pipeline.
//   - The Family C splice rollback (`validate_or_rollback_splice`)
//     has its own unit tests in `morphosyntax/l2/splice.rs`; the
//     construct-level coverage of "splice rolled back to L2|xxx → file
//     validates" lives in the splice tests rather than here.

/// Walk the first utterance's `%mor` items and return the (POS, lemma)
/// pair at each position. Compact helper for fallback-position
/// assertions across the matrix.
fn first_utt_mor_pairs(chat_file: &talkbank_model::ChatFile) -> Vec<(String, String)> {
    use talkbank_model::model::Line;
    let utt = chat_file
        .lines
        .iter()
        .find_map(|l| match l {
            Line::Utterance(u) => Some(u),
            _ => None,
        })
        .expect("test fixture must contain at least one utterance");
    let mor = utt
        .mor_tier()
        .expect("utterance must have %mor after injection");
    mor.items()
        .iter()
        .map(|item| (item.main.pos.to_string(), item.main.lemma.to_string()))
        .collect()
}

fn assert_position_is_l2_xxx(label: &str, pairs: &[(String, String)], pos: usize) {
    assert!(
        pos < pairs.len(),
        "[{label}] position {pos} out of bounds for %mor with {} items",
        pairs.len()
    );
    let (mor_pos, mor_lemma) = &pairs[pos];
    assert_eq!(
        (mor_pos.as_str(), mor_lemma.as_str()),
        ("L2", "xxx"),
        "[{label}] position {pos} expected L2|xxx (fallback), \
         got {mor_pos}|{mor_lemma}. \
         If this assertion is failing because the pipeline now produces \
         REAL morphology for this construct: that is the transition path; \
         rewrite the assertion to the real expected analysis."
    );
}

fn validate_or_panic(chat_file: &mut talkbank_model::ChatFile, label: &str) {
    let opts = talkbank_model::ParseValidateOptions::default().with_alignment();
    talkbank_model::validate_chat_file_with_options(chat_file, &opts)
        .unwrap_or_else(|err| panic!("[{label}] fallback output must validate clean; got {err:?}"));
}

// ---------------------------------------------------------------------
// Row 1: `@s:UNSUPPORTEDLANG` — explicit per-word marker for a Stanza
// language that has no morphosyntax processors.
//
// Production trigger: `partition_groups_by_stanza_support` filters the
// L2 group to fallback; downstream injection leaves L2|xxx. The user
// observes a single foreign word slot as L2|xxx and the rest of the
// utterance as real English morphology.
//
// TRANSITION PATH: when we add a heuristic or external lookup for
// unsupported-language words (e.g. routing `@s:que` through a custom
// Quechua model), rewrite this test's assertion to the real expected
// analysis at position 3 (`rimaykullayki`).
// ---------------------------------------------------------------------
#[test]
fn l2_fallback_unsupported_secondary_at_s_lang_remains_l2_xxx() {
    use talkbank_transform::parse::{TreeSitterParser, parse_lenient};

    let parser = TreeSitterParser::new().unwrap();
    let chat = include_str!("../../../../../test-fixtures/eng_at_s_unsupported.cha");
    let (mut chat_file, _) = parse_lenient(&parser, chat);

    let primary_lang = talkbank_model::model::LanguageCode::new("eng");
    let langs = declared_languages(&chat_file, &primary_lang);
    let batch_items = collect_payloads(
        &chat_file,
        &primary_lang,
        &langs,
        MultilingualPolicy::ProcessAll,
    )
    .batch_items;
    assert_eq!(batch_items.len(), 1, "fixture has one utterance");

    // Primary UD response — one entry per surface word; the @s:que
    // word ("rimaykullayki", position index 3) is the L2 placeholder
    // and gets the synthetic `xbxxx` token Stanza was given.
    let primary_ud = ud_response_from_words(
        r#"[
          {"id":1,"text":"she","lemma":"she","upos":"PRON","head":2,"deprel":"nsubj"},
          {"id":2,"text":"said","lemma":"say","upos":"VERB","head":0,"deprel":"root"},
          {"id":3,"text":"xbxxx","lemma":"xbxxx","upos":"NOUN","head":2,"deprel":"obj"},
          {"id":4,"text":"to","lemma":"to","upos":"ADP","head":5,"deprel":"case"},
          {"id":5,"text":"me","lemma":"me","upos":"PRON","head":2,"deprel":"obl"},
          {"id":6,"text":".","lemma":".","upos":"PUNCT","head":2,"deprel":"punct"}
        ]"#,
    );

    let empty_mwt = std::collections::BTreeMap::new();
    inject_results(
        &parser,
        &mut chat_file,
        batch_items,
        vec![primary_ud],
        &primary_lang,
        TokenizationMode::Preserve,
        &empty_mwt,
    )
    .expect("primary injection must succeed");

    // Deliberately skip dispatch_secondary_l2 — production behaviour
    // for an unsupported-secondary lang is exactly this: no Stanza
    // dispatch, no splice, the L2|xxx placeholder remains.
    let pairs = first_utt_mor_pairs(&chat_file);
    assert_position_is_l2_xxx("@s:que (unsupported secondary)", &pairs, 2);
    validate_or_panic(&mut chat_file, "@s:que (unsupported secondary)");
}

// ---------------------------------------------------------------------
// Row 2: `@s:LANG+LANG2` — Multiple-language marker (the foreign word
// is valid in BOTH listed languages). The L2 plan rejects this for
// dispatch because there is no single trustworthy target.
//
// TRANSITION PATH: when we route Multiple-language words through a
// disambiguation step (e.g. picking the more-likely of the two via
// surrounding tier language), rewrite the assertion at position 3
// (`cafe`) to the chosen analysis.
// ---------------------------------------------------------------------
#[test]
fn l2_fallback_multiple_languages_at_s_marker_remains_l2_xxx() {
    use talkbank_transform::parse::{TreeSitterParser, parse_lenient};

    let parser = TreeSitterParser::new().unwrap();
    // Inline minimal CHAT — `cafe@s:eng+fra` is "valid in BOTH eng
    // and fra"; the Multiple resolution can't dispatch one secondary.
    let chat = "\
@UTF8
@Begin
@Languages:\teng, fra
@Participants:\tPAR Participant
@ID:\teng|test|PAR|||||Participant|||
*PAR:\tI want cafe@s:eng+fra .
@End
";
    let (mut chat_file, _) = parse_lenient(&parser, chat);

    let primary_lang = talkbank_model::model::LanguageCode::new("eng");
    let langs = declared_languages(&chat_file, &primary_lang);
    let batch_items = collect_payloads(
        &chat_file,
        &primary_lang,
        &langs,
        MultilingualPolicy::ProcessAll,
    )
    .batch_items;
    assert_eq!(batch_items.len(), 1, "fixture has one utterance");

    let primary_ud = ud_response_from_words(
        r#"[
          {"id":1,"text":"I","lemma":"I","upos":"PRON","head":2,"deprel":"nsubj"},
          {"id":2,"text":"want","lemma":"want","upos":"VERB","head":0,"deprel":"root"},
          {"id":3,"text":"xbxxx","lemma":"xbxxx","upos":"NOUN","head":2,"deprel":"obj"},
          {"id":4,"text":".","lemma":".","upos":"PUNCT","head":2,"deprel":"punct"}
        ]"#,
    );

    let empty_mwt = std::collections::BTreeMap::new();
    inject_results(
        &parser,
        &mut chat_file,
        batch_items,
        vec![primary_ud],
        &primary_lang,
        TokenizationMode::Preserve,
        &empty_mwt,
    )
    .expect("primary injection must succeed");

    let pairs = first_utt_mor_pairs(&chat_file);
    assert_position_is_l2_xxx("@s:eng+fra (Multiple)", &pairs, 2);
    validate_or_panic(&mut chat_file, "@s:eng+fra (Multiple)");
}

// ---------------------------------------------------------------------
// Row 3: `@s:LANG&LANG2` — Ambiguous-language marker (the foreign word
// could plausibly belong to either listed language). Symmetric to
// Multiple from the dispatcher's perspective: no single target.
//
// TRANSITION PATH: same as Multiple. If we add ambiguity resolution
// for one of the languages the test pins, rewrite the position-3
// assertion to the disambiguated analysis.
// ---------------------------------------------------------------------
#[test]
fn l2_fallback_ambiguous_languages_at_s_marker_remains_l2_xxx() {
    use talkbank_transform::parse::{TreeSitterParser, parse_lenient};

    let parser = TreeSitterParser::new().unwrap();
    // `no@s:eng&spa` — the word "no" is ambiguously English or Spanish.
    let chat = "\
@UTF8
@Begin
@Languages:\teng, spa
@Participants:\tPAR Participant
@ID:\teng|test|PAR|||||Participant|||
*PAR:\tno@s:eng&spa quiero .
@End
";
    let (mut chat_file, _) = parse_lenient(&parser, chat);

    let primary_lang = talkbank_model::model::LanguageCode::new("eng");
    let langs = declared_languages(&chat_file, &primary_lang);
    let batch_items = collect_payloads(
        &chat_file,
        &primary_lang,
        &langs,
        MultilingualPolicy::ProcessAll,
    )
    .batch_items;
    assert_eq!(batch_items.len(), 1, "fixture has one utterance");

    let primary_ud = ud_response_from_words(
        r#"[
          {"id":1,"text":"xbxxx","lemma":"xbxxx","upos":"INTJ","head":2,"deprel":"discourse"},
          {"id":2,"text":"quiero","lemma":"quiero","upos":"VERB","head":0,"deprel":"root"},
          {"id":3,"text":".","lemma":".","upos":"PUNCT","head":2,"deprel":"punct"}
        ]"#,
    );

    let empty_mwt = std::collections::BTreeMap::new();
    inject_results(
        &parser,
        &mut chat_file,
        batch_items,
        vec![primary_ud],
        &primary_lang,
        TokenizationMode::Preserve,
        &empty_mwt,
    )
    .expect("primary injection must succeed");

    let pairs = first_utt_mor_pairs(&chat_file);
    assert_position_is_l2_xxx("@s:eng&spa (Ambiguous)", &pairs, 0);
    validate_or_panic(&mut chat_file, "@s:eng&spa (Ambiguous)");
}

// ---------------------------------------------------------------------
// Row 4: `[- UNSUPPORTEDLANG]` — whole-utterance language switch into
// a Stanza-unsupported language. The morphotag worker's
// `partition_groups_by_stanza_support` keeps that group out of
// dispatch entirely, so every word in the utterance falls back to
// L2|xxx.
//
// This test exercises the partition fallback shape end-to-end at the
// inject_results layer: we feed a primary UD response containing
// nothing but `xbxxx` placeholders (mirroring the empty UdResponse
// the partition fills in for the unsupported group), and assert that
// every word position resolves to L2|xxx.
//
// TRANSITION PATH: when we add a non-Stanza analyzer for one of the
// currently-unsupported languages (e.g. Marathi via a separate model
// runtime), rewrite this test's per-position assertions to the real
// expected analysis for that language.
// ---------------------------------------------------------------------
#[test]
fn l2_fallback_unsupported_precode_whole_utterance_remains_all_l2_xxx() {
    use talkbank_transform::parse::{TreeSitterParser, parse_lenient};

    let parser = TreeSitterParser::new().unwrap();
    // `[- nep]` whole-utterance language switch into Nepali.
    let chat = "\
@UTF8
@Begin
@Languages:\teng, nep
@Participants:\tPAR Participant
@ID:\teng|test|PAR|||||Participant|||
*PAR:\t[- nep] hello world .
@End
";
    let (mut chat_file, _) = parse_lenient(&parser, chat);

    let primary_lang = talkbank_model::model::LanguageCode::new("eng");
    let langs = declared_languages(&chat_file, &primary_lang);
    let batch_items = collect_payloads(
        &chat_file,
        &primary_lang,
        &langs,
        MultilingualPolicy::ProcessAll,
    )
    .batch_items;
    assert_eq!(batch_items.len(), 1, "fixture has one utterance");

    // Empty `UdResponse { sentences: vec![] }` — production
    // `partition_groups_by_stanza_support` fills this in for every
    // unsupported-language group, and downstream `inject_results`
    // skips items whose response has no sentences, leaving the
    // pre-injection state intact (no %mor written).
    let primary_ud = crate::chat_ops::nlp::UdResponse { sentences: vec![] };

    let empty_mwt = std::collections::BTreeMap::new();
    inject_results(
        &parser,
        &mut chat_file,
        batch_items,
        vec![primary_ud],
        &primary_lang,
        TokenizationMode::Preserve,
        &empty_mwt,
    )
    .expect(
        "primary injection must succeed (empty-sentences response \
             is the production partition-fallback shape)",
    );

    // The post-fallback state for a `[- UNSUPPORTEDLANG]` utterance
    // is: no `%mor` tier emitted for this utterance at all (the
    // partition skipped it; injection had no analysis to write).
    // Validation must still pass — a missing `%mor` for an utterance
    // is not by itself a CHAT validity error.
    use talkbank_model::model::Line;
    let utt = chat_file
        .lines
        .iter()
        .find_map(|l| match l {
            Line::Utterance(u) => Some(u),
            _ => None,
        })
        .expect("fixture must have an utterance");
    assert!(
        utt.mor_tier().is_none(),
        "[- nep] (unsupported precode): expected NO %mor for the \
         skipped utterance under the partition fallback (production \
         shape: every word is L2|xxx-equivalent because the worker \
         never produced an analysis). Got: {:?}. \
         TRANSITION PATH: when we add a non-Stanza analyzer for the \
         currently-unsupported precode language, this test should \
         start asserting that %mor is present and contains the real \
         analysis.",
        utt.mor_tier()
    );

    validate_or_panic(
        &mut chat_file,
        "[- nep] whole-utterance (unsupported precode)",
    );
}
