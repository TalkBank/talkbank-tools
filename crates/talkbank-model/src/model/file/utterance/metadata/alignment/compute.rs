use super::count_based::{
    build_mor_tier_from_items, build_phonology_alignment_from_counts,
    build_sin_alignment_from_counts, build_tier_to_tier_alignment,
};
use super::diagnostics::{first_non_dummy_span, skipped_alignment_warning};
use crate::model::{AlignmentSet, AlignmentUnits, ParseHealthTier};
use crate::validation::ValidationContext;
use crate::{ErrorCode, Span, Utterance};

impl Utterance {
    /// Recompute all derived alignment metadata for this utterance.
    pub fn compute_alignments(&mut self, context: &ValidationContext) {
        self.alignment_diagnostics.clear();

        let units = AlignmentUnits::from_utterance(self, context);
        let mut metadata = AlignmentSet::new(units);
        let health = self.parse_health.unwrap_or_default();

        let (mor_items, mor_span) = if let Some(tier) = self.mor_tier() {
            (Some(tier.items.0.clone()), tier.span)
        } else {
            (None, Span::DUMMY)
        };
        let (gra_relations, gra_span) = if let Some(tier) = self.gra_tier() {
            (Some(tier.relations.0.clone()), tier.span)
        } else {
            (None, Span::DUMMY)
        };
        let wor_span = self.wor_tier().map_or(Span::DUMMY, |t| t.span);
        let pho_span = self.pho_tier().map_or(Span::DUMMY, |t| t.span);
        let mod_span = self.mod_tier().map_or(Span::DUMMY, |t| t.span);
        let sin_span = self.sin_tier().map_or(Span::DUMMY, |t| t.span);

        if let Some(items) = mor_items.as_ref() {
            let mor = build_mor_tier_from_items(self, items);
            if health.can_align_main_to_mor() {
                metadata.mor = Some(crate::alignment::align_main_to_mor(&self.main, &mor));
            } else {
                metadata.mor = Some(crate::alignment::MorAlignment::new().with_error(
                    skipped_alignment_warning(
                        "main↔%mor",
                        "main tier",
                        health.is_tier_clean(ParseHealthTier::Main),
                        self.main.span,
                        "%mor tier",
                        health.is_tier_clean(ParseHealthTier::Mor),
                        mor_span,
                    ),
                ));
            }
        }

        if let (Some(items), Some(relations)) = (mor_items.as_ref(), gra_relations.as_ref()) {
            if health.can_align_mor_to_gra() {
                let mor = build_mor_tier_from_items(self, items);
                let gra = crate::model::GraTier::new_gra(relations.clone()).with_span(gra_span);
                metadata.gra = Some(crate::alignment::align_mor_to_gra(&mor, &gra));
            } else {
                metadata.gra = Some(crate::alignment::GraAlignment::new().with_error(
                    skipped_alignment_warning(
                        "%mor↔%gra",
                        "%mor tier",
                        health.is_tier_clean(ParseHealthTier::Mor),
                        mor_span,
                        "%gra tier",
                        health.is_tier_clean(ParseHealthTier::Gra),
                        gra_span,
                    ),
                ));
            }
        }

        if let Some(tier) = self.pho_tier() {
            let item_count = tier.items.0.len();
            if health.can_align_main_to_pho() {
                metadata.pho = Some(build_phonology_alignment_from_counts(
                    &self.main, item_count, pho_span, "%pho",
                ));
            } else {
                metadata.pho = Some(crate::alignment::PhoAlignment::new().with_error(
                    skipped_alignment_warning(
                        "main↔%pho",
                        "main tier",
                        health.is_tier_clean(ParseHealthTier::Main),
                        self.main.span,
                        "%pho tier",
                        health.is_tier_clean(ParseHealthTier::Pho),
                        pho_span,
                    ),
                ));
            }
        }

        if let Some(wor) = self.wor_tier().cloned() {
            if health.can_align_main_to_wor() {
                metadata.wor = Some(crate::alignment::align_main_to_wor(&self.main, &wor));
            } else {
                metadata.wor = Some(crate::alignment::WorAlignment::new().with_error(
                    skipped_alignment_warning(
                        "main↔%wor",
                        "main tier",
                        health.is_tier_clean(ParseHealthTier::Main),
                        self.main.span,
                        "%wor tier",
                        health.is_tier_clean(ParseHealthTier::Wor),
                        wor_span,
                    ),
                ));
            }
        }

        if let Some(tier) = self.mod_tier() {
            let item_count = tier.items.0.len();
            if health.can_align_main_to_mod() {
                metadata.mod_ = Some(build_phonology_alignment_from_counts(
                    &self.main, item_count, mod_span, "%mod",
                ));
            } else {
                metadata.mod_ = Some(crate::alignment::PhoAlignment::new().with_error(
                    skipped_alignment_warning(
                        "main↔%mod",
                        "main tier",
                        health.is_tier_clean(ParseHealthTier::Main),
                        self.main.span,
                        "%mod tier",
                        health.is_tier_clean(ParseHealthTier::Mod),
                        mod_span,
                    ),
                ));
            }
        }

        if let Some(tier) = self.sin_tier() {
            let item_count = tier.items.0.len();
            if health.can_align_main_to_sin() {
                metadata.sin = Some(build_sin_alignment_from_counts(
                    &self.main, item_count, sin_span,
                ));
            } else {
                metadata.sin = Some(crate::alignment::SinAlignment::new().with_error(
                    skipped_alignment_warning(
                        "main↔%sin",
                        "main tier",
                        health.is_tier_clean(ParseHealthTier::Main),
                        self.main.span,
                        "%sin tier",
                        health.is_tier_clean(ParseHealthTier::Sin),
                        sin_span,
                    ),
                ));
            }
        }

        let modsyl_span = self.modsyl_tier().map_or(Span::DUMMY, |t| t.span);
        let phosyl_span = self.phosyl_tier().map_or(Span::DUMMY, |t| t.span);
        let phoaln_span = self.phoaln_tier().map_or(Span::DUMMY, |t| t.span);

        if let (Some(modsyl), Some(mod_tier)) = (self.modsyl_tier(), self.mod_tier()) {
            if health.can_align_modsyl_to_mod() {
                metadata.modsyl = Some(build_tier_to_tier_alignment(
                    modsyl.word_count(),
                    modsyl_span,
                    "%modsyl",
                    mod_tier.items.0.len(),
                    mod_span,
                    "%mod",
                    ErrorCode::ModsylModCountMismatch,
                ));
            } else {
                metadata.modsyl = Some(crate::alignment::PhoAlignment::new().with_error(
                    skipped_alignment_warning(
                        "%modsyl↔%mod",
                        "%modsyl tier",
                        health.is_tier_clean(ParseHealthTier::Modsyl),
                        modsyl_span,
                        "%mod tier",
                        health.is_tier_clean(ParseHealthTier::Mod),
                        mod_span,
                    ),
                ));
            }
        }

        if let (Some(phosyl), Some(pho_tier)) = (self.phosyl_tier(), self.pho_tier()) {
            if health.can_align_phosyl_to_pho() {
                metadata.phosyl = Some(build_tier_to_tier_alignment(
                    phosyl.word_count(),
                    phosyl_span,
                    "%phosyl",
                    pho_tier.items.0.len(),
                    pho_span,
                    "%pho",
                    ErrorCode::PhosylPhoCountMismatch,
                ));
            } else {
                metadata.phosyl = Some(crate::alignment::PhoAlignment::new().with_error(
                    skipped_alignment_warning(
                        "%phosyl↔%pho",
                        "%phosyl tier",
                        health.is_tier_clean(ParseHealthTier::Phosyl),
                        phosyl_span,
                        "%pho tier",
                        health.is_tier_clean(ParseHealthTier::Pho),
                        pho_span,
                    ),
                ));
            }
        }

        if let Some(phoaln) = self.phoaln_tier() {
            let phoaln_wc = phoaln.word_count();
            if health.can_align_phoaln() {
                let mut alignment = crate::alignment::PhoAlignment::new();
                let mod_count = self.mod_tier().map(|t| t.items.0.len());
                let pho_count = self.pho_tier().map(|t| t.items.0.len());

                if let Some(mc) = mod_count
                    && phoaln_wc != mc
                {
                    alignment =
                        alignment.with_error(super::diagnostics::build_count_mismatch_error(
                            phoaln_wc,
                            phoaln_span,
                            "%phoaln",
                            mc,
                            mod_span,
                            "%mod",
                            ErrorCode::PhoalnModCountMismatch,
                        ));
                }
                if let Some(pc) = pho_count
                    && phoaln_wc != pc
                {
                    alignment =
                        alignment.with_error(super::diagnostics::build_count_mismatch_error(
                            phoaln_wc,
                            phoaln_span,
                            "%phoaln",
                            pc,
                            pho_span,
                            "%pho",
                            ErrorCode::PhoalnPhoCountMismatch,
                        ));
                }

                let effective_count = mod_count
                    .unwrap_or(phoaln_wc)
                    .min(pho_count.unwrap_or(phoaln_wc))
                    .min(phoaln_wc);
                for i in 0..effective_count {
                    alignment =
                        alignment.with_pair(crate::alignment::AlignmentPair::new(Some(i), Some(i)));
                }

                metadata.phoaln = Some(alignment);
            } else {
                metadata.phoaln = Some(crate::alignment::PhoAlignment::new().with_error(
                    skipped_alignment_warning(
                        "%phoaln↔%mod/%pho",
                        "%phoaln tier",
                        health.is_tier_clean(ParseHealthTier::Phoaln),
                        phoaln_span,
                        "%mod/%pho tiers",
                        health.is_tier_clean(ParseHealthTier::Mod)
                            && health.is_tier_clean(ParseHealthTier::Pho),
                        first_non_dummy_span([mod_span, pho_span]),
                    ),
                ));
            }
        }

        self.alignment_diagnostics = metadata.collect_errors().into_iter().cloned().collect();
        self.alignments = Some(metadata);
    }

    /// Recompute alignments using a default validation context.
    pub fn compute_alignments_default(&mut self) {
        self.compute_alignments(&ValidationContext::default());
    }

    /// Return `true` when no alignment diagnostics are currently recorded.
    pub fn alignments_valid(&self) -> bool {
        self.alignment_diagnostics.is_empty()
    }

    /// Return borrowed alignment diagnostics currently attached to the utterance.
    pub fn collect_alignment_errors(&self) -> Vec<&crate::ParseError> {
        self.alignment_diagnostics.iter().collect()
    }
}
