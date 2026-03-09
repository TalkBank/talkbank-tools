//! Word parsing for CHAT main-tier tokens.
//!
//! This module handles category prefixes, lexical segments, CA markers,
//! form/language/POS suffix markers, and source-span tracking.
//!
//! # Related CHAT Manual Sections
//!
//! - <https://talkbank.org/0info/manuals/CHAT.html#Main_Tier>
//! - <https://talkbank.org/0info/manuals/CHAT.html#Words>
//! - <https://talkbank.org/0info/manuals/CHAT.html#CA_Option>
//! - <https://talkbank.org/0info/manuals/CHAT.html#CA_Delimiters>
//! - <https://talkbank.org/0info/manuals/CHAT.html#SecondLanguage_Marker_Single>

use chumsky::prelude::*;
use smallvec::SmallVec;
use std::collections::HashSet;
use std::sync::OnceLock;
use talkbank_model::ParseOutcome;
use talkbank_model::content::word::WordCompoundMarker;
use talkbank_model::generated::symbol_sets::{
    CA_DELIMITER_SYMBOLS, CA_ELEMENT_SYMBOLS, WORD_SEGMENT_FORBIDDEN_COMMON_SYMBOLS,
    WORD_SEGMENT_FORBIDDEN_REST_SYMBOLS, WORD_SEGMENT_FORBIDDEN_START_SYMBOLS,
};
use talkbank_model::model::{
    CADelimiter, CADelimiterType, CAElement, CAElementType, FormType, LanguageCode, OverlapIndex,
    OverlapPoint, OverlapPointKind, Word, WordCategory, WordContent, WordContents,
    WordLanguageMarker, WordLengthening, WordShortening, WordStressMarker, WordStressMarkerType,
    WordSyllablePause, WordText, WordUnderlineBegin, WordUnderlineEnd,
};
use talkbank_model::{
    ErrorCode, ErrorContext, ErrorSink, ParseError, Severity, SourceLocation, Span,
};

/// Parse a word using chumsky combinators.
///
/// This is the entry point that integrates with ErrorSink.
pub fn parse_word_impl(input: &str, offset: usize, errors: &impl ErrorSink) -> ParseOutcome<Word> {
    // Fix 2: Defense-in-depth fallback — compose the structured word parser with a
    // raw-text fallback that consumes the entire remaining input. This is safe here
    // because `input` is a pre-tokenized word string (already bounded by the
    // main-tier tokenizer). `word_parser_combinator` does NOT include this fallback
    // because it is used inside larger parsing chains where unbounded `any()` would
    // consume past word boundaries.
    let parser = word_parser(offset).or(any().repeated().at_least(1).to_slice().map_with(
        move |raw: &str, extra| {
            let span: SimpleSpan = extra.span();
            let source_span = Span::from_usize(span.start + offset, span.end + offset);
            tracing::debug!(word = raw, "Word parser fallback: preserving as raw text");
            Word::new_unchecked(raw, raw).with_span(source_span)
        },
    ));

    match parser.parse(input).into_result() {
        Ok(word) => ParseOutcome::parsed(word),
        Err(parse_errors) => {
            // Report chumsky errors to ErrorSink
            for err in parse_errors {
                // Extract span from error
                let span = err.span();
                let msg = format!("Parse error: {}", err.reason());
                errors.report(ParseError::new(
                    ErrorCode::UnparsableContent,
                    Severity::Error,
                    SourceLocation::new(Span::from_usize(span.start + offset, span.end + offset)),
                    ErrorContext::new(
                        input,
                        Span::from_usize(span.start + offset, span.end + offset),
                        input,
                    ),
                    msg,
                ));
            }
            ParseOutcome::rejected()
        }
    }
}

// ============================================================================
// Phase 1: Full Word Structure Parser (Position-Dependent Parsing)
// ============================================================================
//
// Word structure in CHAT (strict order):
//   [category_prefix] word_body [@form_type] [@s[:lang_codes]] [$pos_tag]
//
// Examples:
//   0die               - Omission category prefix
//   dog                - Simple word
//   dog@c              - Child-invented form
//   dog@s:eng          - Explicit language code
//   hao3@s:eng+zho+fra$n - 6 features: body + form + lang(3 codes) + POS
//

