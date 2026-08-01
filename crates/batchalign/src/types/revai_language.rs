//! Rev.AI language capability data: which ISO 639-3 codes the engine supports,
//! and which it is known to handle badly.
//!
//! Lives under `types/` rather than in the `revai` client module, because it is
//! static data that submission VALIDATION consults before any API call is made,
//! and `types/request.rs` is where that validation lives. While it sat in
//! `revai/` the typed model depended on the HTTP client module for a lookup
//! table, which is one of the edges that kept everything downstream of `types`
//! out of the core crate.
//!
//! The client still uses it; the dependency simply runs the other way now.

use crate::api::LanguageCode3;

/// Language hint translated from batchalign's ISO-639-3 world into the code
/// expected by Rev.AI submissions.
///
/// Rev.AI accepts a mix of ISO 639-1 codes and special codes (e.g., `cmn` for
/// Mandarin). The mapping is **explicit and exhaustive**, unknown languages
/// produce a `None` result rather than silently submitting a wrong code.
///
/// # History
///
/// Earlier Python implementations used
/// `pycountry.languages.get(alpha_3=lang).alpha_2` for this conversion. The
/// Rust rewrite initially replaced it with a 13-entry hardcoded match and an
/// `&other[..2]` truncation fallback. That fallback was a regression bug: ISO
/// 639-3 first-two-characters do NOT reliably match ISO 639-1 codes (e.g.,
/// `pol` → `po` instead of `pl`, `hak` → `ha` which doesn't exist). Fixed
/// 2026-03-19 with a comprehensive mapping table covering all
/// Rev.AI-supported languages.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct RevAiLanguageHint(String);

impl RevAiLanguageHint {
    /// Borrow the Rev.AI language code.
    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }

    /// Rev.AI's own auto-detection, used when no language is resolved.
    ///
    /// A named constructor because the inner `String` is private: the client
    /// used to build this value inline, which only worked while the type and
    /// its caller shared a module.
    pub(crate) fn auto() -> Self {
        Self("auto".to_string())
    }
}

/// Try to convert an ISO 639-3 language code to a Rev.AI language hint.
///
/// Returns `None` if the language is not in Rev.AI's supported set. Callers
/// should report a clear diagnostic rather than submitting an unsupported code.
pub(crate) fn try_revai_language_hint(lang: &LanguageCode3) -> Option<RevAiLanguageHint> {
    // Comprehensive ISO 639-3 → Rev.AI code mapping.
    // Rev.AI supported codes (as of 2026-03): ar, af, sq, hy, az, eu, be, bn,
    // bs, bg, my, ca, cmn, hr, cs, da, nl, en, et, fi, fr, de, el, gl, ka,
    // gu, ht, he, hi, hu, is, id, it, ja, kn, kk, km, ko, lo, lv, lt, mk,
    // mg, ms, ml, mt, mr, ne, no, pa, fa, pl, pt, ro, ru, sr, si, sk, sl,
    // es, su, sw, sv, tl, tg, ta, te, th, tr, uk, ur, uz, vi, cy, yi, auto.
    let code = match lang.as_ref() {
        // Major languages (explicit Rev.AI codes)
        "eng" => "en",
        "spa" => "es",
        "fra" => "fr",
        "deu" => "de",
        "ita" => "it",
        "por" => "pt",
        "nld" => "nl",
        "jpn" => "ja",
        "kor" => "ko",
        "rus" => "ru",
        "ara" => "ar",
        "tur" => "tr",
        "zho" | "cmn" => "cmn",
        // European languages
        "pol" => "pl",
        "ces" => "cs",
        "ron" => "ro",
        "hun" => "hu",
        "bul" => "bg",
        "hrv" => "hr",
        "srp" => "sr",
        "slk" => "sk",
        "slv" => "sl",
        "ukr" => "uk",
        "lit" => "lt",
        "lav" => "lv",
        "est" => "et",
        "fin" => "fi",
        "dan" => "da",
        "nor" | "nob" | "nno" => "no",
        "swe" => "sv",
        "isl" => "is",
        "ell" => "el",
        "cat" => "ca",
        "glg" => "gl",
        "eus" => "eu",
        "cym" => "cy",
        "sqi" => "sq",
        "bel" => "be",
        "bos" => "bs",
        "mkd" => "mk",
        "mlt" => "mt",
        // South/Southeast Asian languages
        "hin" => "hi",
        "urd" => "ur",
        "ben" => "bn",
        "tam" => "ta",
        "tel" => "te",
        "kan" => "kn",
        "mal" => "ml",
        "mar" => "mr",
        "pan" => "pa",
        "nep" => "ne",
        "sin" => "si",
        "tha" => "th",
        "vie" => "vi",
        "ind" | "msa" => "id",
        "tgl" => "tl",
        "mya" => "my",
        "khm" => "km",
        "lao" => "lo",
        "sun" => "su",
        // Caucasian / Central Asian
        "kat" => "ka",
        "hye" => "hy",
        "aze" => "az",
        "kaz" => "kk",
        "uzb" => "uz",
        "tgk" => "tg",
        // Other
        "fas" => "fa",
        "heb" => "he",
        "yid" => "yi",
        "afr" => "af",
        "swa" => "sw",
        "hat" => "ht",
        "guj" => "gu",
        "mlg" => "mg",
        // Not supported by Rev.AI, return None
        _ => return None,
    };
    Some(RevAiLanguageHint(code.to_string()))
}

