//! Tests for word-language resolution behavior.
//!
//! The cases cover fallback semantics, shortcut edge conditions, and explicit
//! marker behavior so language-resolution policy stays stable over refactors.

use super::{LanguageResolution, resolve_word_language};
use crate::model::{LanguageCode, ValidationTag, ValidationTagged, Word, WordLanguageMarker};

/// Builds `LanguageCode` values for test fixtures.
fn codes(list: &[&str]) -> Vec<LanguageCode> {
    list.iter().map(|code| LanguageCode::new(*code)).collect()
}

/// Short helper for constructing one `LanguageCode`.
fn lc(code: &str) -> LanguageCode {
    LanguageCode::new(code)
}

/// Words without markers inherit the default/tier language.
///
/// This is the baseline resolution path when no explicit word-level override is present.
#[test]
fn test_resolve_word_language_default() {
    let word1 = Word::new_unchecked("ni3", "ni3");
    let word2 = Word::new_unchecked("hao3", "hao3");

    let declared_languages = codes(&["zho", "eng"]);
    let tier_language = declared_languages.first();

    let (lang1, errors1) = resolve_word_language(&word1, tier_language, &declared_languages);
    assert_eq!(lang1, LanguageResolution::Single(lc("zho")));
    assert_eq!(errors1.len(), 0);

    let (lang2, errors2) = resolve_word_language(&word2, tier_language, &declared_languages);
    assert_eq!(lang2, LanguageResolution::Single(lc("zho")));
    assert_eq!(errors2.len(), 0);
}

/// Tier-scoped language overrides apply uniformly to unmarked words.
///
/// Both words should resolve to the tier language with no diagnostics.
#[test]
fn test_resolve_word_language_tier_scoped() {
    let word1 = Word::new_unchecked("ni3", "ni3");
    let word2 = Word::new_unchecked("hao3", "hao3");

    let declared_languages = codes(&["zho", "eng"]);
    let tier_language = declared_languages.get(1);

    let (lang1, errors1) = resolve_word_language(&word1, tier_language, &declared_languages);
    assert_eq!(lang1, LanguageResolution::Single(lc("eng")));
    assert_eq!(errors1.len(), 0);

    let (lang2, errors2) = resolve_word_language(&word2, tier_language, &declared_languages);
    assert_eq!(lang2, LanguageResolution::Single(lc("eng")));
    assert_eq!(errors2.len(), 0);
}

/// `@s` shortcut resolves to the secondary declared language.
///
/// This test covers the common two-language code-switching shorthand behavior.
#[test]
fn test_resolve_word_language_word_shortcut() {
    let mut word1 = Word::new_unchecked("ni3@s", "ni3");
    word1.lang = Some(WordLanguageMarker::Shortcut);

    let mut word2 = Word::new_unchecked("hao3@s", "hao3");
    word2.lang = Some(WordLanguageMarker::Shortcut);

    let declared_languages = codes(&["zho", "eng"]);
    let tier_language = declared_languages.get(1);

    let (lang1, errors1) = resolve_word_language(&word1, tier_language, &declared_languages);
    assert_eq!(lang1, LanguageResolution::Single(lc("zho")));
    assert_eq!(errors1.len(), 0);

    let (lang2, errors2) = resolve_word_language(&word2, tier_language, &declared_languages);
    assert_eq!(lang2, LanguageResolution::Single(lc("zho")));
    assert_eq!(errors2.len(), 0);
}

/// Explicit `@s:code` markers override tier/default language.
///
/// Resolution should return the explicit code directly with no warnings.
#[test]
fn test_resolve_word_language_word_explicit() {
    let mut word1 = Word::new_unchecked("ni3@s:zho", "ni3");
    word1.lang = Some(WordLanguageMarker::explicit("zho"));

    let mut word2 = Word::new_unchecked("hao3@s:zho", "hao3");
    word2.lang = Some(WordLanguageMarker::explicit("zho"));

    let declared_languages = codes(&["zho", "eng"]);
    let tier_language = declared_languages.get(1);

    let (lang1, errors1) = resolve_word_language(&word1, tier_language, &declared_languages);
    assert_eq!(lang1, LanguageResolution::Single(lc("zho")));
    assert_eq!(errors1.len(), 0);

    let (lang2, errors2) = resolve_word_language(&word2, tier_language, &declared_languages);
    assert_eq!(lang2, LanguageResolution::Single(lc("zho")));
    assert_eq!(errors2.len(), 0);
}

/// Shortcut `@s` without a secondary language emits `E249`.
///
/// Current behavior falls back to tier language while reporting the missing-secondary issue.
#[test]
fn test_resolve_word_language_no_secondary() {
    let mut word = Word::new_unchecked("ni3@s", "ni3");
    word.lang = Some(WordLanguageMarker::Shortcut);

    let declared_languages = codes(&["zho"]);
    let tier_language = declared_languages.first();

    let (lang, errors) = resolve_word_language(&word, tier_language, &declared_languages);
    // Returns tier language as fallback, but reports error
    assert_eq!(lang, LanguageResolution::Single(lc("zho")));
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].code.as_str(), "E249");
}