/// Main word parser - parses full word structure with position-dependent markers.
///
/// Structure: [category] word_body [@form] [@s[:codes]] [$pos]
///
/// Test case: `hao3@s:eng+zho+fra$n` (6 features)
fn word_parser<'a>(offset: usize) -> impl Parser<'a, &'a str, Word, extra::Err<Rich<'a, char>>> {
    // Parse in strict order
    let category = category_parser().or_not();
    let body = word_body_parser(offset); // Pass offset for span calculation
    let form = form_marker_parser().or_not();
    let lang = language_marker_parser().or_not();
    let pos = pos_tag_parser().or_not();

    category
        .then(body)
        .then(form)
        .then(lang)
        .then(pos)
        .map_with(move |((((cat, body), form), lang), pos), extra| {
            // CRITICAL: raw_text must be the FULL original input, nothing stripped.
            // Use extra.slice() to extract the complete matched text including all markers.
            let raw_text = extra.slice().to_string();

            // CRITICAL: Capture source location span for error reporting.
            // Convert chumsky's SimpleSpan to our Span type and apply offset.
            let span = extra.span();
            let source_span = Span::from_usize(span.start + offset, span.end + offset);

            // Compute cleaned_text from content (Text + Shortening variants only)
            let cleaned_text: String = body
                .content
                .iter()
                .filter_map(|c| match c {
                    WordContent::Text(t) => Some(t.as_ref()),
                    WordContent::Shortening(s) => Some(s.as_ref()),
                    _ => None,
                })
                .collect();

            // Build Word with all parsed features
            let mut word = Word::new_unchecked(raw_text, cleaned_text.clone())
                .with_content(body.content)
                .with_span(source_span);

            if let Some(c) = cat {
                word = word.with_category(c);
            }
            if let Some(f) = form {
                word = word.with_form_type(f);
            }
            if let Some(l) = lang {
                word.lang = Some(l);
            }
            if let Some(p) = pos {
                word = word.with_part_of_speech(p);
            }

            word
        })
}

/// Public parser combinator for word parsing, used by other parsers (e.g., main_tier.rs).
/// Returns the word parser that can be composed with other combinators.
pub(crate) fn word_parser_combinator<'a>(
    offset: usize,
) -> impl Parser<'a, &'a str, Word, extra::Err<Rich<'a, char>>> {
    word_parser(offset)
}

// ============================================================================
// Category Prefix Parser (0, &~, &-, &+)
// ============================================================================

/// Parse word category prefix.
///
/// CHAT categories:
/// - 0      - Omission
/// - &~     - Nonword/babbling
/// - &-     - Filler
/// - &+     - Phonological fragment
fn category_parser<'a>() -> impl Parser<'a, &'a str, WordCategory, extra::Err<Rich<'a, char>>> {
    choice((
        just("&+").to(WordCategory::PhonologicalFragment),
        just("&-").to(WordCategory::Filler),
        just("&~").to(WordCategory::Nonword),
        just("0").to(WordCategory::Omission),
    ))
    .labelled("category prefix")
}

// ============================================================================
// Form Marker Parser (ALL 22 @markers + @z:custom)
// ============================================================================

