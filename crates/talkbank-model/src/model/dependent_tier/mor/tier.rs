//! `%mor` dependent-tier model and content validation helpers.
//!
//! This module defines the tier-level container used by parser output and
//! alignment logic, plus lexical-content checks shared by `%mor` validators.
//!
//! CHAT reference anchor:
//! - [Morphological tier](https://talkbank.org/0info/manuals/CHAT.html#Morphological_Tier)

use super::super::WriteChat;
use super::item::Mor;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::ops::{Deref, DerefMut};
use talkbank_derive::{SemanticEq, SpanShift};

/// Type of morphological analysis tier.
///
/// The enum is intentionally explicit even though it currently has one variant.
/// This keeps tier-prefix logic uniform with other dependent-tier families.
///
/// # References
///
/// - [Morphological Tier](https://talkbank.org/0info/manuals/CHAT.html#Morphological_Tier)
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema, SemanticEq, SpanShift,
)]
pub enum MorTierType {
    /// Standard morphological analysis tier (%mor).
    Mor,
}

impl WriteChat for MorTierType {
    /// Writes the serialized tier tag used in CHAT files.
    ///
    /// Keeping this on `MorTierType` lets callers format tier prefixes without
    /// constructing a full [`MorTier`] value first.
    fn write_chat<W: std::fmt::Write>(&self, w: &mut W) -> std::fmt::Result {
        match self {
            MorTierType::Mor => w.write_str("%mor"),
        }
    }
}

/// Morphological analysis tier (%mor).
///
/// Provides word-by-word UD-style morphological annotation aligned with the main tier.
/// Each word receives a morphological code specifying part of speech, lemma,
/// and grammatical features.
///
/// # CHAT Format Example
///
/// ```text
/// *CHI: I want cookies .
/// %mor: pron|I-Prs-Nom-S1 verb|want-Fin-Ind-Pres-S1 noun|cookie-Plur .
/// ```
///
/// # Morphological Format
///
/// Each mor item has the UD structure: `POS|lemma[-Feature]*`
/// - **POS**: UD-style part-of-speech tag (e.g., `noun`, `verb`, `pron`)
/// - **Lemma**: Base form (e.g., `I`, `want`, `cookie`)
/// - **Features**: UD feature values (e.g., `-Plur`, `-Fin-Ind-Pres-S3`)
///
/// # Alignment
///
/// Mor tiers align 1-to-1 with alignable main tier content (words, not pauses/events).
/// See `crate::alignment::mor` for alignment algorithm.
///
/// # Terminator
///
/// The terminator is optional. The aligner validates that either both the main tier
/// and %mor tier have a terminator, or neither does.
///
/// # References
///
/// - [Morphological Tier](https://talkbank.org/0info/manuals/CHAT.html#Morphological_Tier)
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema, SemanticEq, SpanShift)]
pub struct MorTier {
    /// Type of morphological tier.
    pub tier_type: MorTierType,

    /// Morphological items aligned with main tier content.
    pub items: MorItems,

    /// Optional terminator. Must match main tier's terminator presence.
    pub terminator: Option<smol_str::SmolStr>,

    /// Source span for error reporting (not serialized to JSON)
    #[serde(skip)]
    #[schemars(skip)]
    pub span: crate::Span,
}

impl MorTier {
    /// Construct a morphological tier with explicit type and items.
    ///
    /// The constructor does not infer or validate alignment details. Those checks
    /// run later in dedicated validation/alignment stages.
    pub fn new(tier_type: MorTierType, items: Vec<Mor>) -> Self {
        Self {
            tier_type,
            items: items.into(),
            terminator: None,
            span: crate::Span::DUMMY,
        }
    }

    /// Construct a standard `%mor` tier.
    ///
    /// This is the common constructor used by parser outputs and tests when no
    /// alternate tier kind is needed.
    pub fn new_mor(items: Vec<Mor>) -> Self {
        Self::new(MorTierType::Mor, items)
    }

    /// Attach optional tier terminator.
    ///
    /// The terminator participates in `%mor`/main-tier alignment and is counted
    /// as a chunk when present.
    pub fn with_terminator(mut self, terminator: Option<smol_str::SmolStr>) -> Self {
        self.terminator = terminator;
        self
    }

    /// Returns `true` when tier type is `%mor`.
    ///
    /// This helper is mainly useful in generic code paths over tier enums.
    pub fn is_mor(&self) -> bool {
        self.tier_type == MorTierType::Mor
    }

    /// Attach source span for diagnostics.
    ///
    /// Parser-generated values should set this to real offsets so `%mor` validation
    /// reports can point back to the original transcript line.
    pub fn with_span(mut self, span: crate::Span) -> Self {
        self.span = span;
        self
    }

    /// Count total number of chunks (including post-clitics and terminator)
    ///
    /// This is used for %gra alignment, where each chunk (including the terminator)
    /// should have a corresponding %gra relation.
    pub fn count_chunks(&self) -> usize {
        let item_chunks: usize = self.items.iter().map(|m| m.count_chunks()).sum();
        // Add 1 for terminator if present (terminator counts as a chunk for alignment)
        if self.terminator.is_some() {
            item_chunks + 1
        } else {
            item_chunks
        }
    }

