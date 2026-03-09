//! Main-tier to `%wor` alignment.
//!
//! References:
//! - <https://talkbank.org/0info/manuals/CHAT.html#Word_Timing_Tier>
//! - <https://talkbank.org/0info/manuals/CHAT.html#Dependent_Tiers>

use super::helpers::{AlignableItem, AlignmentDomain};
use super::traits::{AlignableTier, MismatchFormat, TierAlignmentResult, positional_align};
use super::types::AlignmentPair;
use crate::model::{MainTier, WorTier};
use crate::{ErrorCode, ParseError, Span};
use schemars::JsonSchema;
use talkbank_derive::SpanShift;

/// Result of aligning main-tier units to `%wor` timing tokens.
///
/// The pair stream preserves positional intent, while `errors` explains count
/// mismatches when placeholder rows are required.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize, JsonSchema, SpanShift)]
pub struct WorAlignment {
    /// Positional mapping rows (`main_index`, `wor_index`).
    pub pairs: Vec<AlignmentPair>,

    /// Diagnostics produced while checking `%wor` count/position invariants.
    pub errors: Vec<ParseError>,
}

impl WorAlignment {
    /// Creates an empty alignment accumulator.
    ///
    /// Used by the builder-style pass before rows and diagnostics are appended.
    pub fn new() -> Self {
        Self {
            pairs: Vec::new(),
            errors: Vec::new(),
        }
    }

    /// Appends one positional alignment row.
    ///
    /// This consumes and returns `Self` so call sites can chain in tight loops.
    pub fn with_pair(mut self, pair: AlignmentPair) -> Self {
        self.pairs.push(pair);
        self
    }

    /// Appends one mismatch diagnostic.
    ///
    /// Multiple errors can be accumulated when a caller chooses not to short-circuit.
    pub fn with_error(mut self, error: ParseError) -> Self {
        self.errors.push(error);
        self
    }

    /// Returns `true` when no `%wor` mismatch diagnostics were emitted.
    ///
    /// In practice this means all rows in `pairs` are complete one-to-one mappings.
    pub fn is_error_free(&self) -> bool {
        self.errors.is_empty()
    }
}

impl Default for WorAlignment {
    /// Builds an empty main-to-`%wor` alignment result.
    fn default() -> Self {
        Self::new()
    }
}

impl TierAlignmentResult for WorAlignment {
    type Pair = AlignmentPair;

    fn pairs(&self) -> &[AlignmentPair] {
        &self.pairs
    }

    fn errors(&self) -> &[ParseError] {
        &self.errors
    }

    fn push_pair(&mut self, pair: AlignmentPair) {
        self.pairs.push(pair);
    }

    fn push_error(&mut self, error: ParseError) {
        self.errors.push(error);
    }
}

impl AlignableTier for WorTier {
    const DOMAIN: AlignmentDomain = AlignmentDomain::Wor;

    fn tier_name(&self) -> &str {
        "%wor tier"
    }

    fn target_count(&self) -> usize {
        self.word_count()
    }

    fn extract_target_items(&self) -> Vec<AlignableItem> {
        self.words()
            .map(|w| AlignableItem {
                text: w.cleaned_text().to_string(),
                description: None,
            })
            .collect()
    }

    fn span(&self) -> Span {
        self.span
    }

    fn error_code_too_few(&self) -> ErrorCode {
        ErrorCode::PhoCountMismatchTooFew
    }

    fn error_code_too_many(&self) -> ErrorCode {
        ErrorCode::PhoCountMismatchTooMany
    }

    fn suggestion_too_few(&self) -> &str {
        "Add word timing tokens to %wor tier to match main tier words"
    }

    fn suggestion_too_many(&self) -> &str {
        "Remove extra word timing tokens from %wor tier"
    }

    fn mismatch_format(&self) -> MismatchFormat {
        MismatchFormat::Diff
    }
}

/// Align main-tier content to `%wor` timing tokens using 1:1 positional pairing.
///
/// Uses the generic [`positional_align`] algorithm via the [`AlignableTier`]
/// implementation on [`WorTier`]. The `%wor` tier uses LCS-based diff formatting
/// for mismatch diagnostics since both sides are word sequences.
///
/// # Example: Retrace handling
///
/// ```ignore
/// use talkbank_model::model::{MainTier, WorTier, UtteranceContent, Word, Terminator};
/// use talkbank_model::Span;
/// use talkbank_model::alignment::align_main_to_wor;
///
/// // Main tier: "you [/] you like it ."
/// let main = MainTier::new(
///     "CHI",
///     vec![
///         UtteranceContent::Word(Box::new(Word::with_retrace("you", "you"))),
///         UtteranceContent::Word(Box::new(Word::new_unchecked("you", "you"))),
///         UtteranceContent::Word(Box::new(Word::new_unchecked("like", "like"))),
///         UtteranceContent::Word(Box::new(Word::new_unchecked("it", "it"))),
///     ],
///     Terminator::Period { span: Span::DUMMY },
/// );
///
/// // %wor tier: "&you you like it ."
/// // Only 4 tokens expected (retraced "you" has filler prefix &)
/// let wor = WorTier::new(vec!["&you", "you", "like", "it"]);
///
/// let alignment = align_main_to_wor(&main, &wor);
/// assert!(alignment.errors.is_empty());
/// assert_eq!(alignment.pairs.len(), 4); // excludes retrace + terminator
/// ```
pub fn align_main_to_wor(main: &MainTier, wor: &WorTier) -> WorAlignment {
    let (pairs, errors) = positional_align(main, wor);
    WorAlignment { pairs, errors }
}