/// Parse form type marker (@a, @b, ..., @z:xxx).
///
/// MUST handle ALL form types from the start to avoid architectural rewrites.
fn form_marker_parser<'a>() -> impl Parser<'a, &'a str, FormType, extra::Err<Rich<'a, char>>> {
    just('@')
        .ignore_then(choice((
            // Multi-character markers MUST come before single-character ones
            just("fp").to(FormType::FP),
            just("ls").to(FormType::LS),
            just("sas").to(FormType::SAS),
            just("si").to(FormType::SI),
            just("sl").to(FormType::SL),
            just("wp").to(FormType::WP),
            just('z')
                .ignore_then(
                    just(':')
                        .or_not()
                        .ignore_then(none_of("@$ \t\n\r").repeated().at_least(1).to_slice()),
                )
                .map(|content: &str| FormType::UserDefined(content.to_string())),
            // Single-character markers
            just("a").to(FormType::A),
            just("b").to(FormType::B),
            just("c").to(FormType::C),
            just("d").to(FormType::D),
            just("f").to(FormType::F),
            just("g").to(FormType::G),
            just("i").to(FormType::I),
            just("k").to(FormType::K),
            just("l").to(FormType::L),
            just("n").to(FormType::N),
            just("o").to(FormType::O),
            just("p").to(FormType::P),
            just("q").to(FormType::Q),
            just("t").to(FormType::T),
            just("u").to(FormType::U),
            just("x").to(FormType::X),
        )))
        .labelled("form marker")
}

// ============================================================================
// Language Marker Parser (@s or @s:eng+zho&fra)
// ============================================================================

/// Parse language marker (@s or @s:code with & and + separators).
///
/// Examples:
/// - @s              - Shortcut (toggle between primary/secondary)
/// - @s:eng          - Single explicit code
/// - @s:eng+zho      - Multiple codes with +
/// - @s:eng&zho      - Ambiguous codes with &
/// - @s:eng+zho+fra  - Three mixed-language codes
fn language_marker_parser<'a>()
-> impl Parser<'a, &'a str, WordLanguageMarker, extra::Err<Rich<'a, char>>> {
    let code = none_of("@$ \t\n\r+&")
        .repeated()
        .at_least(1)
        .to_slice()
        .map(LanguageCode::new);

    let multiple = code
        .separated_by(just('+'))
        .at_least(2)
        .collect::<Vec<_>>()
        .map(WordLanguageMarker::Multiple);

    let ambiguous = code
        .separated_by(just('&'))
        .at_least(2)
        .collect::<Vec<_>>()
        .map(WordLanguageMarker::Ambiguous);

    let explicit = just("@s:").ignore_then(choice((
        multiple,
        ambiguous,
        code.map(WordLanguageMarker::Explicit),
    )));

    let immediate_marker = just("@s").to(WordLanguageMarker::Shortcut);
    let shortcut_marker = just(" @s").to(WordLanguageMarker::Shortcut);

    choice((explicit, immediate_marker, shortcut_marker)).labelled("language marker")
}

// ============================================================================
// POS Tag Parser ($n, $v, $adj, etc.)
// ============================================================================

/// Parse part-of-speech tag ($pos).
///
/// Examples: $n, $v, $adj, $det
fn pos_tag_parser<'a>() -> impl Parser<'a, &'a str, String, extra::Err<Rich<'a, char>>> {
    just('$')
        .ignore_then(none_of(" \t\n\r@").repeated().at_least(1).to_slice())
        .map(|s: &str| s.to_string())
        .labelled("POS tag")
}

// ============================================================================
// Phase 2: Word Body Parser with ALL 10 WordContent Types + Recursion
// ============================================================================

/// Intermediate word body result (before full Word construction).
struct WordBody {
    content: WordContents,
}

