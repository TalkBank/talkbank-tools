//! Alignment metadata for tracking tier-to-tier correspondences.
//!
//! This module provides the [`AlignmentSet`] structure which stores pre-computed
//! alignment information between dependent tiers and their source content, plus
//! the alignable units used to derive those mappings.
//!
//! References:
//! - <https://talkbank.org/0info/manuals/CHAT.html#Dependent_Tiers>
//! - <https://talkbank.org/0info/manuals/CHAT.html#Morphological_Tier>
//! - <https://talkbank.org/0info/manuals/CHAT.html#GrammaticalRelations_Tier>

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use talkbank_derive::SpanShift;

use crate::Span;

/// Single alignable unit within a tier alignment domain.
///
/// Reference: <https://talkbank.org/0info/manuals/CHAT.html#Dependent_Tiers>
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema, SpanShift)]
pub struct AlignmentUnit {
    /// Index in the alignment sequence (0-indexed).
    pub index: usize,

    /// Source span for this unit (if available).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub span: Option<Span>,
}

/// Alignable units for each tier/domain pair.
///
/// References:
/// - <https://talkbank.org/0info/manuals/CHAT.html#Dependent_Tiers>
/// - <https://talkbank.org/0info/manuals/CHAT.html#Word_Tier>
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema, SpanShift, Default)]
pub struct AlignmentUnits {
    /// Main-tier units for %mor alignment.
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub main_mor: Vec<AlignmentUnit>,

    /// Main-tier units for %pho/%mod alignment.
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub main_pho: Vec<AlignmentUnit>,

    /// Main-tier units for %sin alignment.
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub main_sin: Vec<AlignmentUnit>,

    /// Main-tier units for %wor alignment.
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub main_wor: Vec<AlignmentUnit>,

    /// %mor tier items.
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub mor: Vec<AlignmentUnit>,

    /// %mor chunk units (for %gra alignment).
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub mor_chunks: Vec<AlignmentUnit>,

    /// %gra relations.
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub gra: Vec<AlignmentUnit>,

    /// %pho items.
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub pho: Vec<AlignmentUnit>,

    /// %wor items (alignable content only).
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub wor: Vec<AlignmentUnit>,

    /// %mod items.
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub mod_: Vec<AlignmentUnit>,

    /// %sin items.
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub sin: Vec<AlignmentUnit>,

    /// %modsyl words.
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub modsyl: Vec<AlignmentUnit>,

    /// %phosyl words.
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub phosyl: Vec<AlignmentUnit>,

    /// %phoaln words.
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub phoaln: Vec<AlignmentUnit>,
}

/// Alignment metadata for an utterance's dependent tiers.
///
/// References:
/// - <https://talkbank.org/0info/manuals/CHAT.html#Dependent_Tiers>
/// - <https://talkbank.org/0info/manuals/CHAT.html#Morphological_Tier>
/// - <https://talkbank.org/0info/manuals/CHAT.html#GrammaticalRelations_Tier>
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema, SpanShift)]
pub struct AlignmentSet {
    /// Alignable unit lists for each tier/domain.
    pub units: AlignmentUnits,

    /// Main → %mor alignment (if %mor tier present).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mor: Option<crate::alignment::MorAlignment>,

    /// %mor → %gra alignment (if both tiers present).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gra: Option<crate::alignment::GraAlignment>,

    /// Main → %pho alignment (if %pho tier present).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pho: Option<crate::alignment::PhoAlignment>,

    /// Main → %wor alignment (if %wor tier present).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wor: Option<crate::alignment::WorAlignment>,

    /// Main → %mod alignment (if %mod/%xpho tier present).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mod_: Option<crate::alignment::PhoAlignment>,

    /// Main → %sin alignment (if %sin tier present).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sin: Option<crate::alignment::SinAlignment>,

    /// %modsyl → %mod alignment (tier-to-tier, word count).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub modsyl: Option<crate::alignment::PhoAlignment>,

    /// %phosyl → %pho alignment (tier-to-tier, word count).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub phosyl: Option<crate::alignment::PhoAlignment>,

    /// %phoaln → %mod & %pho alignment (tier-to-tier, word count).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub phoaln: Option<crate::alignment::PhoAlignment>,
}

impl AlignmentSet {
    /// Creates a new alignment set from pre-built alignment units.
    pub fn new(units: AlignmentUnits) -> Self {
        Self {
            units,
            mor: None,
            gra: None,
            pho: None,
            wor: None,
            mod_: None,
            sin: None,
            modsyl: None,
            phosyl: None,
            phoaln: None,
        }
    }

