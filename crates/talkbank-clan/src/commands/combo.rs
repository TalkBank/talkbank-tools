//! COMBO — Boolean keyword search across utterances.
//!
//! Reimplements CLAN's COMBO command, which searches for utterances matching
//! boolean combinations of keywords. Supports AND (`+`) and OR (`,`) logic
//! with case-insensitive substring matching. This is the primary search tool
//! for finding utterances containing specific words or word combinations.
//!
//! # CLAN Manual
//!
//! See the [CLAN manual](https://talkbank.org/0info/manuals/CLAN.html#_Toc220409095)
//! for the original COMBO command specification.
//!
//! # CLAN Equivalence
//!
//! | CLAN command                                | Rust equivalent                                          |
//! |---------------------------------------------|----------------------------------------------------------|
//! | `combo +s"want^cookie" file.cha`            | `chatter analyze combo file.cha -s "want+cookie"`        |
//! | `combo +s"want\|milk" file.cha`             | `chatter analyze combo file.cha -s "want,milk"`          |
//! | `combo +s"want^cookie" +t*CHI file.cha`     | `chatter analyze combo file.cha -s "want+cookie" -S CHI` |
//!
//! # Search Syntax
//!
//! - `+` between terms means AND (all terms must be present in the utterance)
//! - `,` between terms means OR (at least one term must be present)
//! - Terms are case-insensitive substring matches against countable words
//! - Multiple `-s` flags are combined with OR (any expression matching counts)
//! - AND takes precedence if both `+` and `,` appear in one expression
//!
//! # Differences from CLAN
//!
//! - CLAN uses `^` for AND and `\|` for OR; this implementation uses `+` and `,`
//!   respectively for shell-friendliness.
//!
//! # Output
//!
//! Each matching utterance with:
//! - Source filename
//! - Speaker code
//! - Full utterance text (CHAT format)
//! - Summary counts of matching vs. total utterances

use serde::Serialize;
use talkbank_model::{Utterance, WriteChat};

use crate::framework::word_filter::{countable_words, word_pattern_matches};
use crate::framework::{
    AnalysisCommand, AnalysisResult, CommandOutput, FileContext, NormalizedWord, OutputFormat,
    Section, TableRow, UtteranceCount,
};

/// A single search expression (terms joined by AND or OR).
///
/// # Examples
///
/// ```
/// use talkbank_clan::commands::combo::SearchExpr;
///
/// // AND: all terms must appear
/// let expr = SearchExpr::parse("want+cookie");
/// assert!(matches!(expr, SearchExpr::And(_)));
///
/// // OR: at least one term must appear
/// let expr = SearchExpr::parse("cookie,milk");
/// assert!(matches!(expr, SearchExpr::Or(_)));
///
/// // Bare term: treated as single-element AND
/// let expr = SearchExpr::parse("hello");
/// assert!(matches!(expr, SearchExpr::And(_)));
/// ```
#[derive(Debug, Clone)]
pub enum SearchExpr {
    /// All terms must be present in the utterance.
    And(Vec<String>),
    /// At least one term must be present in the utterance.
    Or(Vec<String>),
}

impl SearchExpr {
    /// Parse a search string into an expression.
    ///
    /// - `+` splits into AND terms
    /// - `,` splits into OR terms
    /// - If neither is present, treated as a single AND term
    ///
    /// AND takes precedence: if both `+` and `,` appear, the string
    /// is split on `+` first (matching CLAN's behavior).
    pub fn parse(s: &str) -> Self {
        if s.contains('+') {
            let terms: Vec<String> = s.split('+').map(|t| t.trim().to_lowercase()).collect();
            SearchExpr::And(terms)
        } else if s.contains(',') {
            let terms: Vec<String> = s.split(',').map(|t| t.trim().to_lowercase()).collect();
            SearchExpr::Or(terms)
        } else {
            SearchExpr::And(vec![s.trim().to_lowercase()])
        }
    }