/// Parse word body with FULL recursion and ALL 10 WordContent types.
///
/// Word bodies can contain nested structures like:
/// - °softer(foo)° - CA delimiter wrapping shortening
/// - ↫som:↫somebod(y)↫body↫ - nested CA delimiters with lengthening and shortening
///
/// ALL 10 WordContent types:
/// 1. Text segments (base case)
/// 2. Shortenings (foo)
/// 3. Lengthening :
/// 4. SyllablePause ^
/// 5. StressMarker ˈ, ˌ
/// 6. CAElement ↑, ↓, ≠, etc.
/// 7. CADelimiter °, ∆, ∇, etc. (recursive!)
///    8-9. OverlapPoint ⌈⌉⌊⌋
///    10-11. UnderlineBegin/End \u{0002}\u{0001} and \u{0002}\u{0002}
///
/// offset: byte offset from file start for absolute span calculation
fn word_body_parser<'a>(
    offset: usize,
) -> impl Parser<'a, &'a str, WordBody, extra::Err<Rich<'a, char>>> {
    // Parse one or more word content elements
    let single_content = choice((
        // Compound marker + (NOT part of cleaned_text - metadata about word structure)
        just('+').map_with(move |_, extra| {
            let span: SimpleSpan = extra.span();
            let source_span = Span::from_usize(span.start + offset, span.end + offset);
            WordContent::CompoundMarker(WordCompoundMarker::new().with_span(source_span))
        }),
        // Separator ~ (tilde by itself is a text segment)
        // CRITICAL: Just parse the ~ character alone, don't consume following text
        // This matches tree-sitter behavior: foo~bar -> [Text("foo"), Text("~"), Text("bar")]
        just('~')
            .to_slice()
            .try_map(|text: &str, span: SimpleSpan| {
                WordText::new(text)
                    .map(WordContent::Text)
                    .ok_or_else(|| Rich::custom(span, "Empty tilde segment"))
            }),
        // 1. Shortenings (foo) - parenthesized text
        just('(')
            .ignore_then(any().filter(|&c| c != ')').repeated().to_slice())
            .then_ignore(just(')'))
            .try_map(|text: &str, span: SimpleSpan| {
                WordShortening::new(text)
                    .map(WordContent::Shortening)
                    .ok_or_else(|| Rich::custom(span, "Empty shortening"))
            }),
        // 2. Lengthening : (each colon is a separate lengthening marker)
        // Multiple colons (e.g., "a::n") become multiple Lengthening elements.
        // Lengthening is prosodic notation, excluded from cleaned_text.
        // IMPORTANT: We capture span for exact parser equivalence.
        just(':').map_with(move |_, extra| {
            let span: SimpleSpan = extra.span();
            let source_span = Span::from_usize(span.start + offset, span.end + offset);
            WordContent::Lengthening(WordLengthening::new().with_span(source_span))
        }),
        // 3. Syllable pause ^
        just('^').map_with(move |_, extra| {
            let span: SimpleSpan = extra.span();
            let source_span = Span::from_usize(span.start + offset, span.end + offset);
            WordContent::SyllablePause(WordSyllablePause::new().with_span(source_span))
        }),
        // 4. Stress markers (ˈ primary, ˌ secondary)
        choice((
            just('ˈ').to(WordStressMarkerType::Primary),
            just('ˌ').to(WordStressMarkerType::Secondary),
        ))
        .map_with(move |marker_type, extra| {
            let span: SimpleSpan = extra.span();
            let source_span = Span::from_usize(span.start + offset, span.end + offset);
            WordContent::StressMarker(WordStressMarker::new(marker_type).with_span(source_span))
        }),
        // 5. CA Elements (individual markers)
        choice((
            just("≠").to(CAElementType::BlockedSegments),
            just("∾").to(CAElementType::Constriction),
            just("⁑").to(CAElementType::Hardening),
            just("⤇").to(CAElementType::HurriedStart),
            just("∙").to(CAElementType::Inhalation),
            just("Ἡ").to(CAElementType::LaughInWord),
            just("↓").to(CAElementType::PitchDown),
            just("↻").to(CAElementType::PitchReset),
            just("↑").to(CAElementType::PitchUp),
            just("⤆").to(CAElementType::SuddenStop),
        ))
        .map_with(move |ca_type, extra| {
            let span: SimpleSpan = extra.span();
            let source_span = Span::from_usize(span.start + offset, span.end + offset);
            WordContent::CAElement(CAElement::new(ca_type).with_span(source_span))
        }),
        // 6. CA Delimiters (single markers, NOT paired - content is parsed separately)
        // Each delimiter character is parsed individually; the text between them
        // is handled by the text parser. E.g., °softer° becomes:
        //   [CADelimiter(Softer), Text("softer"), CADelimiter(Softer)]
        choice((
            just("∆").to(CADelimiterType::Faster),
            just("∇").to(CADelimiterType::Slower),
            just("°").to(CADelimiterType::Softer),
            just("▁").to(CADelimiterType::LowPitch),
            just("▔").to(CADelimiterType::HighPitch),
            just("☺").to(CADelimiterType::SmileVoice),
            just("♋").to(CADelimiterType::BreathyVoice),
            just("⁇").to(CADelimiterType::Unsure),
            just("∬").to(CADelimiterType::Whisper),
            just("Ϋ").to(CADelimiterType::Yawn),
            just("∮").to(CADelimiterType::Singing),
            just("↫").to(CADelimiterType::SegmentRepetition),
            just("⁎").to(CADelimiterType::Creaky),
            just("◉").to(CADelimiterType::Louder),
            just("§").to(CADelimiterType::Precise),
        ))
        .map_with(move |delimiter_type, extra| {
            let span: SimpleSpan = extra.span();
            let source_span = Span::from_usize(span.start + offset, span.end + offset);
            WordContent::CADelimiter(CADelimiter::new(delimiter_type).with_span(source_span))
        }),
        // 7-8. Overlap points ⌈⌉⌊⌋ with optional index
        choice((
            just("⌈")
                .ignore_then(
                    any()
                        .filter(|c: &char| c.is_ascii_digit())
                        .repeated()
                        .to_slice()
                        .or_not(),
                )
                .map(|idx: Option<&str>| {
                    let index = idx
                        .and_then(|s| s.parse::<u32>().ok())
                        .map(OverlapIndex::new);
                    OverlapPoint::new(OverlapPointKind::TopOverlapBegin, index)
                }),
            just("⌉")
                .ignore_then(
                    any()
                        .filter(|c: &char| c.is_ascii_digit())
                        .repeated()
                        .to_slice()
                        .or_not(),
                )
                .map(|idx: Option<&str>| {
                    let index = idx
                        .and_then(|s| s.parse::<u32>().ok())
                        .map(OverlapIndex::new);
                    OverlapPoint::new(OverlapPointKind::TopOverlapEnd, index)
                }),
            just("⌊")
                .ignore_then(
                    any()
                        .filter(|c: &char| c.is_ascii_digit())
                        .repeated()
                        .to_slice()
                        .or_not(),
                )
                .map(|idx: Option<&str>| {
                    let index = idx
                        .and_then(|s| s.parse::<u32>().ok())
                        .map(OverlapIndex::new);
                    OverlapPoint::new(OverlapPointKind::BottomOverlapBegin, index)
                }),
            just("⌋")
                .ignore_then(
                    any()
                        .filter(|c: &char| c.is_ascii_digit())
                        .repeated()
                        .to_slice()
                        .or_not(),
                )
                .map(|idx: Option<&str>| {
                    let index = idx
                        .and_then(|s| s.parse::<u32>().ok())
                        .map(OverlapIndex::new);
                    OverlapPoint::new(OverlapPointKind::BottomOverlapEnd, index)
                }),
        ))
        .map_with(move |point, extra| {
            let span: SimpleSpan = extra.span();
            let source_span = Span::from_usize(span.start + offset, span.end + offset);
            WordContent::OverlapPoint(point.with_span(source_span))
        }),
        // 9-10. Underline markers
        just("\u{0002}\u{0001}").map_with(move |_, extra| {
            let span: SimpleSpan = extra.span();
            let source_span = Span::from_usize(span.start + offset, span.end + offset);
            WordContent::UnderlineBegin(WordUnderlineBegin::new().with_span(source_span))
        }),
        just("\u{0002}\u{0002}").map_with(move |_, extra| {
            let span: SimpleSpan = extra.span();
            let source_span = Span::from_usize(span.start + offset, span.end + offset);
            WordContent::UnderlineEnd(WordUnderlineEnd::new().with_span(source_span))
        }),
        // 11. Digit continuation - digits 1-9 can appear after other content (e.g., "aa:3")
        // This handles the case where a digit follows a lengthening marker or other content
        // Digits 1-9 are FORBIDDEN_START but allowed as continuation after other elements
        any()
            .filter(|&c: &char| c.is_ascii_digit() && c != '0')
            .then(any().filter(|&c: &char| is_word_rest_char(c)).repeated())
            .to_slice()
            .try_map(|text: &str, span: SimpleSpan| {
                WordText::new(text)
                    .map(WordContent::Text)
                    .ok_or_else(|| Rich::custom(span, "Empty digit segment"))
            }),
        // 12. Text segments (anything not special) - MUST come last
        // Inline the text segment parsing to avoid Clone issues
        any()
            .filter(|&c: &char| is_word_start_char(c))
            .then(any().filter(|&c: &char| is_word_rest_char(c)).repeated())
            .to_slice()
            .try_map(|text: &str, span: SimpleSpan| {
                WordText::new(text)
                    .map(WordContent::Text)
                    .ok_or_else(|| Rich::custom(span, "Empty text segment"))
            }),
    ));

    // Parse one or more content elements
    single_content
        .repeated()
        .at_least(1)
        .collect::<Vec<WordContent>>()
        .try_map(|content: Vec<WordContent>, span: SimpleSpan| {
            // Build cleaned_text: lexical content only (what the speaker actually said).
            //
            // Included:  Text segments, Shortening (elided material restored)
            // Excluded:  Lengthening(:), SyllablePause(^), StressMarker(ˈˌ),
            //            CompoundMarker(+), OverlapPoint(⌈⌉⌊⌋), CAElement,
            //            CADelimiter, UnderlineBegin/End
            //
            // This drives untranscribed detection (xxx/yyy/www) and NLP — if
            // prosodic markers leak in, words like `xxx:` won't be recognised.
            let cleaned_text = content
                .iter()
                .filter_map(|c| match c {
                    WordContent::Text(t) => Some(t.as_ref()),
                    WordContent::Shortening(s) => Some(s.as_ref()),
                    _ => None, // Exclude all markers from cleaned_text
                })
                .collect::<String>();

            // Cleaned text must be non-empty (words must have phonetic content)
            if cleaned_text.is_empty() {
                return Err(Rich::custom(span, "Word has no text content"));
            }

            // Convert Vec to SmallVec
            let content_smallvec = SmallVec::from_vec(content);

            Ok(WordBody {
                content: WordContents::new(content_smallvec),
            })
        })
}

