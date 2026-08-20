//! English capitalization helpers for ASR post-processing.
//!
//! THIS PIPELINE'S POLICY, not a CHAT-format fact. These lived in chatter's
//! `talkbank-transform::capitalize` until 2026-08-19 and moved out because
//! chatter is the CHAT-format authority and English orthography is a
//! convention of one language: chatter never called any of it itself.
//!
//! Only the two helpers this crate actually uses came across. The whole-file
//! transform did not: it walks a typed `ChatFile`, and this pipeline
//! capitalizes its own `AsrWord` representation before a `ChatFile` exists.
//! The other former user, the MICASE converter, took that transform instead.
//!
//! `cleanup.rs` already declined the third function chatter offered,
//! `is_capitalizable_initial`, and documented why: it answers "could this token
//! be capitalized" where this pipeline needs "does this token own the
//! utterance-initial slot", and the two diverge on every surface starting with
//! a non-letter. That divergence is the reason the module is split rather than
//! shared, and `owns_initial_capitalization_slot` there remains the local
//! answer.

/// Lowercased pronoun-"I" surfaces and their capitalized forms.
const I_CAP_REWRITES: &[(&str, &str)] = &[
    ("i", "I"),
    ("i'll", "I'll"),
    ("i'm", "I'm"),
    ("i've", "I've"),
    ("i'd", "I'd"),
];

/// If `word` (case-insensitively) is a pronoun-"I" surface (`i`, `i'm`,
/// `i'll`, `i've`, `i'd`), return its capitalized form.
#[must_use]
pub fn capitalized_pronoun_i(word: &str) -> Option<&'static str> {
    // No `to_lowercase()`: that allocated a `String` for every word of every
    // utterance to compare against five ASCII literals. The table is pure
    // ASCII, and the only characters whose lowercase is ASCII `i` are `i` and
    // `I`, so an ASCII-case-insensitive compare gives identical verdicts on any
    // input. That equivalence depends on the table STAYING ASCII.
    I_CAP_REWRITES
        .iter()
        .find(|(src, _)| src.eq_ignore_ascii_case(word))
        .map(|(_, dst)| *dst)
}

/// Uppercase the first character of `text` if it is lowercase; otherwise
/// return it unchanged.
///
/// # This keeps the KNOWN DEFECT `cleanup.rs` documents
///
/// It uppercases the first CHARACTER, so an apostrophe-initial surface gets no
/// capital at all (`'twas` stays `'twas`). That behaviour is preserved
/// deliberately in this move, which is a relocation and not a fix: changing it
/// here would alter ASR output in the same commit that moves the code, and the
/// two should be separable. The MICASE converter's copy DID fix it, because
/// that fix was part of its own defect work; see
/// `owns_initial_capitalization_slot` in `cleanup.rs` for this pipeline's
/// standing note on the same issue.
#[must_use]
pub fn capitalize_first(text: &str) -> String {
    let mut chars = text.chars();
    match chars.next() {
        Some(first) if first.is_lowercase() => {
            first.to_uppercase().collect::<String>() + chars.as_str()
        }
        _ => text.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pronoun_i_forms_capitalize_case_insensitively() {
        assert_eq!(capitalized_pronoun_i("i"), Some("I"));
        assert_eq!(capitalized_pronoun_i("i'm"), Some("I'm"));
        assert_eq!(capitalized_pronoun_i("I'M"), Some("I'm"));
        assert_eq!(capitalized_pronoun_i("in"), None);
    }

    #[test]
    fn capitalize_first_is_idempotent_and_does_not_yet_capitalize_behind_an_apostrophe() {
        assert_eq!(capitalize_first("dog"), "Dog");
        assert_eq!(capitalize_first("Dog"), "Dog");
        // The documented defect, pinned so a change to it is deliberate.
        assert_eq!(capitalize_first("'twas"), "'twas");
    }
}
