//! Splice merged L2 morphology back into a ChatFile.
//!
//! Overwrites `L2|xxx` MOR items with pre-mapped `Mor` items from the
//! structural merge algorithm, and optionally corrects GRA deprels.

use super::extract::L2DeferredPosition;
use super::merge::MergedL2Morphology;

/// Outcome of splicing L2 results into a `ChatFile`.
#[derive(Debug, Default)]
pub struct SpliceOutcome {
    /// Number of @s positions successfully spliced with real morphology.
    pub spliced: usize,
    /// Number of @s positions that fell back to L2|xxx (no secondary result).
    pub fallback: usize,
    /// Number of GRA deprels corrected.
    pub gra_upgraded: usize,
}

/// Overwrite `L2|xxx` MOR items with merged morphology.
///
/// Each `MergedL2Morphology` contains a fully-mapped `Mor` item (produced
/// by `map_ud_sentence` which handles MWT contractions, then POS-overridden
/// by the merge algorithm). The splice is a simple assignment.
///
/// This function must be called AFTER `inject_results` has set L2|xxx on
/// all @s positions.
pub fn splice_l2_into_chat(
    chat_file: &mut talkbank_model::model::ChatFile,
    deferred: &[L2DeferredPosition],
    merged_results: &[Option<MergedL2Morphology>],
) -> SpliceOutcome {
    use talkbank_model::model::DependentTier;
    use talkbank_model::model::Line;

    let mut outcome = SpliceOutcome::default();

    for (def, merged_opt) in deferred.iter().zip(merged_results.iter()) {
        let merged = match merged_opt {
            Some(m) => m,
            None => {
                outcome.fallback += 1;
                continue;
            }
        };

        let utt = match &mut chat_file.lines[def.line_idx] {
            Line::Utterance(u) => u,
            _ => {
                outcome.fallback += 1;
                continue;
            }
        };

        let mut mor_tier = None;
        let mut gra_tier = None;
        for tier in &mut utt.dependent_tiers {
            match tier {
                DependentTier::Mor(m) => mor_tier = Some(m),
                DependentTier::Gra(g) => gra_tier = Some(g),
                _ => {}
            }
        }

        if let Some(mor) = mor_tier {
            if let Some(gra) = gra_tier {
                // Coordinated mutation handles both MOR and GRA.
                if mor
                    .splice_coordinated(
                        gra,
                        def.word_idx,
                        merged.mor.clone(),
                        merged.gras.clone(),
                        merged.external_anchor,
                    )
                    .is_ok()
                {
                    outcome.spliced += 1;
                    if merged.corrected_deprel.is_some() {
                        outcome.gra_upgraded += 1;
                    }
                } else {
                    outcome.fallback += 1;
                }
            } else if let Some(mor_item) = mor.items_mut().get_mut(def.word_idx) {
                // No GRA tier, just update MOR.
                *mor_item = merged.mor.clone();
                outcome.spliced += 1;
            } else {
                outcome.fallback += 1;
            }
        } else {
            outcome.fallback += 1;
        }
    }

    outcome
}

#[cfg(test)]
mod cardinality_tests {
    use super::*;
    use crate::morphosyntax::l2::extract::{L2DeferredPosition, PrimaryStructuralInfo};
    use crate::morphosyntax::l2::merge::MergedL2Morphology;
    use crate::parse::parse_lenient;
    use talkbank_model::ParseValidateOptions;
    use talkbank_model::model::LanguageCode;
    use talkbank_model::model::dependent_tier::GrammaticalRelation;
    use talkbank_model::model::dependent_tier::mor::{Mor, MorStem, MorWord, PosCategory};
    use talkbank_parser::TreeSitterParser;

