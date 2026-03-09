//! %wor tier parsing using pure Chumsky combinators.
//!
//! The word timing tier (%wor) provides word-level timing information.
//! Format: `word \x15start_end\x15 word \x15start_end\x15 ... terminator`
//!
//! This parser splits text by whitespace into tokens, classifies each as
//! a word or tag-marker separator, pairs words with following inline timing
//! bullets, and extracts the terminator from the last token.
//!
//! # Related CHAT Manual Sections
//!
//! - <https://talkbank.org/0info/manuals/CHAT.html#Dependent_Tiers>
//! - <https://talkbank.org/0info/manuals/CHAT.html#Word_Timing_Tier>
//! - <https://talkbank.org/0info/manuals/CHAT.html#Working_with_Media>

use talkbank_model::ErrorSink;
use talkbank_model::ParseOutcome;
use talkbank_model::model::content::word::Word;
use talkbank_model::model::dependent_tier::WorItem;
use talkbank_model::model::{Bullet, Terminator, WorTier};

/// Tag-marker separator characters that appear untimed in %wor tiers.
/// Comma `,`, tag `„` (U+201E), vocative `‡` (U+2021).
const TAG_MARKER_CHARS: &[&str] = &[",", "\u{201E}", "\u{2021}"];

/// Parse %wor tier content (without `%wor:\t` prefix) using chumsky combinators.
///
/// The input is flat text with optional inline bullets: `word \x15N_N\x15 word ... .`
pub fn parse_wor_tier_content(
    input: &str,
    offset: usize,
    _errors: &impl ErrorSink,
) -> ParseOutcome<WorTier> {
    let (items, terminator) = parse_wor_text(input);

    let tier = WorTier::new(items).with_terminator(terminator).with_span(
        talkbank_model::Span::from_usize(offset, offset + input.len()),
    );
    ParseOutcome::parsed(tier)
}

/// Parse %wor text into items + terminator.
///
/// Splits on bullet delimiters (\x15) first, then splits text parts by whitespace.
/// Each text token is classified as a word or tag-marker separator.
/// Words are paired with a following bullet's timing if present.
fn parse_wor_text(input: &str) -> (Vec<WorItem>, Option<Terminator>) {
    let mut tokens: Vec<WorToken> = Vec::new();

    // Split on bullet delimiter \x15
    let parts: Vec<&str> = input.split('\x15').collect();

    for (i, part) in parts.iter().enumerate() {
        if i % 2 == 0 {
            // Text segment (outside bullets)
            for word in part.split_whitespace() {
                tokens.push(WorToken::Text(word.to_string()));
            }
        } else {
            // Inside bullet: should be "start_end"
            if let Some(timing) = parse_bullet_content(part) {
                tokens.push(WorToken::Timing(timing));
            }
        }
    }

    // Convert tokens to WorItems + extract terminator
    let mut items: Vec<WorItem> = Vec::new();
    let mut idx = 0;

    while idx < tokens.len() {
        match &tokens[idx] {
            WorToken::Text(text) => {
                // Check if this token is a tag-marker separator
                if TAG_MARKER_CHARS.contains(&text.as_str()) {
                    items.push(WorItem::Separator {
                        text: text.clone(),
                        span: talkbank_model::Span::DUMMY,
                    });
                } else {
                    let bullet = if idx + 1 < tokens.len() {
                        if let WorToken::Timing(b) = &tokens[idx + 1] {
                            idx += 1;
                            Some(b.clone())
                        } else {
                            None
                        }
                    } else {
                        None
                    };

                    // Create word with inline bullet
                    let mut word = Word::new_unchecked(text, text);
                    word.inline_bullet = bullet;
                    items.push(WorItem::Word(Box::new(word)));
                }
            }
            WorToken::Timing(_) => {} // orphan timing — skip
        }
        idx += 1;
    }

    // Extract terminator from last item (must be a word)
    let terminator = match items.last() {
        Some(WorItem::Word(w)) => parse_terminator(w.raw_text()),
        _ => None,
    };

    if terminator.is_some() {
        items.pop();
    }

    (items, terminator)
}

/// Lexemes emitted by the lightweight `%wor` tokenizer.
enum WorToken {
    Text(String),
    Timing(Bullet),
}

/// Parse bullet content "start_end" into a Bullet.
fn parse_bullet_content(s: &str) -> Option<Bullet> {
    // Handle both "start_end" and "start_end-" (skip marker)
    let s = s.trim_end_matches('-');
    let parts: Vec<&str> = s.split('_').collect();
    if parts.len() == 2
        && let (Ok(start), Ok(end)) = (parts[0].parse::<u64>(), parts[1].parse::<u64>())
    {
        return Some(Bullet::new(start, end));
    }
    None
}

/// Try to parse a terminator from text.
fn parse_terminator(text: &str) -> Option<Terminator> {
    let span = talkbank_model::Span::DUMMY;
    match text {
        "." => Some(Terminator::Period { span }),
        "?" => Some(Terminator::Question { span }),
        "!" => Some(Terminator::Exclamation { span }),
        "+..." => Some(Terminator::TrailingOff { span }),
        "+/." => Some(Terminator::Interruption { span }),
        "+//." => Some(Terminator::SelfInterruption { span }),
        "+/?" => Some(Terminator::InterruptedQuestion { span }),
        "+!?" => Some(Terminator::BrokenQuestion { span }),
        "+\"/." => Some(Terminator::QuotedNewLine { span }),
        "+\"." => Some(Terminator::QuotedPeriodSimple { span }),
        "+//?" => Some(Terminator::SelfInterruptedQuestion { span }),
        "+..?" => Some(Terminator::TrailingOffQuestion { span }),
        "+." => Some(Terminator::BreakForCoding { span }),
        "\u{21D7}" => Some(Terminator::CaRisingToHigh { span }),
        "\u{2197}" => Some(Terminator::CaRisingToMid { span }),
        "\u{2192}" => Some(Terminator::CaLevel { span }),
        "\u{2198}" => Some(Terminator::CaFallingToMid { span }),
        "\u{21D8}" => Some(Terminator::CaFallingToLow { span }),
        "\u{224B}" => Some(Terminator::CaTechnicalBreak { span }),
        "+\u{224B}" => Some(Terminator::CaTechnicalBreakLinker { span }),
        "\u{2248}" => Some(Terminator::CaNoBreak { span }),
        "+\u{2248}" => Some(Terminator::CaNoBreakLinker { span }),
        _ => None,
    }
}