    /// Check whether the given normalized word set satisfies this expression.
    ///
    /// Matching is case-insensitive with exact word matching (wildcards `*`
    /// supported). Words are already lowercased via [`NormalizedWord`].
    fn matches(&self, words: &[NormalizedWord]) -> bool {
        match self {
            SearchExpr::And(terms) => terms.iter().all(|term| {
                words
                    .iter()
                    .any(|w| word_pattern_matches(w.as_str(), term.as_str()))
            }),
            SearchExpr::Or(terms) => terms.iter().any(|term| {
                words
                    .iter()
                    .any(|w| word_pattern_matches(w.as_str(), term.as_str()))
            }),
        }
    }

    /// Return the set of word forms in `words` that contributed to a
    /// successful match. For And, returns one word per term (the
    /// first occurrence). For Or, returns every word whose form
    /// matches any term. Lowercased forms.
    ///
    /// Used by CLAN-format rendering to wrap matched words as
    /// `(N)<word>` in the utterance echo.
    fn matched_words(&self, words: &[NormalizedWord]) -> Vec<String> {
        let mut out = Vec::new();
        match self {
            SearchExpr::And(terms) => {
                for term in terms {
                    if let Some(w) = words
                        .iter()
                        .find(|w| word_pattern_matches(w.as_str(), term.as_str()))
                    {
                        out.push(w.as_str().to_owned());
                    }
                }
            }
            SearchExpr::Or(terms) => {
                for w in words {
                    if terms
                        .iter()
                        .any(|t| word_pattern_matches(w.as_str(), t.as_str()))
                    {
                        out.push(w.as_str().to_owned());
                    }
                }
            }
        }
        out
    }
}

/// Configuration for the COMBO command.
#[derive(Debug, Clone, Default)]
pub struct ComboConfig {
    /// Search expressions (multiple are combined with OR).
    pub search: Vec<SearchExpr>,
}

/// A single match found during COMBO processing.
#[derive(Debug, Clone, Serialize)]
pub struct ComboMatch {
    /// Speaker code.
    pub speaker: String,
    /// Full utterance text (CHAT format).
    pub utterance_text: String,
    /// Source filename.
    pub filename: String,
    /// 1-based source line number of the utterance — used by
    /// CLAN-compatible rendering to emit
    /// `*** File "pipeout": line N.`. `0` when no line map is
    /// available.
    pub line_number: usize,
    /// Per-search-expression hits: for each configured `SearchExpr`,
    /// the 1-based index of the expression and the set of lowercased
    /// word tokens that contributed to its match. CLAN-format
    /// rendering wraps each matched word as `(N)<word>` where `N` is
    /// the expression index.
    pub expr_hits: Vec<MatchedExpr>,
}

/// One search expression's contribution to a `ComboMatch`.
#[derive(Debug, Clone, Serialize)]
pub struct MatchedExpr {
    /// 1-based index of the expression in `ComboConfig.search`.
    pub index: usize,
    /// Lowercased word forms that the expression matched against this
    /// utterance.
    pub matched_words: Vec<String>,
}

/// Typed output for the COMBO command.
#[derive(Debug, Clone, Serialize)]
pub struct ComboResult {
    /// All matching utterances in order encountered.
    pub matches: Vec<ComboMatch>,
    /// Total utterances examined (including non-matches).
    pub total_utterances: UtteranceCount,
}

impl ComboResult {
    /// Convert typed matches into the shared table-based rendering container.
    fn to_analysis_result(&self) -> AnalysisResult {
        let mut result = AnalysisResult::new("combo");
        if !self.matches.is_empty() {
            let rows: Vec<TableRow> = self
                .matches
                .iter()
                .map(|m| TableRow {
                    values: vec![
                        m.filename.clone(),
                        m.speaker.clone(),
                        m.utterance_text.clone(),
                    ],
                })
                .collect();

            let mut section = Section::with_table(
                "Matches".to_owned(),
                vec![
                    "File".to_owned(),
                    "Speaker".to_owned(),
                    "Utterance".to_owned(),
                ],
                rows,
            );
            section.fields.insert(
                "Matching utterances".to_owned(),
                self.matches.len().to_string(),
            );
            section.fields.insert(
                "Total utterances".to_owned(),
                self.total_utterances.to_string(),
            );
            result.add_section(section);
        }
        result
    }
}

