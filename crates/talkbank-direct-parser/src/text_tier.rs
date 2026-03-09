//! Simple text-based tier parsing using pure Chumsky combinators.
//!
//! Text tiers (%com, %exp, %add, %gpx, %int, %spa, %sit) contain free-form text content.
//!
//! # Related CHAT Manual Sections
//!
//! - <https://talkbank.org/0info/manuals/CHAT.html#Dependent_Tiers>
//! - <https://talkbank.org/0info/manuals/CHAT.html#Working_with_Media>
//! - <https://talkbank.org/0info/manuals/CHAT.html#GesturePosition_Tier>
//!
//! # API Contract: Content-Only Input Expected
//!
//! The ChatParser trait methods expect **content-only input** (without the %tier:\t prefix).
//!
//! **Example - CORRECT:**
//! - Input: `"after transcribing, I realized\n\tthis means something"`
//! - parse_com_tier expects just the text, no `%com:\t` prefix
//!
//! **Example - INCORRECT:**
//! - Input: `"%com:\tafter transcribing..."`  ❌ Don't include the prefix!
//!
//! **Why:** TreeSitterParser's wrapper_parse_tier() function adds the prefix internally.
//! If input already had the prefix, the wrapper would create invalid CHAT format.
//! This convention ensures both DirectParser and TreeSitterParser work identically.
//!
//! See: `../API_PREFIX_CONVENTIONS.md` for complete documentation
//!
//! # Bullet Parsing
//!
//! Text tiers can contain inline bullets in the format `START_END` (e.g., `2061689_2062652`).
//! These bullets must be parsed as separate segments to match tree-sitter behavior.

use chumsky::prelude::*;
use talkbank_model::ErrorSink;
use talkbank_model::ParseOutcome;
use talkbank_model::dependent_tier::{BulletContent, BulletContentSegment};
use talkbank_model::model::{
    ActTier, AddTier, CodTier, ComTier, ExpTier, GpxTier, IntTier, SitTier, SpaTier,
};

/// Parse text tier content using pure chumsky combinators.
///
/// **IMPORTANT**: Text tiers can contain inline bullets in NAK-delimited format:
/// `\u{15}START_END\u{15}` (e.g., `\u{15}2061689_2062652\u{15}`)
///
/// These must be parsed as separate bullet segments to match TreeSitterParser behavior.
fn text_tier_content_parser<'a>()
-> impl Parser<'a, &'a str, Vec<BulletContentSegment>, extra::Err<Rich<'a, char>>> {
    // Picture: \u{15}%pic:"filename"\u{15}
    let picture = just('\u{15}')
        .ignore_then(just('%'))
        .ignore_then(just('p'))
        .ignore_then(just('i'))
        .ignore_then(just('c'))
        .ignore_then(just(':'))
        .ignore_then(just('"'))
        .ignore_then(none_of(['"', '\u{15}']).repeated().collect::<String>())
        .then_ignore(just('"'))
        .then_ignore(just('\u{15}'))
        .map(|filename: String| BulletContentSegment::picture(filename));

    // Bullet: \u{15}START_END\u{15}
    let digits_to_u64 = one_of("0123456789")
        .repeated()
        .at_least(1)
        .collect::<Vec<char>>()
        .map(|digits| {
            digits
                .into_iter()
                .fold(0u64, |acc, digit| acc * 10 + u64::from(digit as u8 - b'0'))
        });

    let bullet = just('\u{15}')
        .ignore_then(digits_to_u64)
        .then_ignore(just('_'))
        .then(digits_to_u64)
        .then_ignore(just('\u{15}'))
        .map(|(start_ms, end_ms): (u64, u64)| BulletContentSegment::bullet(start_ms, end_ms));

    // Continuation marker: exactly \n\t
    let continuation = just("\n\t").to(BulletContentSegment::continuation());

    // Text content: everything that is not a bullet, continuation, or NAK
    // We need to stop at \n (for continuation check) and \u{15} (for bullet check)
    let text_content = {
        let non_special = none_of("\n\u{15}");
        let newline_not_tab = just('\n').then(none_of("\t")).map(|(n, c): (char, char)| {
            let mut s = String::new();
            s.push(n);
            s.push(c);
            s
        });

        choice((non_special.map(|c: char| c.to_string()), newline_not_tab))
            .repeated()
            .at_least(1)
            .fold(String::new(), |mut acc: String, s: String| {
                acc.push_str(&s);
                acc
            })
            .map(BulletContentSegment::text)
    };

    // Parse segments: try picture first (most specific), then bullet, then continuation, then text
    choice((picture, bullet, continuation, text_content))
        .repeated()
        .collect::<Vec<_>>()
}