    /// Splice replaces a 1-chunk `L2|xxx` slot with an N-chunk merged Mor.
    /// The output `%mor` chunk count grows but `%gra` count stays the same;
    /// the resulting ChatFile must still validate. Currently fails because
    /// the splice does not adjust GRA cardinality.
    #[test]
    fn multi_chunk_merged_mor_keeps_chat_valid() {
        let chat_text = "@UTF8\n\
                         @Begin\n\
                         @Languages:\tfra, ara\n\
                         @Participants:\tPAR Participant\n\
                         @ID:\tfra|test|PAR|||||Participant|||\n\
                         *PAR:\tyellow@s .\n\
                         %mor:\tL2|xxx .\n\
                         %gra:\t1|0|ROOT 2|1|PUNCT\n\
                         @End\n";
        let parser = TreeSitterParser::new().unwrap();
        let (mut chat_file, _errors) = parse_lenient(&parser, chat_text);

        let mut precondition = chat_file.clone();
        let opts = ParseValidateOptions::default().with_alignment();
        assert!(
            talkbank_model::validate_chat_file_with_options(&mut precondition, &opts).is_ok(),
            "fixture precondition: input must validate before splice",
        );

        let merged_mor = Mor::new(MorWord::new(PosCategory::new("verb"), MorStem::new("yel")))
            .with_post_clitic(MorWord::new(PosCategory::new("part"), MorStem::new("lo")));

        let mut utt_idx = None;
        for (i, line) in chat_file.lines.iter().enumerate() {
            if let talkbank_model::model::Line::Utterance(_) = line {
                utt_idx = Some(i);
                break;
            }
        }
        let line_idx = utt_idx.expect("utterance present");

        let deferred = vec![L2DeferredPosition {
            line_idx,
            word_idx: 0,
            target_lang: LanguageCode::new("ara"),
            primary: PrimaryStructuralInfo {
                deprel: crate::morphosyntax::l2::deprel::UdDeprel::new("root"),
                upos: None,
                head: 0,
                dependent_deprels: Vec::new(),
                head_upos: None,
            },
        }];
        let merged = vec![Some(MergedL2Morphology {
            mor: merged_mor,
            gras: vec![
                GrammaticalRelation::new(1, 0, "ROOT"),
                GrammaticalRelation::new(2, 1, "DEP"),
            ],
            corrected_deprel: None,
            external_anchor: None,
        })];

        let outcome = splice_l2_into_chat(&mut chat_file, &deferred, &merged);
        assert_eq!(
            outcome.spliced, 1,
            "splice must report success for the slot"
        );
        assert_eq!(outcome.fallback, 0);

        validate_morphosyntax(&mut chat_file);
    }

    fn validate_morphosyntax(chat: &mut talkbank_model::model::ChatFile) {
        use talkbank_model::ParseValidateOptions;
        let opts = ParseValidateOptions::default().with_alignment();
        if let Err(e) = talkbank_model::validate_chat_file_with_options(chat, &opts) {
            panic!("Morphosyntax validation failed: {:#?}", e);
        }
    }
}

/// Apply L2|xxx fallback to deferred positions that have no merged result.
pub fn apply_l2_fallback(
    chat_file: &mut talkbank_model::model::ChatFile,
    deferred: &[L2DeferredPosition],
) {
    use talkbank_model::model::DependentTier;
    use talkbank_model::model::Line;
    use talkbank_model::model::dependent_tier::mor::{MorStem, PosCategory};

    for def in deferred {
        let utt = match &mut chat_file.lines[def.line_idx] {
            Line::Utterance(u) => u,
            _ => continue,
        };
        let mor_tier = utt.dependent_tiers.iter_mut().find_map(|t| match t {
            DependentTier::Mor(m) => Some(m),
            _ => None,
        });
        if let Some(mor) = mor_tier
            && let Some(mor_item) = mor.items_mut().get_mut(def.word_idx)
        {
            mor_item.main.pos = PosCategory::new("L2");
            mor_item.main.lemma = MorStem::new("xxx");
            mor_item.main.features.clear();
        }
    }
}