    /// Check if all alignments are error-free (no errors).
    ///
    /// `%wor` alignment errors are intentionally excluded: `%wor` is a
    /// timing-annotation tier, not a structural alignment tier.  Old corpus
    /// files (pre-2026-04) include `xxx`/`yyy`/`www` tokens in `%wor` under the
    /// previous policy; those produce count-mismatch errors in `WorAlignment`
    /// that are only used internally by batchalign's injection layer — they must
    /// not affect corpus validation verdicts.
    pub fn is_error_free(&self) -> bool {
        let mor_ok = self.mor.as_ref().is_none_or(|a| a.is_error_free());
        let gra_ok = self.gra.as_ref().is_none_or(|a| a.is_error_free());
        let pho_ok = self.pho.as_ref().is_none_or(|a| a.is_error_free());
        // %wor alignment errors intentionally excluded — see doc comment above.
        let mod_ok = self.mod_.as_ref().is_none_or(|a| a.is_error_free());
        let sin_ok = self.sin.as_ref().is_none_or(|a| a.is_error_free());
        let modsyl_ok = self.modsyl.as_ref().is_none_or(|a| a.is_error_free());
        let phosyl_ok = self.phosyl.as_ref().is_none_or(|a| a.is_error_free());
        let phoaln_ok = self.phoaln.as_ref().is_none_or(|a| a.is_error_free());

        mor_ok && gra_ok && pho_ok && mod_ok && sin_ok && modsyl_ok && phosyl_ok && phoaln_ok
    }

    /// Collect all alignment errors for corpus validation output.
    ///
    /// `%wor` alignment errors are intentionally excluded: `%wor` is a
    /// timing-annotation tier with no word-count invariant against the main tier
    /// after the 2026-04 policy change (untranscribed tokens `xxx`/`yyy`/`www`
    /// are excluded from the Wor domain).  `WorAlignment` errors are only used
    /// internally by batchalign's injection layer; they must never surface in
    /// `chatter validate` output.
    pub fn collect_errors(&self) -> Vec<&crate::ParseError> {
        let mut errors = Vec::new();

        if let Some(mor) = &self.mor {
            errors.extend(&mor.errors);
        }
        if let Some(gra) = &self.gra {
            errors.extend(&gra.errors);
        }
        if let Some(pho) = &self.pho {
            errors.extend(&pho.errors);
        }
        // %wor alignment errors intentionally excluded — see doc comment above.
        if let Some(mod_) = &self.mod_ {
            errors.extend(&mod_.errors);
        }
        if let Some(sin) = &self.sin {
            errors.extend(&sin.errors);
        }
        if let Some(modsyl) = &self.modsyl {
            errors.extend(&modsyl.errors);
        }
        if let Some(phosyl) = &self.phosyl {
            errors.extend(&phosyl.errors);
        }
        if let Some(phoaln) = &self.phoaln {
            errors.extend(&phoaln.errors);
        }

        errors
    }
}

impl Default for AlignmentSet {
    /// Builds an empty alignment set with no computed mappings.
    fn default() -> Self {
        Self::new(AlignmentUnits::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::alignment::WorAlignment;
    use crate::{ErrorCode, ParseError, Span};

    /// Build a dummy E715 error as would be generated by `align_main_to_wor`
    /// when a `%wor` tier has more tokens than the main tier's Wor-domain words.
    fn dummy_e715_error() -> ParseError {
        ParseError::build(ErrorCode::PhoCountMismatchTooMany)
            .at_span(Span::DUMMY)
            .message("Main tier has 0 alignable items, but %wor tier has 1 items")
            .finish()
            .expect("test error construction must succeed")
    }

    /// `AlignmentSet::collect_errors()` MUST NOT include errors from the `wor`
    /// field.
    ///
    /// `%wor` is a timing-annotation tier, not a structural alignment tier.
    /// Old corpus files (pre-2026-04) include `xxx`/`yyy`/`www` in `%wor`
    /// but the Wor domain now excludes those tokens from the main-tier count.
    /// Surfacing these count-mismatch errors in `chatter validate` produces
    /// thousands of false-positive E715 reports on valid corpus files.
    ///
    /// `align_main_to_wor` is still called by batchalign's injection layer to
    /// verify its own output — those errors must stay in the model — but they
    /// must never surface through `collect_errors()`.
    ///
    /// This test is currently RED: `collect_errors()` unconditionally includes
    /// `wor.errors` at line 197.
    #[test]
    fn collect_errors_excludes_wor_errors() {
        let mut set = AlignmentSet::default();
        // Inject a synthetic E715 error into the wor alignment slot.
        set.wor = Some(WorAlignment::new().with_error(dummy_e715_error()));

        let errors = set.collect_errors();

        assert!(
            errors.is_empty(),
            "collect_errors() must not surface %wor alignment errors; \
             got {} error(s): {:?}",
            errors.len(),
            errors.iter().map(|e| e.code.as_str()).collect::<Vec<_>>(),
        );
    }

    /// `AlignmentSet::is_error_free()` MUST NOT count `wor` errors.
    ///
    /// Same invariant: `%wor` alignment errors exist for batchalign's injection
    /// use, but must not affect the corpus validation verdict.
    #[test]
    fn is_error_free_ignores_wor_errors() {
        let mut set = AlignmentSet::default();
        set.wor = Some(WorAlignment::new().with_error(dummy_e715_error()));

        assert!(
            set.is_error_free(),
            "is_error_free() must ignore %wor alignment errors; \
             a %wor count mismatch is not a corpus validation failure"
        );
    }
}
