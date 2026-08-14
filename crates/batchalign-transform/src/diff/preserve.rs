//! Dependent tier preservation for incremental processing.
//!
//! When the diff engine determines that an utterance is unchanged, its
//! dependent tiers (%mor, %gra, %wor, etc.) from the "before" file can be
//! copied to the "after" file, avoiding unnecessary reprocessing.

// Denies wildcard matches over closed enums, per chatter's ratchet. This
// file decides which tiers survive an incremental rerun, so a variant
// quietly joining the no-match set means a tier the caller asked to preserve
// is dropped while the call reports success.
#![deny(clippy::wildcard_enum_match_arm)]
// Test code is exempt, matching this crate's existing treatment of the panic
// lints: `other => panic!("unexpected {other:?}")` is how a test says a variant
// should be unreachable, and denying it there would push tests toward asserting
// less rather than more.
#![cfg_attr(test, allow(clippy::wildcard_enum_match_arm))]

use talkbank_model::UtteranceIdx;
use talkbank_model::model::{ChatFile, DependentTier, Line};

use crate::dependent_tiers::replace_or_add_tier;

/// Which dependent tier kinds to copy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TierKind {
    /// %mor tier.
    Mor,
    /// %gra tier.
    Gra,
    /// %wor tier.
    Wor,
}

/// Check if a dependent tier matches any of the requested kinds.
fn tier_matches(tier: &DependentTier, kinds: &[TierKind]) -> bool {
    // Exhaustive over `TierKind`, so adding a kind stops compiling here until
    // someone says which tier it names. The guarded form this replaced ended
    // in `_ => {}`, which reads as "no match" and would also have swallowed a
    // new kind silently: a caller asking to preserve it would have been told,
    // truthfully in type and falsely in fact, that no tier matched.
    kinds.iter().any(|kind| match kind {
        TierKind::Mor => matches!(tier, DependentTier::Mor(_)),
        TierKind::Gra => matches!(tier, DependentTier::Gra(_)),
        TierKind::Wor => matches!(tier, DependentTier::Wor(_)),
    })
}

/// Copy specified dependent tiers from a "before" utterance to an "after" utterance.
///
/// Uses `replace_or_add_tier` for idempotent insertion, safe to call multiple times.
///
/// Returns the number of tiers copied.
pub fn copy_dependent_tiers(
    before: &ChatFile,
    before_idx: UtteranceIdx,
    after: &mut ChatFile,
    after_idx: UtteranceIdx,
    kinds: &[TierKind],
) -> usize {
    // First, collect the tiers to copy from the "before" file.
    let tiers_to_copy: Vec<DependentTier> = {
        let mut utt_count = 0usize;
        let mut result = Vec::new();
        for line in &before.lines {
            if let Line::Utterance(utt) = line {
                if utt_count == before_idx.raw() {
                    for tier in utt.dependent_tiers.iter() {
                        if tier_matches(&tier.tier, kinds) {
                            // Copy the tier itself, not the entry: separator is
                            // provenance of the SOURCE file's line, and the destination's
                            // own spacing governs there (replace keeps the destination's,
                            // append is CLEAN).
                            result.push(tier.tier.clone());
                        }
                    }
                    break;
                }
                utt_count += 1;
            }
        }
        result
    };

    let copied = tiers_to_copy.len();

    // Then inject them into the "after" file.
    let mut utt_count = 0usize;
    for line in after.lines.as_mut_slice().iter_mut() {
        if let Line::Utterance(utt) = line {
            if utt_count == after_idx.raw() {
                for tier in tiers_to_copy {
                    replace_or_add_tier(&mut utt.dependent_tiers, tier);
                }
                break;
            }
            utt_count += 1;
        }
    }

    copied
}