// ============================================================================
// Word Segment Parser (Base text without markers)
// ============================================================================

/// Build a `HashSet<char>` from string symbols that are exactly one char long.
fn single_char_symbol_set(symbols: &'static [&'static str]) -> HashSet<char> {
    symbols
        .iter()
        .filter_map(|symbol| {
            let mut chars = symbol.chars();
            match (chars.next(), chars.next()) {
                (Some(c), None) => Some(c),
                _ => None,
            }
        })
        .collect()
}

/// Cached set of symbols that cannot start a lexical word segment.
fn forbidden_start_symbols() -> &'static HashSet<char> {
    static SET: OnceLock<HashSet<char>> = OnceLock::new();
    SET.get_or_init(|| {
        let mut set = single_char_symbol_set(WORD_SEGMENT_FORBIDDEN_START_SYMBOLS);
        set.extend(single_char_symbol_set(CA_ELEMENT_SYMBOLS));
        set.extend(single_char_symbol_set(CA_DELIMITER_SYMBOLS));
        set
    })
}

/// Cached set of symbols that cannot appear after the first lexical char.
fn forbidden_rest_symbols() -> &'static HashSet<char> {
    static SET: OnceLock<HashSet<char>> = OnceLock::new();
    SET.get_or_init(|| {
        let mut set = single_char_symbol_set(WORD_SEGMENT_FORBIDDEN_REST_SYMBOLS);
        set.extend(single_char_symbol_set(CA_ELEMENT_SYMBOLS));
        set.extend(single_char_symbol_set(CA_DELIMITER_SYMBOLS));
        set
    })
}