impl CommandOutput for ComboResult {
    /// Render via the shared tabular text formatter.
    fn render_text(&self) -> String {
        self.to_analysis_result().render(OutputFormat::Text)
    }

    /// CLAN-compatible output matching legacy `combo` character-for-character.
    ///
    /// Format (from CLAN snapshot):
    /// ```text
    /// ----------------------------------------
    /// *** File "pipeout": line 6.
    /// *MOT:    (1)the (1)cat is on the mat .
    /// ----------------------------------------
    /// *** File "pipeout": line 12.
    /// *MOT:    yes , (1)the (1)cat .
    /// ----------------------------------------
    ///
    ///     Strings matched 3 times
    /// ```
    ///
    /// CLAN's combo wraps each word that matched the configured
    /// search expression with `(N)` where `N` is the 1-based index
    /// of the expression. Multiple expressions can match in the
    /// same utterance; each contributing word gets its own
    /// `(<expression-index>)` prefix.
    fn render_clan(&self) -> String {
        use std::fmt::Write;
        let mut out = String::new();

        for m in &self.matches {
            writeln!(out, "----------------------------------------").ok();
            // CLAN uses "pipeout" as the filename when reading from
            // stdin (chatter follows the same convention for
            // CLAN-format output to match the byte stream).
            writeln!(out, "*** File \"pipeout\": line {}.", m.line_number).ok();
            // utterance_text already carries the `*SPK:\t...` prefix
            // (`Utterance::Main::to_chat_string()` includes it), so
            // we don't add another speaker prefix here. Wrap each
            // matched word as (N)<word> in place.
            let annotated = annotate_combo_matches(&m.utterance_text, &m.expr_hits);
            writeln!(out, "{annotated}").ok();
        }
        // Summary line. CLAN emits:
        //   <last match line>\n\n    Strings matched N times\n\n
        // No trailing `----` after the last match (the separators
        // appear *before* each match, not between or after).
        if !self.matches.is_empty() {
            writeln!(out).ok();
            writeln!(out, "    Strings matched {} times", self.matches.len()).ok();
            writeln!(out).ok();
        }
        out
    }
}

/// Wrap matched words in `text` with their expression-index prefix
/// `(N)`. CLAN's combo annotates the **first occurrence** of each
/// matched word per expression, not every occurrence — so for the
/// AND search `the+cat` against `the cat is on the mat`, only the
/// first `the` gets `(1)the`; the second `the` is left bare.
///
/// Implementation: keep a per-word "still owed" count for each
/// `(expr_index, lowercased_word)` pair, decrement on each match
/// during token walk, stop annotating that word once the budget is
/// exhausted.
fn annotate_combo_matches(text: &str, expr_hits: &[MatchedExpr]) -> String {
    if expr_hits.is_empty() {
        return text.to_owned();
    }
    // (lowercased_word) -> (expr_index, remaining_budget).
    // Lower expr indices "win" when multiple expressions matched
    // the same word, matching CLAN's first-expression-wins shape.
    let mut budget: std::collections::HashMap<String, (usize, usize)> =
        std::collections::HashMap::new();
    for hit in expr_hits {
        for w in &hit.matched_words {
            let entry = budget.entry(w.clone()).or_insert((hit.index, 0));
            // Lower expression index wins.
            if hit.index < entry.0 {
                *entry = (hit.index, entry.1 + 1);
            } else {
                entry.1 += 1;
            }
        }
    }
    // Preserve the leading `*SPK:\t` prefix verbatim — CLAN emits
    // a real tab between speaker and content; `split_whitespace`
    // would collapse it to a single space. Only the body after the
    // tab is rewritten with the `(N)` prefixes.
    let (prefix, body) = match text.find('\t') {
        Some(tab_pos) => text.split_at(tab_pos + 1),
        None => ("", text),
    };
    let mut out = String::with_capacity(text.len());
    out.push_str(prefix);
    // Token-walk the body, prefixing each matched token with `(N)`
    // while it still has budget. Token boundaries are whitespace;
    // punctuation tokens (`,`, `.`, etc.) are left untouched and
    // don't consume budget.
    let mut first = true;
    for tok in body.split_whitespace() {
        if !first {
            out.push(' ');
        }
        first = false;
        let lower = tok.to_lowercase();
        if let Some(slot) = budget.get_mut(lower.as_str()) {
            if slot.1 > 0 {
                out.push_str(&format!("({}){tok}", slot.0));
                slot.1 -= 1;
                continue;
            }
        }
        out.push_str(tok);
    }
    out
}

