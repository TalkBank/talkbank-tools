//! Provenance tracking for strings coming from Python/Stanza using phantom types.
//!
//! This module helps avoid "primitive obsession" by tagging types
//! with their origin and intended use.
//!
//! References:
//! - <https://talkbank.org/0info/manuals/CHAT.html#File_Format>
//! - <https://talkbank.org/0info/manuals/CHAT.html#Main_Tier>
//! - <https://talkbank.org/0info/manuals/CHAT.html#Dependent_Tiers>
//! - <https://talkbank.org/0info/manuals/CHAT.html#Morphological_Tier>

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::marker::PhantomData;

/// A generic wrapper for data with associated provenance or intent markers.
///
/// Uses `PhantomData` to distinguish between different types of data
/// that share the same underlying representation (e.g., String).
///
/// Reference: <https://talkbank.org/0info/manuals/CHAT.html#File_Format>
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(transparent)]
pub struct Provenance<M, T = String> {
    /// The wrapped data value.
    pub data: T,
    #[serde(skip)]
    #[schemars(skip)]
    _marker: PhantomData<M>,
}

impl<M, T> Provenance<M, T> {
    /// Wrap a value with provenance metadata.
    pub fn new(data: T) -> Self {
        Self {
            data,
            _marker: PhantomData,
        }
    }
}

impl<M, T: AsRef<str>> Provenance<M, T> {
    /// Returns the data as a string slice.
    pub fn as_str(&self) -> &str {
        self.data.as_ref()
    }
}

impl<M, T: fmt::Display> fmt::Display for Provenance<M, T> {
    /// Formats only the wrapped payload value.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.data)
    }
}

// ---------------------------------------------------------------------------
// Marker Types (Zero-Sized Types)
// ---------------------------------------------------------------------------

// --- Origins ---

/// Data originating from Python integration layers.
///
/// Reference: <https://talkbank.org/0info/manuals/CHAT.html#File_Format>
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FromPython;

/// Data produced by an NLP tool (e.g., Stanza).
///
/// Reference: <https://talkbank.org/0info/manuals/CHAT.html#Morphological_Tier>
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NlpProduced;

/// Data extracted directly from CHAT transcript text.
///
/// Reference: <https://talkbank.org/0info/manuals/CHAT.html#Main_Tier>
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChatOriginal;

// --- Intents / Roles ---

/// Raw CHAT text intended for parsing.
///
/// Reference: <https://talkbank.org/0info/manuals/CHAT.html#File_Format>
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RawChatText;

/// JSON representing a transcript build request.
///
/// Reference: <https://talkbank.org/0info/manuals/CHAT.html#File_Format>
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TranscriptJson;

/// JSON representing ASR words.
///
/// Reference: <https://talkbank.org/0info/manuals/CHAT.html#Utterance_Media>
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AsrWordsJson;

/// A language identifier (e.g., "eng").
///
/// Reference: <https://talkbank.org/0info/manuals/CHAT.html#Language_Codes>
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LanguageId;

/// An alignment domain name (e.g., "mor").
///
/// Reference: <https://talkbank.org/0info/manuals/CHAT.html#Dependent_Tiers>
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AlignmentDomainMarker;

/// Words that have been tokenized by an NLP tool.
///
/// Reference: <https://talkbank.org/0info/manuals/CHAT.html#Word_Tier>
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TokenizedWords;

/// Morphosyntax produced by an NLP tool.
///
/// Reference: <https://talkbank.org/0info/manuals/CHAT.html#Morphological_Tier>
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Morphosyntax;

/// JSON received as a response from an NLP callback.
///
/// Reference: <https://talkbank.org/0info/manuals/CHAT.html#Morphological_Tier>
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NlpResponseJson;

// ---------------------------------------------------------------------------
// Convenience Aliases
// ---------------------------------------------------------------------------

/// Raw CHAT text coming from Python.
pub type PythonChatText = Provenance<RawChatText, String>;

/// Transcript JSON coming from Python.
pub type PythonTranscriptJson = Provenance<TranscriptJson, String>;

/// ASR words JSON coming from Python.
pub type PythonAsrWordsJson = Provenance<AsrWordsJson, String>;

/// A language identifier passed from Python.
pub type PythonLanguageId = Provenance<LanguageId, String>;

/// An alignment domain passed from Python.
pub type PythonAlignmentDomain = Provenance<AlignmentDomainMarker, String>;

/// Tokenized words produced by NLP.
pub type NlpTokens = Provenance<TokenizedWords, Vec<String>>;

/// Morphosyntax JSON response from NLP.
pub type NlpResponse = Provenance<NlpResponseJson, String>;

// ---------------------------------------------------------------------------
// PyO3 Conversion Traits
// ---------------------------------------------------------------------------

#[cfg(feature = "python")]
impl<'a, 'py, M, T> pyo3::FromPyObject<'a, 'py> for Provenance<M, T>
where
    T: pyo3::FromPyObject<'a, 'py>,
{
    type Error = T::Error;

    /// Extracts the wrapped payload from Python and tags it with provenance.
    fn extract(ob: pyo3::Borrowed<'a, 'py, pyo3::PyAny>) -> Result<Self, Self::Error> {
        let data = T::extract(ob)?;
        Ok(Provenance::new(data))
    }
}
