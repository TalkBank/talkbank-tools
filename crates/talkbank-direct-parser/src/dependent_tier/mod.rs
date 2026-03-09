//! Dependent-tier parsing dispatcher for full `%tier:\t...` lines.
//!
//! This module performs tier-label routing (`%mor`, `%gra`, `%pho`, `%wor`, etc.),
//! validates basic tier shape, and preserves parse-health taint when recovery is
//! needed.
//!
//! # Related CHAT Manual Sections
//!
//! - <https://talkbank.org/0info/manuals/CHAT.html#Dependent_Tiers>
//! - <https://talkbank.org/0info/manuals/CHAT.html#Morphological_Tier>
//! - <https://talkbank.org/0info/manuals/CHAT.html#GrammaticalRelations_Tier>
//! - <https://talkbank.org/0info/manuals/CHAT.html#Phonology_Tier>
//! - <https://talkbank.org/0info/manuals/CHAT.html#Sign_Tier>
//! - <https://talkbank.org/0info/manuals/CHAT.html#Word_Timing_Tier>

mod dispatch;
mod helpers;
#[cfg(test)]
mod tests;

pub(crate) use dispatch::{TierParseResult, parse_dependent_tier_internal};
pub(crate) use helpers::classify_dependent_tier_parse_health;

use talkbank_model::ErrorSink;
use talkbank_model::ParseOutcome;
use talkbank_model::dependent_tier::DependentTier;

/// Parse a generic dependent tier from a full tier line.
///
/// The input should be a complete tier line like `%mor:\tpro|I v|want .`
/// This dispatcher extracts the tier type from the prefix and routes to
/// the appropriate content parser.
pub fn parse_dependent_tier_impl(
    input: &str,
    offset: usize,
    errors: &impl ErrorSink,
) -> ParseOutcome<DependentTier> {
    match parse_dependent_tier_internal(input, offset, errors) {
        TierParseResult::Clean(tier) | TierParseResult::Recovered(tier, _) => {
            ParseOutcome::parsed(tier)
        }
        TierParseResult::Failed(_) => ParseOutcome::rejected(),
    }
}