/// Accumulated state for COMBO across all files.
#[derive(Debug, Default)]
pub struct ComboState {
    /// All matches found
    matches: Vec<ComboMatch>,
    /// Total utterances examined
    total_utterances: u64,
}

/// COMBO command implementation.
///
/// For each utterance, extracts all countable words and checks whether
/// any search expression is satisfied. Multiple search expressions are
/// combined with OR logic (any expression matching counts).
#[derive(Debug, Clone, Default)]
pub struct ComboCommand {
    config: ComboConfig,
}

impl ComboCommand {
    /// Create a COMBO command with the given configuration.
    pub fn new(config: ComboConfig) -> Self {
        Self { config }
    }
}

impl AnalysisCommand for ComboCommand {
    type Config = ComboConfig;
    type State = ComboState;
    type Output = ComboResult;

    /// Evaluate all configured boolean keyword expressions for one utterance.
    fn process_utterance(
        &self,
        utterance: &Utterance,
        file_context: &FileContext<'_>,
        state: &mut Self::State,
    ) {
        if self.config.search.is_empty() {
            return;
        }

        state.total_utterances += 1;

        // Collect normalized words using the shared iterator
        let words: Vec<NormalizedWord> = countable_words(&utterance.main.content.content)
            .map(NormalizedWord::from_word)
            .collect();

        // Collect per-expression match details for CLAN-format rendering.
        let expr_hits: Vec<MatchedExpr> = self
            .config
            .search
            .iter()
            .enumerate()
            .filter_map(|(i, expr)| {
                if expr.matches(&words) {
                    Some(MatchedExpr {
                        index: i + 1,
                        matched_words: expr.matched_words(&words),
                    })
                } else {
                    None
                }
            })
            .collect();

        if !expr_hits.is_empty() {
            let utterance_text = utterance.main.to_chat_string();
            let line_number = file_context
                .line_map
                .map(|lm| lm.line_of(utterance.main.span.start))
                .unwrap_or(0);
            state.matches.push(ComboMatch {
                speaker: utterance.main.speaker.as_str().to_owned(),
                utterance_text,
                filename: file_context.filename.to_owned(),
                line_number,
                expr_hits,
            });
        }
    }

