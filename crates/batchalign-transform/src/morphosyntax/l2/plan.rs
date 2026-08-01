//! Transform-layer planning for secondary L2 dispatch.
//!
//! `batchalign` should only obtain primary/secondary UD analyses. The stable
//! CHAT-specific planning seam lives here: contiguous span grouping,
//! provenance-preserving word extraction, and host attachment planning.

use talkbank_model::model::LanguageCode;

use super::deprel::UdDeprel;
use super::extract::L2DeferredPosition;

/// How the secondary span's ROOT should anchor back into the host utterance.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum L2RootAnchor {
    /// Attach the secondary ROOT under the current governing host chunk for the
    /// chosen deferred placeholder position.
    HostGovernor {
        /// Global index into the `L2DeferredPosition` array for the deferred word
        /// that supplied the host attachment.
        source_deferred_index: usize,
    },
    /// Preserve utterance-root anchoring for the chosen deferred placeholder
    /// position.
    UtteranceRoot {
        /// Global index into the `L2DeferredPosition` array for the deferred word
        /// that supplied the root attachment.
        source_deferred_index: usize,
    },
}

/// Host-utterance attachment metadata for one secondary-dispatch span.
///
/// This stays explicit until final lowering so secondary sentence roots can be
/// re-anchored back into the host utterance without carrying raw numeric
/// indices across word-domain/chunk-domain boundaries.
#[derive(Debug, Clone, PartialEq, Default)]
pub enum L2Attachment {
    /// The secondary span keeps its own internal ROOT; no host-side anchor.
    #[default]
    InternalRoot,
    /// The secondary ROOT should reattach to the host utterance, using the
    /// supplied host-side deprel once splice resolves the actual chunk anchor
    /// from the placeholder `%gra` relation.
    ExternalRoot {
        /// Host-side deprel to use when rewriting the secondary ROOT away from
        /// `ROOT` after attaching it back to the host utterance.
        host_deprel: UdDeprel,
        /// Provenance for how the secondary ROOT should anchor back into the host.
        root_anchor: L2RootAnchor,
    },
}

impl L2Attachment {
    /// Returns the host-side deprel used when an external root attachment is
    /// present.
    pub fn external_root_deprel(&self) -> Option<&UdDeprel> {
        match self {
            Self::InternalRoot => None,
            Self::ExternalRoot { host_deprel, .. } => Some(host_deprel),
        }
    }

    /// Returns whether this span's secondary root attaches back to the host.
    pub fn is_external_root(&self) -> bool {
        matches!(self, Self::ExternalRoot { .. })
    }

    /// Returns the deferred position that supplied the root attachment.
    pub fn source_deferred_index(&self) -> Option<usize> {
        match self {
            Self::InternalRoot => None,
            Self::ExternalRoot {
                root_anchor:
                    L2RootAnchor::HostGovernor {
                        source_deferred_index,
                    }
                    | L2RootAnchor::UtteranceRoot {
                        source_deferred_index,
                    },
                ..
            } => Some(*source_deferred_index),
        }
    }

    /// Returns whether the external root should attach to a host governor.
    pub fn uses_host_governor_anchor(&self) -> bool {
        matches!(
            self,
            Self::ExternalRoot {
                root_anchor: L2RootAnchor::HostGovernor { .. },
                ..
            }
        )
    }

    /// Returns whether the external root should preserve utterance-root anchoring.
    pub fn uses_utterance_root_anchor(&self) -> bool {
        matches!(
            self,
            Self::ExternalRoot {
                root_anchor: L2RootAnchor::UtteranceRoot { .. },
                ..
            }
        )
    }

    /// Return this attachment with a replacement host-side deprel, preserving
    /// its anchor provenance.
    pub fn with_host_deprel(&self, host_deprel: UdDeprel) -> Self {
        match self {
            Self::InternalRoot => Self::InternalRoot,
            Self::ExternalRoot { root_anchor, .. } => Self::ExternalRoot {
                host_deprel,
                root_anchor: root_anchor.clone(),
            },
        }
    }
}