    /// Number of `%mor` items (excluding terminator/chunk expansion).
    ///
    /// Each item may still expand to multiple `%gra` chunks if it contains
    /// post-clitics.
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// Returns `true` when there are no `%mor` items.
    ///
    /// A tier with no items can still have a terminator; use [`Self::count_chunks`]
    /// when alignment logic needs the full chunk count.
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// Serialize full `%mor` line (`%mor:\t...`) to CHAT text.
    ///
    /// This writes prefix, items, and optional terminator in canonical order.
    /// It is the non-allocating path used by `WriteChat`.
    pub fn write_chat<W: std::fmt::Write>(&self, w: &mut W) -> std::fmt::Result {
        match self.tier_type {
            MorTierType::Mor => w.write_str("%mor:\t")?,
        }

        for (i, item) in self.items.iter().enumerate() {
            if i > 0 {
                w.write_char(' ')?;
            }
            item.write_chat(w)?;
        }

        // Write terminator if present
        if let Some(ref term) = self.terminator {
            // Only add space if there are items before the terminator
            if !self.items.is_empty() {
                w.write_char(' ')?;
            }
            w.write_str(term)?;
        }

        Ok(())
    }

    /// Serialize full `%mor` line to an owned string.
    ///
    /// Prefer [`Self::write_chat`] when writing into existing buffers to avoid
    /// transient allocation.
    pub fn to_chat(&self) -> String {
        let mut s = String::new();
        let _ = self.write_chat(&mut s);
        s
    }

    /// Write tier content only (items and terminator), without the tier prefix (%mor:\t).
    ///
    /// This is used for roundtrip testing against golden data that contains
    /// content-only, and for the ChatParser API which expects content-only input.
    pub fn write_content<W: std::fmt::Write>(&self, w: &mut W) -> std::fmt::Result {
        for (i, item) in self.items.iter().enumerate() {
            if i > 0 {
                w.write_char(' ')?;
            }
            item.write_chat(w)?;
        }

        // Write terminator if present
        if let Some(ref term) = self.terminator {
            // Only add space if there are items before the terminator
            if !self.items.is_empty() {
                w.write_char(' ')?;
            }
            w.write_str(term)?;
        }

        Ok(())
    }

    /// Serialize content-only `%mor` payload to an owned string.
    ///
    /// This mirrors [`Self::write_content`] and is mainly a convenience for
    /// tests and debugging output.
    pub fn to_content(&self) -> String {
        let mut s = String::new();
        let _ = self.write_content(&mut s);
        s
    }
}

impl MorTier {
    /// Validate lexical content of all `%mor` items in this tier.
    ///
    /// Checks for empty POS/lemma/feature fields and reports `E711` diagnostics
    /// for each violation found.
    pub fn validate_content(&self, errors: &impl crate::ErrorSink) {
        validate_mor_content(&self.items, self.span, errors);
    }
}

impl WriteChat for MorTier {
    /// Serializes the full `%mor` line (prefix, items, optional terminator).
    fn write_chat<W: std::fmt::Write>(&self, w: &mut W) -> std::fmt::Result {
        MorTier::write_chat(self, w)
    }
}

/// Newtype wrapper around a list of morphological items for a %mor tier.
///
/// # Reference
///
/// - [Morphological tier](https://talkbank.org/0info/manuals/CHAT.html#Morphological_Tier)
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema, SemanticEq, SpanShift)]
#[serde(transparent)]
#[schemars(transparent)]
pub struct MorItems(pub Vec<Mor>);

impl MorItems {
    /// Create a new list of morphological items.
    ///
    /// Construction is intentionally lightweight so parser code can build the
    /// model first and run validation in a separate phase.
    pub fn new(items: Vec<Mor>) -> Self {
        Self(items)
    }

    /// Returns `true` if the list contains no items.
    ///
    /// This reflects only raw item count, not whether a parent tier has a
    /// terminator chunk.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl Deref for MorItems {
    type Target = Vec<Mor>;