    /// Move accumulated matches and counters into the typed result.
    fn finalize(&self, state: Self::State) -> ComboResult {
        ComboResult {
            matches: state.matches,
            total_utterances: state.total_utterances,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use talkbank_model::Span;
    use talkbank_model::{MainTier, Terminator, UtteranceContent, Word};

    /// Build a minimal utterance with plain lexical tokens for tests.
    fn make_utterance(speaker: &str, words: &[&str]) -> Utterance {
        let content: Vec<UtteranceContent> = words
            .iter()
            .map(|w| UtteranceContent::Word(Box::new(Word::simple(*w))))
            .collect();
        let main = MainTier::new(speaker, content, Terminator::Period { span: Span::DUMMY });
        Utterance::new(main)
    }

    /// Build a stable `FileContext` fixture reused by command tests.
    fn file_ctx(chat_file: &talkbank_model::ChatFile) -> FileContext<'_> {
        FileContext {
            path: std::path::Path::new("test.cha"),
            chat_file,
            filename: "test",
            line_map: None,
        }
    }

    /// AND expressions should match only when all terms are present.
    #[test]
    fn combo_and_both_present() {
        let command = ComboCommand::new(ComboConfig {
            search: vec![SearchExpr::parse("want+cookie")],
        });
        let mut state = ComboState::default();
        let chat_file = talkbank_model::ChatFile::new(vec![]);
        let ctx = file_ctx(&chat_file);

        let u = make_utterance("CHI", &["I", "want", "cookie"]);
        command.process_utterance(&u, &ctx, &mut state);

        assert_eq!(state.matches.len(), 1);
    }

    /// AND expressions should fail when any required term is missing.
    #[test]
    fn combo_and_missing_one() {
        let command = ComboCommand::new(ComboConfig {
            search: vec![SearchExpr::parse("want+cookie")],
        });
        let mut state = ComboState::default();
        let chat_file = talkbank_model::ChatFile::new(vec![]);
        let ctx = file_ctx(&chat_file);

        // Has "want" but not "cookie"
        let u = make_utterance("CHI", &["I", "want", "milk"]);
        command.process_utterance(&u, &ctx, &mut state);

        assert_eq!(state.matches.len(), 0);
    }

    /// OR expressions should match when any candidate term appears.
    #[test]
    fn combo_or_either_present() {
        let command = ComboCommand::new(ComboConfig {
            search: vec![SearchExpr::parse("cookie,milk")],
        });
        let mut state = ComboState::default();
        let chat_file = talkbank_model::ChatFile::new(vec![]);
        let ctx = file_ctx(&chat_file);

        let u1 = make_utterance("CHI", &["I", "want", "cookie"]);
        let u2 = make_utterance("CHI", &["I", "want", "milk"]);
        let u3 = make_utterance("CHI", &["I", "want", "juice"]);

        command.process_utterance(&u1, &ctx, &mut state);
        command.process_utterance(&u2, &ctx, &mut state);
        command.process_utterance(&u3, &ctx, &mut state);

        assert_eq!(state.matches.len(), 2); // cookie and milk match, juice doesn't
    }

    /// Multiple `-s` expressions combine with top-level OR semantics.
    #[test]
    fn combo_multiple_expressions_or() {
        // Multiple -s flags: "want+cookie" OR "need+milk"
        let command = ComboCommand::new(ComboConfig {
            search: vec![
                SearchExpr::parse("want+cookie"),
                SearchExpr::parse("need+milk"),
            ],
        });
        let mut state = ComboState::default();
        let chat_file = talkbank_model::ChatFile::new(vec![]);
        let ctx = file_ctx(&chat_file);

        let u1 = make_utterance("CHI", &["I", "want", "cookie"]);
        let u2 = make_utterance("CHI", &["I", "need", "milk"]);
        let u3 = make_utterance("CHI", &["I", "want", "milk"]); // neither AND matches fully

        command.process_utterance(&u1, &ctx, &mut state);
        command.process_utterance(&u2, &ctx, &mut state);
        command.process_utterance(&u3, &ctx, &mut state);

        assert_eq!(state.matches.len(), 2);
    }

    /// Empty search config should produce no matches.
    #[test]
    fn combo_empty_search() {
        let command = ComboCommand::new(ComboConfig { search: vec![] });
        let mut state = ComboState::default();
        let chat_file = talkbank_model::ChatFile::new(vec![]);
        let ctx = file_ctx(&chat_file);

        let u = make_utterance("CHI", &["hello"]);
        command.process_utterance(&u, &ctx, &mut state);

        let result = command.finalize(state);
        assert!(result.matches.is_empty());
    }

    /// Parsing should map `+` to AND, `,` to OR, and bare terms to single AND.
    #[test]
    fn search_expr_parse() {
        match SearchExpr::parse("want+cookie") {
            SearchExpr::And(terms) => assert_eq!(terms, vec!["want", "cookie"]),
            _ => panic!("expected And"),
        }
        match SearchExpr::parse("want,cookie") {
            SearchExpr::Or(terms) => assert_eq!(terms, vec!["want", "cookie"]),
            _ => panic!("expected Or"),
        }
        match SearchExpr::parse("want") {
            SearchExpr::And(terms) => assert_eq!(terms, vec!["want"]),
            _ => panic!("expected And with single term"),
        }
    }
}