/// One contiguous secondary-dispatch span plus its planned host attachment.
#[derive(Debug, Clone, PartialEq)]
pub struct L2SpanPlan {
    /// Global indices into the `L2DeferredPosition` array.
    pub deferred_indices: Vec<usize>,
    /// Owning line in the `ChatFile`.
    pub line_idx: usize,
    /// Target language to dispatch this span to.
    pub target_lang: LanguageCode,
    /// Provenance-sealed word texts to send to the secondary model.
    pub words: Vec<talkbank_model::ChatCleanedText>,
    /// Explicit host-attachment plan for the span's secondary root.
    pub attachment: L2Attachment,
    /// Terminator of the utterance this span came from.
    ///
    /// An INPUT to the secondary Stanza model, which changes its analysis
    /// when sentence-final punctuation is absent or wrong.
    pub terminator: talkbank_model::Terminator,
}

/// Full secondary-dispatch plan for one utterance batch.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct L2DispatchPlan {
    /// Planned spans in transcript order.
    pub spans: Vec<L2SpanPlan>,
}

fn planned_attachment(deferred: &[L2DeferredPosition], deferred_indices: &[usize]) -> L2Attachment {
    let mut utterance_root_candidate: Option<(usize, UdDeprel)> = None;

    for &global_idx in deferred_indices {
        let Some(current) = deferred.get(global_idx) else {
            tracing::warn!(
                global_idx,
                "L2 dispatch plan: deferred index out of range while computing attachment"
            );
            continue;
        };

        let primary_head = current.primary.head;
        let head_in_span = primary_head > 0
            && deferred_indices.iter().any(|&candidate_idx| {
                deferred
                    .get(candidate_idx)
                    .is_some_and(|candidate| candidate.word_idx + 1 == primary_head)
            });

        if !head_in_span {
            if primary_head > 0 {
                return L2Attachment::ExternalRoot {
                    host_deprel: current.primary.deprel.clone(),
                    root_anchor: L2RootAnchor::HostGovernor {
                        source_deferred_index: global_idx,
                    },
                };
            }
            if primary_head == 0 && utterance_root_candidate.is_none() {
                // The L2 word is the host primary's utterance root.
                // The host-side deprel for an UtteranceRoot anchor MUST
                // be "root" so the splice's eventual head=0 promotion
                // pairs with deprel=ROOT (joint invariant `(head == 0)
                // ⟺ (deprel == "ROOT")`). Copying `current.primary.deprel`
                // verbatim was the bug behind ~308 wild E722 occurrences
                // (head=0 with deprel ∈ {DET, NMOD, …}) on 2026-05-06.
                utterance_root_candidate = Some((global_idx, UdDeprel::new("root")));
            }
        }
    }

    if let Some((source_deferred_index, host_deprel)) = utterance_root_candidate {
        return L2Attachment::ExternalRoot {
            host_deprel,
            root_anchor: L2RootAnchor::UtteranceRoot {
                source_deferred_index,
            },
        };
    }

    L2Attachment::InternalRoot
}

fn push_planned_span(
    spans: &mut Vec<L2SpanPlan>,
    deferred: &[L2DeferredPosition],
    line_idx: usize,
    target_lang: LanguageCode,
    deferred_indices: Vec<usize>,
    words: Vec<talkbank_model::ChatCleanedText>,
    terminator: talkbank_model::Terminator,
) {
    if deferred_indices.is_empty() {
        return;
    }

    spans.push(L2SpanPlan {
        attachment: planned_attachment(deferred, &deferred_indices),
        deferred_indices,
        line_idx,
        target_lang,
        words,
        terminator,
    });
}

