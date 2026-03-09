//! Chumsky parsers for CHAT file structure (tier splitting with continuation lines).

use chumsky::prelude::*;

/// Tier type classification for CHAT format.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TierType {
    Header,        // Starts with @
    MainTier,      // Starts with *
    DependentTier, // Starts with %
}

/// Raw tier with prefix, content (including embedded \n\t continuations), and offset.
#[derive(Debug)]
pub(crate) struct RawTier {
    pub(crate) tier_type: TierType,
    pub(crate) content: String, // Includes continuation markers (\n\t, \r\n\t) for ws_parser()
    pub(crate) offset: usize,   // Byte offset of the tier start (prefix character)
}

/// Parse tier content: everything from after the prefix until the next tier starts.
///
/// A tier's content extends until we see a newline followed by @, *, or %.
/// Continuation lines (\n\t) are kept in the content for ws_parser() to handle.
fn tier_content_parser<'a>() -> impl Parser<'a, &'a str, String, extra::Err<Rich<'a, char>>> + Clone
{
    // Parse characters until we see \n followed by [@*%]
    // We need to keep \n\t sequences for ws_parser()

    // Strategy: consume characters, including newlines, but stop when we see \n[@*%]
    // Use a recursive approach: consume a character if it's not the start of a tier boundary

    any()
        .and_is(
            // Lookahead: NOT (\n followed by [@*%])
            just('\n').then(one_of("@*%")).not(),
        )
        .or(
            // Allow \r at end of file or before continuation
            just('\r').and_is(just('\n').then(one_of("@*%")).not()),
        )
        .repeated()
        .collect::<String>()
}

/// Parse a single tier (header, main, or dependent) with its continuation lines.
///
/// Returns the tier type, content (INCLUDING prefix and \n\t markers), and starting offset.
fn single_tier_parser<'a>() -> impl Parser<'a, &'a str, RawTier, extra::Err<Rich<'a, char>>> + Clone
{
    just('@')
        .to(TierType::Header)
        .or(just('*').to(TierType::MainTier))
        .or(just('%').to(TierType::DependentTier))
        .then(tier_content_parser())
        .map_with(|(tier_type, tier_content), e| {
            // Reconstruct full content including the prefix character
            let prefix_char = match tier_type {
                TierType::Header => '@',
                TierType::MainTier => '*',
                TierType::DependentTier => '%',
            };
            let mut content = String::with_capacity(1 + tier_content.len());
            content.push(prefix_char);
            content.push_str(&tier_content);

            RawTier {
                tier_type,
                content,
                offset: e.span().start,
            }
        })
}

/// Parse all tiers in a CHAT file.
///
/// Skips empty lines and handles continuation lines automatically.
pub(crate) fn file_tiers_parser<'a>()
-> impl Parser<'a, &'a str, Vec<RawTier>, extra::Err<Rich<'a, char>>> {
    // Skip leading whitespace/empty lines
    let empty_lines = one_of(" \t\n\r").repeated();

    empty_lines
        .ignore_then(
            single_tier_parser()
                .separated_by(empty_lines)
                .allow_trailing()
                .collect::<Vec<_>>(),
        )
        .then_ignore(empty_lines)
        .then_ignore(end())
}