/// Parse bullet-aware free text into structured `BulletContent`.
pub(crate) fn parse_bullet_content_text(input: &str) -> BulletContent {
    match text_tier_content_parser().parse(input).into_result() {
        Ok(segments) => BulletContent::new(segments),
        Err(_) => BulletContent::from_text(input),
    }
}

/// Helper to create a text tier from parsed content
fn parse_text_tier<T, F>(
    input: &str,
    _offset: usize,
    _errors: &impl ErrorSink,
    constructor: F,
) -> ParseOutcome<T>
where
    F: FnOnce(BulletContent) -> T,
{
    let parser = text_tier_content_parser();
    match parser.parse(input).into_result() {
        Ok(segments) => {
            let content = BulletContent::new(segments);
            ParseOutcome::parsed(constructor(content))
        }
        Err(_) => ParseOutcome::rejected(),
    }
}

/// Parse %com tier content
pub fn parse_com_tier_content(
    input: &str,
    offset: usize,
    errors: &impl ErrorSink,
) -> ParseOutcome<ComTier> {
    parse_text_tier(input, offset, errors, ComTier::new)
}

/// Parse %exp tier content
pub fn parse_exp_tier_content(
    input: &str,
    offset: usize,
    errors: &impl ErrorSink,
) -> ParseOutcome<ExpTier> {
    parse_text_tier(input, offset, errors, ExpTier::new)
}

/// Parse %add tier content
pub fn parse_add_tier_content(
    input: &str,
    offset: usize,
    errors: &impl ErrorSink,
) -> ParseOutcome<AddTier> {
    parse_text_tier(input, offset, errors, AddTier::new)
}

/// Parse %gpx tier content
pub fn parse_gpx_tier_content(
    input: &str,
    offset: usize,
    errors: &impl ErrorSink,
) -> ParseOutcome<GpxTier> {
    parse_text_tier(input, offset, errors, GpxTier::new)
}

/// Parse %int tier content
pub fn parse_int_tier_content(
    input: &str,
    offset: usize,
    errors: &impl ErrorSink,
) -> ParseOutcome<IntTier> {
    parse_text_tier(input, offset, errors, IntTier::new)
}

/// Parse %spa tier content
pub fn parse_spa_tier_content(
    input: &str,
    offset: usize,
    errors: &impl ErrorSink,
) -> ParseOutcome<SpaTier> {
    parse_text_tier(input, offset, errors, SpaTier::new)
}

/// Parse %sit tier content
pub fn parse_sit_tier_content(
    input: &str,
    offset: usize,
    errors: &impl ErrorSink,
) -> ParseOutcome<SitTier> {
    parse_text_tier(input, offset, errors, SitTier::new)
}

/// Parse %act tier content
pub fn parse_act_tier_content(
    input: &str,
    offset: usize,
    errors: &impl ErrorSink,
) -> ParseOutcome<ActTier> {
    parse_text_tier(input, offset, errors, ActTier::new)
}

/// Parse %cod tier content
pub fn parse_cod_tier_content(
    input: &str,
    offset: usize,
    errors: &impl ErrorSink,
) -> ParseOutcome<CodTier> {
    parse_text_tier(input, offset, errors, CodTier::new)
}
