//! `LanguageCode` model and validation helpers.
//!
//! This type is the canonical language token used across headers, utterance
//! language metadata, and word-level language-switch annotations.
//!
//! References:
//! - <https://talkbank.org/0info/manuals/CHAT.html#Language_Codes>
//! - <https://talkbank.org/0info/manuals/CHAT.html#Language_Switching>

use crate::validation::{Validate, ValidationContext};
use crate::{ErrorCode, ErrorContext, ParseError, Severity, SourceLocation};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use talkbank_derive::{SemanticEq, SpanShift};

/// Interned ISO 639-3 language token used across header/main-tier language fields.
///
/// Three-letter language code following the ISO 639-3 standard. Used throughout
/// CHAT files to specify the language of utterances, participants, and transcripts.
///
/// ## Memory Optimization
///
/// This type uses `Arc<str>` with interning for memory efficiency:
/// - All codes are interned through a global interner
/// - Common codes (eng, spa, deu, etc.) are pre-populated on first use
/// - Cloning is O(1) (atomic reference count increment)
/// - Multiple occurrences of the same code share a single Arc
///
/// This reduces memory usage by 5-20MB for large corpora.
///
/// # CHAT Usage
///
/// **In headers:**
/// - `@Languages:` - Declares all languages used in the transcript
/// - `@Language of SPK:` - Specifies a participant's language
/// - `@ID` header field 1 - Primary language of transcript
///
/// **In main tiers:**
/// - `[- code]` - Language switching annotation for individual words
/// - `[+ code]` - Extended language annotation
///
/// # CHAT Format Examples
///
/// ```text
/// @Languages: eng, spa
/// @Language of CHI: eng
/// @ID: eng|corpus|CHI|...
/// *CHI: I want agua [- spa].
/// *MOT: say [+ eng] water.
/// ```
///
/// # Common Language Codes
///
/// - `eng` - English
/// - `spa` - Spanish
/// - `deu` - German (Deutsch)
/// - `fra` - French
/// - `zho` - Chinese
/// - `jpn` - Japanese
/// - `ita` - Italian
/// - `por` - Portuguese
/// - `rus` - Russian
/// - `ara` - Arabic
/// - `hin` - Hindi
/// - `kor` - Korean
///
/// # Validation
///
/// Parser acceptance is permissive; validation enforces:
/// - Three-letter lowercase format
/// - obvious placeholder rejection (`xyz`, `xxx`, `yyy`, `zzz`)
///
/// # References
///
/// - [CHAT Manual: Language Codes](https://talkbank.org/0info/manuals/CHAT.html#Language_Codes)
/// - [ISO 639-3 Standard](https://iso639-3.sil.org/)
/// - [Language Switching](https://talkbank.org/0info/manuals/CHAT.html#Language_Switching)
#[derive(
    Serialize, Deserialize, JsonSchema, Debug, Clone, PartialEq, Eq, Hash, SemanticEq, SpanShift,
)]
#[serde(transparent)]
pub struct LanguageCode(pub Arc<str>);

impl LanguageCode {
    /// Construct and intern a language token.
    ///
    /// Interning keeps repeated codes pointer-shared and avoids repeated heap
    /// allocation for high-frequency values like `eng`.
    pub fn new(value: impl AsRef<str>) -> Self {
        let s = value.as_ref();
        Self(crate::model::language_interner().intern(s))
    }

    /// Borrow as `&str`.
    ///
    /// Prefer this accessor instead of depending on the internal `Arc<str>`
    /// representation in callers.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl crate::model::WriteChat for LanguageCode {
    /// Writes the raw code token with no additional normalization.
    fn write_chat<W: std::fmt::Write>(&self, w: &mut W) -> std::fmt::Result {
        w.write_str(&self.0)
    }
}

impl std::fmt::Display for LanguageCode {
    /// Displays the interned language code text.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::ops::Deref for LanguageCode {
    type Target = str;

    /// Exposes the code as `&str` for generic string APIs.
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl AsRef<str> for LanguageCode {
    /// Borrows this code as `&str`.
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl From<String> for LanguageCode {
    /// Interns an owned string as a `LanguageCode`.
    fn from(value: String) -> Self {
        Self::new(value)
    }
}

impl From<&str> for LanguageCode {
    /// Interns a borrowed string as a `LanguageCode`.
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

impl std::borrow::Borrow<str> for LanguageCode {
    /// Supports hashmap/set lookups keyed by `str`.
    fn borrow(&self) -> &str {
        self.as_str()
    }
}

impl Validate for LanguageCode {
    /// Enforce basic CHAT-facing language-code constraints.
    fn validate(&self, _context: &ValidationContext, errors: &impl crate::ErrorSink) {
        let is_three_lowercase =
            self.0.len() == 3 && self.0.chars().all(|c| c.is_ascii_lowercase());

        // Language codes should be 3 lowercase letters (ISO 639-3 format)
        if self.0.len() != 3 {
            errors.report(
                ParseError::new(
                    ErrorCode::InvalidLanguageCode,
                    Severity::Error,
                    SourceLocation::at_offset(0),
                    ErrorContext::new(self.as_str(), 0..self.0.len(), "language_code"),
                    format!(
                        "Language code '{}' should be 3 characters (got {})",
                        self.0,
                        self.0.len()
                    ),
                )
                .with_suggestion("Use ISO 639-3 three-letter language codes (e.g., eng, spa, deu)"),
            );
        }

        // Check if all characters are lowercase letters
        if !self.0.chars().all(|c| c.is_ascii_lowercase()) {
            errors.report(
                ParseError::new(
                    ErrorCode::InvalidLanguageCode,
                    Severity::Error,
                    SourceLocation::at_offset(0),
                    ErrorContext::new(self.as_str(), 0..self.0.len(), "language_code"),
                    format!(
                        "Language code '{}' should be lowercase letters only",
                        self.0
                    ),
                )
                .with_suggestion("Use lowercase ISO 639-3 codes (e.g., eng not ENG)"),
            );
        }

        if is_three_lowercase && is_disallowed_placeholder_language_code(self.as_str()) {
            errors.report(
                ParseError::new(
                    ErrorCode::InvalidLanguageCode,
                    Severity::Error,
                    SourceLocation::at_offset(0),
                    ErrorContext::new(self.as_str(), 0..self.0.len(), "language_code"),
                    format!(
                        "Language code '{}' is not a recognized ISO 639-3 code",
                        self.0
                    ),
                )
                .with_suggestion(
                    "Use a valid ISO 639-3 code (e.g., eng, spa, deu) in @Languages and @ID",
                ),
            );
        }
    }
}

/// Rejects obvious placeholder values often used in synthetic examples.
///
/// This helper is intentionally conservative and only blocks values that are
/// almost certainly placeholders rather than real language codes.
fn is_disallowed_placeholder_language_code(code: &str) -> bool {
    matches!(code, "xyz" | "xxx" | "yyy" | "zzz")
}
