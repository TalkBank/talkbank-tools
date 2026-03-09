//! CHAT whitespace parsing.
//!
//! CRITICAL RULES:
//! - Tab is NOT general whitespace
//! - Tab only allowed: (1) after prefix `:\t`, (2) in continuation `\n\t`
//! - Whitespace is: space OR continuation line
//!
//! # Related CHAT Manual Sections
//!
//! - <https://talkbank.org/0info/manuals/CHAT.html#File_Format>
//! - <https://talkbank.org/0info/manuals/CHAT.html#Main_Tier>
//! - <https://talkbank.org/0info/manuals/CHAT.html#Dependent_Tiers>

use chumsky::prelude::*;

/// Parse CHAT whitespace: space OR continuation line (LF/CRLF + tab).
///
/// **Tab is NOT whitespace except in continuation lines!**
///
/// Examples:
/// - `" "` → space (valid)
/// - `"\n\t"` → continuation line LF+tab (valid)
/// - `"\r\n\t"` → continuation line CRLF+tab (valid)
/// - `"\t"` → NOT whitespace (invalid!)
pub fn ws_parser<'a>() -> impl Parser<'a, &'a str, (), extra::Err<Rich<'a, char>>> {
    choice((
        just("\r\n\t"), // Continuation line (CRLF + tab = whitespace)
        just("\n\t"),   // Continuation line (LF + tab = whitespace)
        just(" "),      // Space only (NOT tab!) - use &str for consistency
    ))
    .repeated()
    .at_least(1)
    .ignored()
}

/// Optional whitespace (0 or more).
#[allow(dead_code)]
pub fn ws0_parser<'a>() -> impl Parser<'a, &'a str, (), extra::Err<Rich<'a, char>>> {
    choice((just("\r\n\t"), just("\n\t"), just(" ")))
        .repeated()
        .ignored()
}

/// Parse non-whitespace content (anything except space, newline, tab).
///
/// Use this for parsing word content that should stop at whitespace boundaries.
#[allow(dead_code)]
pub fn non_ws<'a>() -> impl Parser<'a, &'a str, char, extra::Err<Rich<'a, char>>> {
    none_of(" \n\t\r")
}

#[cfg(test)]
mod tests {
    use super::*;
    use chumsky::Parser;

    /// Tests space is whitespace.
    #[test]
    fn test_space_is_whitespace() {
        let result = ws_parser().parse(" ").into_result();
        assert!(result.is_ok());
    }

    /// Tests continuation lf is whitespace.
    #[test]
    fn test_continuation_lf_is_whitespace() {
        let result = ws_parser().parse("\n\t").into_result();
        assert!(result.is_ok());
    }

    /// Tests continuation crlf is whitespace.
    #[test]
    fn test_continuation_crlf_is_whitespace() {
        let result = ws_parser().parse("\r\n\t").into_result();
        assert!(result.is_ok());
    }

    /// Tests tab alone not whitespace.
    #[test]
    fn test_tab_alone_not_whitespace() {
        let result = ws_parser().parse("\t").into_result();
        assert!(result.is_err()); // Tab alone is NOT whitespace!
    }

    /// Tests multiple spaces.
    #[test]
    fn test_multiple_spaces() {
        let result = ws_parser().parse("   ").into_result();
        assert!(result.is_ok());
    }

    /// Tests space then continuation.
    #[test]
    fn test_space_then_continuation() {
        let result = ws_parser().parse(" \n\t ").into_result();
        assert!(result.is_ok());
    }

    /// Tests ws0 empty.
    #[test]
    fn test_ws0_empty() {
        let result = ws0_parser().parse("").into_result();
        assert!(result.is_ok());
    }

    /// Tests non ws.
    #[test]
    fn test_non_ws() {
        let result = non_ws().parse("a").into_result();
        assert!(result.is_ok());

        let result = non_ws().parse(" ").into_result();
        assert!(result.is_err());

        let result = non_ws().parse("\t").into_result();
        assert!(result.is_err());
    }
}