/// Entry in the Rev.AI known-broken `(engine, language)` deny-list.
///
/// Each entry records a language whose Rev.AI model we have observed to
/// produce output unusable for CHAT construction, cross-script tokens,
/// embedded replacement characters, or other CHAT-illegal content that the
/// downstream validator (`ChatWordText::try_from_lang`) refuses. Listing
/// the pair here causes `validate_language_support()` to reject the job
/// at preflight with an error that names a working alternative, instead of
/// letting the failure surface as confusing per-token validation errors.
///
/// The deny-list is Option A from
/// [`book/src/batchalign/reference/revai-language-quality-strategy.md`]. Each entry
/// carries a dated provenance comment so a successor reading this table
/// can see *why* it exists and *when* it should be re-evaluated against
/// Rev.AI's current model.
#[derive(Debug, Clone, Copy)]
pub(crate) struct RevAiKnownBroken {
    /// ISO 639-3 language code for which Rev.AI output is unusable.
    pub(crate) iso3: &'static str,
    /// One-line reason (appears in the operator-facing error message).
    pub(crate) reason: &'static str,
    /// Engine name string (matches `AsrEngineName::wire_name()`) that the
    /// error message should recommend as an alternative. Must be a name
    /// the user can pass to `--asr-engine`.
    pub(crate) recommended_engine: &'static str,
}

/// Rev.AI `(engine, language)` deny-list.
///
/// Keep entries alphabetized by `iso3` for readability. When adding an
/// entry, include a provenance comment with the incident date and a short
/// description of the observed failure. When Rev.AI's model for a listed
/// language is known to have improved (e.g. a changelog entry from Rev.AI
/// or a successful re-test), remove the entry in the same patch that
/// records the verification.
///
/// Escalation criteria (when the deny-list stops being enough and we
/// should build the runtime script-coherence gate or the empirical
/// capability probe) live in the strategy document.
pub(crate) const REVAI_KNOWN_BROKEN: &[RevAiKnownBroken] = &[
    // 2026-04-22: Rev.AI's Malayalam model (language=ml) returns tokens
    // in unrelated scripts (Hangul, Gurmukhi, Latin, Cyrillic) mixed
    // with U+FFFD replacement characters and bare punctuation. A
    // 1-minute test sample produced 55 tokens, effectively zero of
    // which were in Malayalam script. Submission-side mapping is
    // correct (mal → ml); this is a model-quality issue on Rev.AI's
    // side.
    //
    // The recommendation is ``whisper_hub`` rather than ``whisper``: a
    // follow-up empirical evaluation on the same sample showed stock
    // OpenAI Whisper (both medium and large-v3) also fails on Malayalam
    //: medium collapsed into Khmer/Gurmukhi character loops, large-v3
    // hallucinated "Thank you for watching." Only the community
    // fine-tune ``thennal/whisper-medium-ml`` (routed through the
    // ``whisper_hub`` engine) produced coherent Malayalam output. See
    // ``book/src/batchalign/reference/whisper-hub-asr.md`` for the comparison.
    RevAiKnownBroken {
        iso3: "mal",
        reason: "Malayalam ASR returns cross-script tokens (Hangul, Gurmukhi, Latin) and \
                 replacement characters; output cannot be represented as CHAT",
        recommended_engine: "whisper_hub",
    },
];

/// Look up a language in the Rev.AI deny-list.
///
/// Returns the deny-list entry when the given code matches a known-broken
/// language, or `None` when Rev.AI is not known to be broken for this
/// language. Callers in request validation use this to reject submissions
/// before making any Rev.AI API call.
pub(crate) fn revai_known_broken(lang: &LanguageCode3) -> Option<&'static RevAiKnownBroken> {
    REVAI_KNOWN_BROKEN
        .iter()
        .find(|entry| entry.iso3 == lang.as_ref())
}

/// Convert an ISO 639-3 language code to a Rev.AI language hint.
///
/// Falls back to `"auto"` (Rev.AI auto-detection) for unsupported languages
/// and logs a warning. This is preferable to failing silently or submitting
/// a wrong code.
impl From<&LanguageCode3> for RevAiLanguageHint {
    fn from(value: &LanguageCode3) -> Self {
        match try_revai_language_hint(value) {
            Some(hint) => hint,
            None => {
                tracing::warn!(
                    lang = %value,
                    "Language not in Rev.AI supported set; using auto-detection. \
                     ASR quality may be degraded. Add an explicit mapping in \
                     revai/preflight.rs if this language should be supported."
                );
                Self("auto".to_string())
            }
        }
    }
}