/// Shortcut `@s` in 3+ language context emits `E248`.
///
/// Ambiguous secondary selection falls back to tier language with an error.
#[test]
fn test_resolve_word_language_tertiary() {
    let mut word = Word::new_unchecked("word@s", "word");
    word.lang = Some(WordLanguageMarker::Shortcut);

    let declared_languages = codes(&["zho", "eng", "spa"]);
    let tier_language = declared_languages.get(2);

    let (lang, errors) = resolve_word_language(&word, tier_language, &declared_languages);
    // Returns tier language as fallback, but reports error
    assert_eq!(lang, LanguageResolution::Single(lc("spa")));
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].code.as_str(), "E248");
}

/// Explicit language codes are allowed even if absent from `@Languages`.
///
/// This preserves CHAT behavior where `@s:code` can introduce new language codes.
#[test]
fn test_resolve_word_language_undeclared_explicit_code() {
    // @s:LANGCODE does NOT require the code to be in @Languages.
    // Any language can be introduced at any time.
    let mut word = Word::new_unchecked("ciao@s:ita", "ciao");
    word.lang = Some(WordLanguageMarker::explicit("ita"));

    let declared_languages = codes(&["zho", "eng"]);
    let tier_language = declared_languages.first();

    let (lang, errors) = resolve_word_language(&word, tier_language, &declared_languages);
    assert_eq!(lang, LanguageResolution::Single(lc("ita")));
    assert_eq!(errors.len(), 0);
}

/// Explicit language codes declared in `@Languages` resolve cleanly.
///
/// No diagnostics should be produced in this straightforward case.
#[test]
fn test_resolve_word_language_declared_explicit_code() {
    // Valid case: explicit code that IS in @Languages header
    let mut word = Word::new_unchecked("hello@s:eng", "hello");
    word.lang = Some(WordLanguageMarker::explicit("eng"));

    let declared_languages = codes(&["zho", "eng"]);
    let tier_language = declared_languages.first();

    let (lang, errors) = resolve_word_language(&word, tier_language, &declared_languages);
    assert_eq!(lang, LanguageResolution::Single(lc("eng")));
    assert_eq!(errors.len(), 0); // No error - code is declared
}

/// Explicit markers still resolve when `@Languages` is missing.
///
/// File-level validation handles missing declarations; resolver accepts explicit word codes.
#[test]
fn test_resolve_word_language_explicit_with_no_declared_languages() {
    // Edge case: No @Languages header at all
    // (file-level validation should catch missing @Languages)
    let mut word = Word::new_unchecked("hello@s:eng", "hello");
    word.lang = Some(WordLanguageMarker::explicit("eng"));

    let declared_languages: Vec<LanguageCode> = vec![];
    let tier_language: Option<&LanguageCode> = None;

    let (lang, errors) = resolve_word_language(&word, tier_language, &declared_languages);
    assert_eq!(lang, LanguageResolution::Single(lc("eng")));
    assert_eq!(errors.len(), 0);
}

/// Shortcut `@s` without any language context resolves as unresolved.
///
/// The resolver should emit `E249` because no secondary language can be inferred.
#[test]
fn test_resolve_word_language_shortcut_without_context_is_unresolved() {
    let mut word = Word::new_unchecked("word@s", "word");
    word.lang = Some(WordLanguageMarker::Shortcut);

    let declared_languages: Vec<LanguageCode> = vec![];
    let tier_language: Option<&LanguageCode> = None;

    let (lang, errors) = resolve_word_language(&word, tier_language, &declared_languages);
    assert_eq!(lang, LanguageResolution::Unresolved);
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].code.as_str(), "E249");
}

/// Unmarked words with no language context resolve as unresolved.
///
/// Unlike shortcut failures, this path currently emits no additional diagnostics.
#[test]
fn test_resolve_word_language_no_marker_without_context_is_unresolved() {
    let word = Word::new_unchecked("word", "word");
    let declared_languages: Vec<LanguageCode> = vec![];
    let tier_language: Option<&LanguageCode> = None;

    let (lang, errors) = resolve_word_language(&word, tier_language, &declared_languages);
    assert_eq!(lang, LanguageResolution::Unresolved);
    assert_eq!(errors.len(), 0);
}

/// Language-resolution variants map to expected validation tags.
///
/// Resolved variants are clean, while unresolved state is treated as an error.
#[test]
fn test_language_resolution_validation_tags() {
    assert_eq!(
        LanguageResolution::Single(lc("eng")).validation_tag(),
        ValidationTag::Clean
    );
    assert_eq!(
        LanguageResolution::Multiple(codes(&["eng", "spa"])).validation_tag(),
        ValidationTag::Clean
    );
    assert_eq!(
        LanguageResolution::Ambiguous(codes(&["eng", "spa"])).validation_tag(),
        ValidationTag::Clean
    );
    assert_eq!(
        LanguageResolution::Unresolved.validation_tag(),
        ValidationTag::Error
    );
}
