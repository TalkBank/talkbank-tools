//! Common index-pair primitives shared by tier-alignment passes.
//!
//! References:
//! - <https://talkbank.org/0info/manuals/CHAT.html#Main_Tier>
//! - <https://talkbank.org/0info/manuals/CHAT.html#Dependent_Tiers>
//! - <https://talkbank.org/0info/manuals/CHAT.html#Word_Tier>
//! - <https://talkbank.org/0info/manuals/CHAT.html#Morphological_Tier>
//! - <https://talkbank.org/0info/manuals/CHAT.html#GrammaticalRelations_Tier>

use schemars::JsonSchema;
use talkbank_derive::SpanShift;

/// One positional mapping entry between a source tier and a target tier.
///
/// Most entries are complete `Some -> Some` matches. Placeholder entries with
/// one `None` preserve mismatch shape (extra/missing units) so diagnostics can
/// still report concrete positions for both tiers.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize, JsonSchema, SpanShift)]
pub struct AlignmentPair {
    /// Index in the source tier (for example the main tier), or `None` for placeholder rows.
    pub source_index: Option<usize>,
    /// Index in the target tier (for example `%mor`/`%pho`/`%wor`), or `None` for placeholders.
    pub target_index: Option<usize>,
}

impl AlignmentPair {
    /// Builds one alignment row from optional source/target indices.
    ///
    /// Callers usually pass two `Some` values for normal matches, or one `None`
    /// when recording an insertion/deletion style mismatch.
    pub fn new(source_index: Option<usize>, target_index: Option<usize>) -> Self {
        Self {
            source_index,
            target_index,
        }
    }

    /// Returns `true` when this row is a concrete one-to-one match.
    ///
    /// Complete rows are the ones eligible for downstream pairwise joins.
    pub fn is_complete(&self) -> bool {
        self.source_index.is_some() && self.target_index.is_some()
    }

    /// Returns `true` when this row represents an unmatched position.
    ///
    /// Placeholder rows are emitted only for mismatches and are expected when
    /// tiers have different alignable counts.
    pub fn is_placeholder(&self) -> bool {
        !self.is_complete()
    }
}

impl super::traits::IndexPair for AlignmentPair {
    fn source(&self) -> Option<usize> {
        self.source_index
    }

    fn target(&self) -> Option<usize> {
        self.target_index
    }

    fn from_indices(source: Option<usize>, target: Option<usize>) -> Self {
        Self::new(source, target)
    }
}