/// Cached set of symbols disallowed in any lexical position.
fn forbidden_common_symbols() -> &'static HashSet<char> {
    static SET: OnceLock<HashSet<char>> = OnceLock::new();
    SET.get_or_init(|| single_char_symbol_set(WORD_SEGMENT_FORBIDDEN_COMMON_SYMBOLS))
}

/// Cached set of CA element marker symbols.
fn ca_element_symbols() -> &'static HashSet<char> {
    static SET: OnceLock<HashSet<char>> = OnceLock::new();
    SET.get_or_init(|| single_char_symbol_set(CA_ELEMENT_SYMBOLS))
}

/// Cached set of CA delimiter marker symbols.
fn ca_delimiter_symbols() -> &'static HashSet<char> {
    static SET: OnceLock<HashSet<char>> = OnceLock::new();
    SET.get_or_init(|| single_char_symbol_set(CA_DELIMITER_SYMBOLS))
}

/// Returns whether ca marker char.
pub(crate) fn is_ca_marker_char(c: char) -> bool {
    ca_element_symbols().contains(&c) || ca_delimiter_symbols().contains(&c)
}

/// Check if a character is allowed at the start of a word segment.
pub(super) fn is_word_start_char(c: char) -> bool {
    // Keep tree-sitter behavior: only 0 may start a segment; 1-9 are disallowed.
    if c.is_ascii_digit() && c != '0' {
        return false;
    }

    !forbidden_start_symbols().contains(&c) && !forbidden_common_symbols().contains(&c)
}