/// Plan contiguous secondary-dispatch spans from deferred positions.
///
/// Everything a span needs travels on the positions themselves, so there is
/// no second derivation to keep in step and no lookup that can miss.
pub fn plan_dispatch_spans(deferred: &[L2DeferredPosition]) -> L2DispatchPlan {
    let mut spans = Vec::new();
    let mut current_indices = Vec::new();
    let mut current_words = Vec::new();
    let mut current_line: Option<(usize, talkbank_model::Terminator)> = None;
    let mut current_lang: Option<LanguageCode> = None;
    let mut previous_word_idx = None;

    for (global_idx, def) in deferred.iter().enumerate() {
        let extends = current_line.as_ref().map(|(idx, _)| *idx) == Some(def.line_idx)
            && current_lang.as_ref() == Some(&def.target_lang)
            && previous_word_idx.is_some_and(|prev| prev + 1 == def.word_idx);

        if !extends {
            if let (Some((line_idx, line_terminator)), Some(target_lang)) =
                (current_line.take(), current_lang.take())
            {
                push_planned_span(
                    &mut spans,
                    deferred,
                    line_idx,
                    target_lang,
                    std::mem::take(&mut current_indices),
                    std::mem::take(&mut current_words),
                    line_terminator,
                );
            }
            current_line = Some((def.line_idx, def.terminator.clone()));
            current_lang = Some(def.target_lang.clone());
        }

        current_indices.push(global_idx);
        current_words.push(def.word.clone());
        previous_word_idx = Some(def.word_idx);
    }

    if let (Some((line_idx, line_terminator)), Some(target_lang)) = (current_line, current_lang) {
        push_planned_span(
            &mut spans,
            deferred,
            line_idx,
            target_lang,
            current_indices,
            current_words,
            line_terminator,
        );
    }

    L2DispatchPlan { spans }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::morphosyntax::UniversalPos;
    use crate::morphosyntax::l2::PrimaryStructuralInfo;

    fn make_deferred(
        line_idx: usize,
        word_idx: usize,
        lang: &str,
        deprel: &str,
        head: usize,
        word: &str,
    ) -> L2DeferredPosition {
        L2DeferredPosition {
            line_idx,
            word_idx,
            target_lang: LanguageCode::new(lang).expect("valid test language code"),
            word: talkbank_model::ChatCleanedText::test_unchecked(word),
            terminator: talkbank_model::Terminator::Period {
                span: talkbank_model::Span::DUMMY,
            },
            primary: PrimaryStructuralInfo {
                deprel: UdDeprel::new(deprel),
                upos: Some(UniversalPos::Noun),
                head,
                dependent_deprels: Vec::new(),
                head_upos: Some(UniversalPos::Verb),
            },
        }
    }

    #[test]
    fn plan_dispatch_spans_tracks_external_attachment_for_contiguous_span() {
        let deferred = vec![
            make_deferred(5, 1, "spa", "obj", 1, "los"),
            make_deferred(5, 2, "spa", "obl", 1, "ninos"),
        ];

        let plan = plan_dispatch_spans(&deferred);
        assert_eq!(plan.spans.len(), 1);
        assert_eq!(plan.spans[0].deferred_indices, vec![0, 1]);
        assert_eq!(plan.spans[0].line_idx, 5);
        assert_eq!(plan.spans[0].words, vec!["los", "ninos"]);
        assert_eq!(
            plan.spans[0]
                .attachment
                .external_root_deprel()
                .map(UdDeprel::as_str),
            Some("obj")
        );
        assert!(plan.spans[0].attachment.is_external_root());
        assert!(plan.spans[0].attachment.uses_host_governor_anchor());
        assert_eq!(plan.spans[0].attachment.source_deferred_index(), Some(0));
    }

    #[test]
    fn plan_dispatch_spans_separates_noncontiguous_same_language_words() {
        let deferred = vec![
            make_deferred(5, 1, "spa", "obj", 1, "uno"),
            make_deferred(5, 3, "spa", "obl", 1, "dos"),
        ];

        let plan = plan_dispatch_spans(&deferred);
        assert_eq!(plan.spans.len(), 2);
        assert_eq!(plan.spans[0].deferred_indices, vec![0]);
        assert_eq!(plan.spans[1].deferred_indices, vec![1]);
    }

    #[test]
    fn plan_dispatch_spans_prefers_real_host_attachment_over_earlier_primary_root_noise() {
        let deferred = vec![
            make_deferred(5, 1, "spa", "root", 0, "uno"),
            make_deferred(5, 2, "spa", "obj", 6, "dos"),
        ];

        let plan = plan_dispatch_spans(&deferred);
        assert_eq!(plan.spans.len(), 1);
        assert_eq!(plan.spans[0].deferred_indices, vec![0, 1]);
        assert_eq!(
            plan.spans[0]
                .attachment
                .external_root_deprel()
                .map(UdDeprel::as_str),
            Some("obj"),
            "when one contiguous @s span contains an earlier primary-root token \
             and a later token with a real host-governed attachment, the plan \
             must prefer the host-governed attachment source instead of \
             collapsing the span to the earlier `root` deprel"
        );
        assert!(plan.spans[0].attachment.uses_host_governor_anchor());
        assert_eq!(plan.spans[0].attachment.source_deferred_index(), Some(1));
    }

    #[test]
    fn plan_dispatch_spans_preserves_utterance_root_when_no_host_governor_exists() {
        let deferred = vec![
            make_deferred(5, 1, "spa", "dep", 2, "uno"),
            make_deferred(5, 2, "spa", "root", 0, "dos"),
        ];

        let plan = plan_dispatch_spans(&deferred);
        assert_eq!(plan.spans.len(), 1);
        assert_eq!(plan.spans[0].deferred_indices, vec![0, 1]);
        assert!(plan.spans[0].attachment.is_external_root());
        assert!(plan.spans[0].attachment.uses_utterance_root_anchor());
        assert_eq!(plan.spans[0].attachment.source_deferred_index(), Some(1));
    }

    // ========================================================================
    // Family B (planner level), RED tests pinning the planner's
    // invariant violation that produces wild-corpus `head=0/deprel≠ROOT`.
    //
    // See the L2 architectural-reassessment notes (Family B partition,
    // §4) for the structural rationale these tests pin.
    //
    // Bug location (suspected): `planned_attachment` at line 174-176:
    //
    //     if primary_head == 0 && utterance_root_candidate.is_none() {
    //         utterance_root_candidate = Some((global_idx, current.primary.deprel.clone()));
    //     }
    //
    // The planner copies the primary parse's deprel verbatim into
    // `host_deprel`. If the primary's deprel for the head=0 token is
    // anything other than "root" (case-insensitive), Stanza noise,
    // edge-case copular constructions, mis-typed root labels, the
    // splice then writes `head=0` paired with that non-"root" deprel,
    // violating the bidirectional invariant `(head==0) ⟺ (deprel==ROOT)`.
    //
    // Wild evidence: sastre03.cha:843, herring09.cha:2570,
    // sastre03.cha:2823 each exhibit `head=0` with `deprel ∈
    // {DET, NMOD, …}`. See plan §3 Family B for full traces.
    //
    // Architectural rule the fix should enforce:
    //
    //     When `root_anchor == UtteranceRoot`, `host_deprel` MUST be
    //     "root" (case-insensitive). The deprel of the secondary span's
    //     externally-anchored root is determined by HOW it attaches to
    //     the host (utterance root → "root"; host governor → primary
    //     parse's deprel for the L2 position), not by what the primary
    //     happened to label the L2 token internally.
    // ========================================================================

    /// **Family B planner RED-1**, when the L2 word is the host
    /// primary's utterance root and primary's deprel is *not* "root"
    /// (rare but observed in the wild via Stanza noise / typed
    /// non-root labels), the planner must still emit
    /// `host_deprel = "root"` for the `UtteranceRoot` attachment.
    /// Otherwise the splice writes `head=0` paired with the bogus
    /// deprel and the validator fires E722.
    ///
    /// EXPECTED on current build: FAILS, `planned_attachment` at
    /// `plan.rs:175` copies `current.primary.deprel.clone()` verbatim,
    /// so `host_deprel = "det"` here.
    #[test]
    fn family_b_planner_utterance_root_attachment_must_use_root_deprel_not_primary_deprel() {
        // Single L2 word; primary parse marked it head=0 (utterance
        // root) but with deprel="det", exactly the shape that produces
        // the wild `head=0/deprel=DET` pattern at sastre03.cha:843.
        let deferred = vec![make_deferred(5, 1, "spa", "det", 0, "el")];

        let plan = plan_dispatch_spans(&deferred);
        assert_eq!(plan.spans.len(), 1);

        let attachment = &plan.spans[0].attachment;
        assert!(
            attachment.is_external_root(),
            "head=0 L2 word must be planned as ExternalRoot, not InternalRoot"
        );
        assert!(
            attachment.uses_utterance_root_anchor(),
            "head=0 L2 word must anchor as UtteranceRoot"
        );

        let host_deprel = attachment.external_root_deprel().expect("external root");
        assert!(
            host_deprel.as_str().eq_ignore_ascii_case("root"),
            "Family B planner bug: when the planned attachment is \
             UtteranceRoot, host_deprel must be \"root\" so the splice's \
             eventual head=0 promotion pairs with deprel=ROOT (joint \
             invariant). Got host_deprel={:?}; the planner copied the \
             primary's deprel verbatim instead of using \"root\".",
            host_deprel.as_str()
        );
    }

    /// **Family B planner RED-2**, same bug surface for `nmod`.
    /// Mirrors herring09.cha:2570 where the wild output has chunk 7
    /// `head=0/deprel=NMOD`.
    ///
    /// EXPECTED on current build: FAILS for the same reason as -1.
    #[test]
    fn family_b_planner_utterance_root_attachment_rejects_nmod_deprel_propagation() {
        let deferred = vec![make_deferred(5, 1, "spa", "nmod", 0, "camino")];

        let plan = plan_dispatch_spans(&deferred);
        let attachment = &plan.spans[0].attachment;

        assert!(attachment.uses_utterance_root_anchor());
        let host_deprel = attachment.external_root_deprel().expect("external root");
        assert!(
            host_deprel.as_str().eq_ignore_ascii_case("root"),
            "Family B planner bug (nmod variant): host_deprel must be \
             \"root\" for UtteranceRoot anchors regardless of the primary \
             parse's local deprel for the L2 token. Got {:?}.",
            host_deprel.as_str()
        );
    }

    /// **Family B planner GREEN guard**, the canonical case where
    /// the primary's head=0 deprel IS "root". This must keep working
    /// after the fix; locks the GREEN baseline so the fix doesn't
    /// over-correct.
    #[test]
    fn family_b_planner_utterance_root_with_root_primary_deprel_stays_root() {
        let deferred = vec![make_deferred(5, 1, "spa", "root", 0, "camino")];

        let plan = plan_dispatch_spans(&deferred);
        let attachment = &plan.spans[0].attachment;

        assert!(attachment.uses_utterance_root_anchor());
        let host_deprel = attachment.external_root_deprel().expect("external root");
        assert!(
            host_deprel.as_str().eq_ignore_ascii_case("root"),
            "GREEN baseline: when primary deprel is already \"root\", \
             host_deprel stays \"root\". Got {:?}.",
            host_deprel.as_str()
        );
    }

    /// A planned span carries the terminator of the positions it groups.
    ///
    /// The utterance-level source of that terminator is tested where it
    /// enters, in `extract`; here it only has to survive grouping.
    #[test]
    fn planned_span_carries_the_positions_terminator() {
        let mut def = make_deferred(5, 1, "spa", "obj", 1, "camino");
        def.terminator = talkbank_model::Terminator::Question {
            span: talkbank_model::Span::DUMMY,
        };

        let plan = plan_dispatch_spans(&[def]);

        assert!(
            matches!(
                plan.spans[0].terminator,
                talkbank_model::Terminator::Question { .. }
            ),
            "grouping must not substitute a period; got {:?}",
            plan.spans[0].terminator
        );
    }
}