    /// Borrows the underlying `%mor` item vector.
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for MorItems {
    /// Mutably borrows the underlying `%mor` item vector.
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl From<Vec<Mor>> for MorItems {
    /// Wraps `%mor` items without copying.
    fn from(items: Vec<Mor>) -> Self {
        Self(items)
    }
}

impl crate::validation::Validate for MorItems {
    /// Item-level constraints are enforced by `%mor` and alignment validators.
    fn validate(
        &self,
        _context: &crate::validation::ValidationContext,
        _errors: &impl crate::ErrorSink,
    ) {
    }
}

/// Validate content integrity of %mor items.
///
/// Checks every `MorWord` (in main words and post-clitics) for:
/// - E711: Empty lemma (`pos|` with no lemma after the pipe)
/// - E711: Empty POS category (`|lemma` with no POS before the pipe)
/// - E711: Empty feature (bare `-` separator with no text)
///
/// Structural alignment checks are intentionally out of scope here; this helper
/// only validates per-token lexical morphology content.
///
pub fn validate_mor_content(items: &[Mor], span: crate::Span, errors: &impl crate::ErrorSink) {
    use super::word::MorWord;
    use crate::{ErrorCode, ParseError, Severity};

    /// Validates a single `%mor` word for empty POS/lemma/feature fields.
    fn check_word(word: &MorWord, span: crate::Span, errors: &impl crate::ErrorSink) {
        if word.lemma.is_empty() {
            errors.report(
                ParseError::at_span(
                    ErrorCode::MorEmptyContent,
                    Severity::Error,
                    span,
                    format!("%mor word has empty lemma (POS='{}')", word.pos.as_str()),
                )
                .with_suggestion("Ensure the lemma is not empty"),
            );
        }
        if word.pos.is_empty() {
            errors.report(
                ParseError::at_span(
                    ErrorCode::MorEmptyContent,
                    Severity::Error,
                    span,
                    format!(
                        "%mor word has empty POS category (lemma='{}')",
                        word.lemma.as_str()
                    ),
                )
                .with_suggestion("Ensure the part-of-speech category is not empty"),
            );
        }
        for feature in &word.features {
            if feature.is_empty() {
                errors.report(
                    ParseError::at_span(
                        ErrorCode::MorEmptyContent,
                        Severity::Error,
                        span,
                        format!(
                            "%mor word has empty feature (lemma='{}')",
                            word.lemma.as_str()
                        ),
                    )
                    .with_suggestion("Remove the empty feature or provide feature text"),
                );
            }
        }
    }

    for item in items {
        // Check main word
        check_word(&item.main, span, errors);
        // Check post-clitics
        for clitic in &item.post_clitics {
            check_word(clitic, span, errors);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::analysis::{MorFeature, MorStem, PosCategory};
    use super::super::item::Mor;
    use super::super::word::MorWord;
    use super::*;
    use crate::{ErrorCode, ErrorCollector};

    /// Builds a `%mor` word fixture with given POS and lemma.
    ///
    /// Keeping this helper local avoids repetitive setup in each validation test.
    fn make_word(pos: &str, lemma: &str) -> MorWord {
        MorWord::new(PosCategory::new(pos), MorStem::new(lemma))
    }

    /// Wraps a word fixture into a single `%mor` item.
    ///
    /// Tests compose these items into tiers to exercise tier-level validators.
    fn make_mor(word: MorWord) -> Mor {
        Mor::new(word)
    }

    /// Builds a `%mor` tier fixture with a terminator.
    ///
    /// The terminator mirrors common corpus shape and keeps chunk accounting realistic.
    fn make_tier(items: Vec<Mor>) -> MorTier {
        MorTier::new_mor(items).with_terminator(Some(".".into()))
    }

    /// Well-formed `%mor` content emits no `E711` diagnostics.
    ///
    /// This is the baseline guard for the validator's non-error path.
    #[test]
    fn test_mor_valid_content_no_errors() {
        let tier = make_tier(vec![
            make_mor(make_word("noun", "dog")),
            make_mor(make_word("verb", "run").with_feature(MorFeature::new("Past"))),
        ]);
        let errors = ErrorCollector::new();
        tier.validate_content(&errors);
        assert!(errors.into_vec().is_empty());
    }

    /// Empty lemma fields are rejected with `E711`.
    ///
    /// The message should explicitly mention the missing lemma component.
    #[test]
    fn test_mor_empty_lemma_produces_e711() {
        let tier = make_tier(vec![make_mor(make_word("noun", ""))]);
        let errors = ErrorCollector::new();
        tier.validate_content(&errors);
        let errs = errors.into_vec();
        assert_eq!(errs.len(), 1);
        assert_eq!(errs[0].code, ErrorCode::MorEmptyContent);
        assert!(errs[0].message.contains("empty lemma"));
    }

    /// Empty POS categories are rejected with `E711`.
    ///
    /// This protects against malformed `|lemma` forms.
    #[test]
    fn test_mor_empty_pos_category_produces_e711() {
        let tier = make_tier(vec![make_mor(make_word("", "dog"))]);
        let errors = ErrorCollector::new();
        tier.validate_content(&errors);
        let errs = errors.into_vec();
        assert_eq!(errs.len(), 1);
        assert_eq!(errs[0].code, ErrorCode::MorEmptyContent);
        assert!(errs[0].message.contains("empty POS category"));
    }

    /// Empty feature entries are rejected with `E711`.
    ///
    /// Bare `-` separators must not survive normalization.
    #[test]
    fn test_mor_empty_feature_produces_e711() {
        let word = make_word("verb", "walk").with_feature(MorFeature::new(""));
        let tier = make_tier(vec![make_mor(word)]);
        let errors = ErrorCollector::new();
        tier.validate_content(&errors);
        let errs = errors.into_vec();
        assert_eq!(errs.len(), 1);
        assert_eq!(errs[0].code, ErrorCode::MorEmptyContent);
        assert!(errs[0].message.contains("empty feature"));
    }
}