/// Check if a character is allowed in the rest of a word segment.
fn is_word_rest_char(c: char) -> bool {
    !forbidden_rest_symbols().contains(&c) && !forbidden_common_symbols().contains(&c)
}

// Note: word_segment_parser was inlined into word_body_parser for better control flow

#[cfg(test)]
mod tests {
    use super::parse_word_impl;
    use talkbank_model::ErrorCollector;
    use talkbank_model::model::{
        CADelimiterType, FormType, WordCategory, WordContent, WordLanguageMarker,
    };

    /// Tests parses low pitch delimiters as ca delimiters.
    #[test]
    fn parses_low_pitch_delimiters_as_ca_delimiters() -> Result<(), String> {
        let errors = ErrorCollector::new();
        let word = parse_word_impl("▁lower▁", 0, &errors)
            .ok_or_else(|| "word should parse".to_string())?;
        assert_eq!(word.cleaned_text(), "lower");
        assert!(matches!(
            word.content.first(),
            Some(WordContent::CADelimiter(delim))
                if delim.delimiter_type == CADelimiterType::LowPitch
        ));
        assert!(matches!(
            word.content.last(),
            Some(WordContent::CADelimiter(delim))
                if delim.delimiter_type == CADelimiterType::LowPitch
        ));
        Ok(())
    }

    /// Tests parses language marker variants.
    #[test]
    fn parses_language_marker_variants() -> Result<(), String> {
        let errors = ErrorCollector::new();
        let multiple = parse_word_impl("hao3@s:eng+zho+fra", 0, &errors)
            .ok_or_else(|| "word should parse".to_string())?;
        let ambiguous = parse_word_impl("hao3@s:eng&zho&fra", 0, &errors)
            .ok_or_else(|| "word should parse".to_string())?;

        assert!(matches!(
            multiple.lang,
            Some(WordLanguageMarker::Multiple(ref codes)) if codes.len() == 3
        ));
        assert!(matches!(
            ambiguous.lang,
            Some(WordLanguageMarker::Ambiguous(ref codes)) if codes.len() == 3
        ));
        Ok(())
    }

    #[test]
    fn parses_user_form_with_label() -> Result<(), String> {
        let errors = ErrorCollector::new();
        let word = parse_word_impl("is@z:foo", 0, &errors)
            .ok_or_else(|| "word should parse".to_string())?;
        assert!(matches!(word.form_type, Some(FormType::UserDefined(ref value)) if value == "foo"));
        Ok(())
    }

    // =========================================================================
    // Basic word parsing (mutant-targeted)
    // =========================================================================

    #[test]
    fn parses_simple_word() {
        let errors = ErrorCollector::new();
        let word = parse_word_impl("hello", 0, &errors);
        assert!(word.is_parsed());
        let w = word.ok_or("should parse").unwrap();
        assert_eq!(w.cleaned_text(), "hello");
        assert!(errors.is_empty());
    }

    #[test]
    fn parses_word_with_nonzero_offset() {
        let errors = ErrorCollector::new();
        let word = parse_word_impl("hello", 50, &errors);
        assert!(word.is_parsed());
        let w = word.ok_or("should parse").unwrap();
        // Span should be offset-adjusted
        assert!(w.span.start >= 50);
        assert!(w.span.end >= 55);
    }

    #[test]
    fn parses_category_prefix_omission() {
        let errors = ErrorCollector::new();
        let word = parse_word_impl("0die", 0, &errors)
            .ok_or("should parse")
            .unwrap();
        assert!(matches!(word.category, Some(WordCategory::Omission)));
        assert_eq!(word.cleaned_text(), "die");
    }

    #[test]
    fn parses_single_language_marker() {
        let errors = ErrorCollector::new();
        let word = parse_word_impl("bonjour@s:fra", 0, &errors)
            .ok_or("should parse")
            .unwrap();
        assert!(matches!(
            word.lang,
            Some(WordLanguageMarker::Explicit(ref code)) if code.as_str() == "fra"
        ));
    }

    #[test]
    fn parses_form_type_child_invented() {
        let errors = ErrorCollector::new();
        let word = parse_word_impl("doggie@c", 0, &errors)
            .ok_or("should parse")
            .unwrap();
        assert!(matches!(word.form_type, Some(FormType::C)));
    }

    #[test]
    fn parses_form_type_babbling() {
        let errors = ErrorCollector::new();
        let word = parse_word_impl("bababa@b", 0, &errors)
            .ok_or("should parse")
            .unwrap();
        assert!(matches!(word.form_type, Some(FormType::B)));
    }

    #[test]
    fn parses_form_type_onomatopoeia() {
        let errors = ErrorCollector::new();
        let word = parse_word_impl("woof@o", 0, &errors)
            .ok_or("should parse")
            .unwrap();
        assert!(matches!(word.form_type, Some(FormType::O)));
    }

    // =========================================================================
    // Character classification (mutant-targeted)
    // =========================================================================

    #[test]
    fn word_start_char_rejects_nonzero_digits() {
        assert!(!super::is_word_start_char('1'));
        assert!(!super::is_word_start_char('9'));
    }

    #[test]
    fn word_start_char_allows_zero() {
        assert!(super::is_word_start_char('0'));
    }

    #[test]
    fn word_start_char_allows_letters() {
        assert!(super::is_word_start_char('a'));
        assert!(super::is_word_start_char('Z'));
    }

    #[test]
    fn ca_marker_char_includes_elements_and_delimiters() {
        // These are CA notation characters
        assert!(super::is_ca_marker_char('↑')); // pitch up (element)
        assert!(super::is_ca_marker_char('↓')); // pitch down (element)
        assert!(super::is_ca_marker_char('▁')); // low pitch (delimiter)
    }

    #[test]
    fn ca_marker_char_rejects_normal_chars() {
        assert!(!super::is_ca_marker_char('a'));
        assert!(!super::is_ca_marker_char('1'));
        assert!(!super::is_ca_marker_char(' '));
    }

    // =========================================================================
    // Compound and special words
    // =========================================================================

    #[test]
    fn parses_compound_word() {
        let errors = ErrorCollector::new();
        let word = parse_word_impl("ice+cream", 0, &errors)
            .ok_or("should parse")
            .unwrap();
        // cleaned_text strips the compound marker; verify it parsed as compound
        assert!(
            word.content
                .iter()
                .any(|c| matches!(c, WordContent::CompoundMarker(_)))
        );
    }

    #[test]
    fn parses_word_with_shortening() {
        let errors = ErrorCollector::new();
        let word = parse_word_impl("(be)cause", 0, &errors)
            .ok_or("should parse")
            .unwrap();
        assert!(
            word.content
                .iter()
                .any(|c| matches!(c, WordContent::Shortening(_)))
        );
    }

    #[test]
    fn empty_input_is_rejected() {
        let errors = ErrorCollector::new();
        let word = parse_word_impl("", 0, &errors);
        assert!(word.is_rejected());
    }
}
