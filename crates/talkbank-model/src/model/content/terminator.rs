//! Utterance terminator tokens.
//!
//! Terminators encode sentence-final punctuation, interruption/completion state,
//! and CA intonation boundaries.
//!
//! # CHAT Format References
//!
//! - [Terminators](https://talkbank.org/0info/manuals/CHAT.html#Terminators)
//! - [CA Intonation](https://talkbank.org/0info/manuals/CHAT.html#CA_Intonation)

use super::WriteChat;
use crate::Span;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use talkbank_derive::{SemanticEq, SpanShift};

/// End-of-utterance token variant.
///
/// # Standard Terminators
///
/// - `.` - Period (declarative)
/// - `?` - Question mark (interrogative)
/// - `!` - Exclamation (imperative/exclamatory)
///
/// # Interruption Terminators
///
/// - `+...` - Trailing off (incomplete thought)
/// - `+/.` - Interruption by another speaker
/// - `+//.` - Self-interruption
/// - `+/?` - Interrupted question
/// - `+//?` - Self-interrupted question
/// - `+/??` - Broken off question
///
/// # CA (Conversation Analysis) Intonation
///
/// - `⇗` - Rising to high
/// - `↗` - Rising to mid
/// - `→` - Level/continuing
/// - `↘` - Falling to mid
/// - `⇘` - Falling to low
///
/// # CHAT Format Examples
///
/// ```text
/// *CHI: I want that .         Standard declarative
/// *MOT: what do you want ?    Question
/// *CHI: look at this !        Exclamation
/// *CHI: I was going to +...   Trailing off
/// *MOT: did you +/. yes I did Interrupted by CHI
/// *CHI: um the +//. the dog   Self-interruption
/// ```
///
/// # References
///
/// - [Terminators](https://talkbank.org/0info/manuals/CHAT.html#Terminators)
/// - [Interruption Terminator](https://talkbank.org/0info/manuals/CHAT.html#Interruption_Terminator)
/// - [CA Intonation](https://talkbank.org/0info/manuals/CHAT.html#CA_Intonation)
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema, SemanticEq, SpanShift)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum Terminator {
    /// Period `.` - declarative statement
    /// Reference: <https://talkbank.org/0info/manuals/CHAT.html#Period_Terminator>
    Period {
        #[serde(skip)]
        #[schemars(skip)]
        #[semantic_eq(skip)]
        /// Source span for error reporting.
        span: Span,
    },
    /// Question mark `?` - interrogative
    /// Reference: <https://talkbank.org/0info/manuals/CHAT.html#QuestionMark_Terminator>
    Question {
        #[serde(skip)]
        #[schemars(skip)]
        #[semantic_eq(skip)]
        /// Source span for error reporting.
        span: Span,
    },
    /// Exclamation `!` - imperative/exclamatory
    /// Reference: <https://talkbank.org/0info/manuals/CHAT.html#ExclamationMark_Terminator>
    Exclamation {
        #[serde(skip)]
        #[schemars(skip)]
        #[semantic_eq(skip)]
        /// Source span for error reporting.
        span: Span,
    },
    /// +... - trailing off (incomplete utterance)
    /// Reference: <https://talkbank.org/0info/manuals/CHAT.html#TrailingOff_Terminator>
    TrailingOff {
        #[serde(skip)]
        #[schemars(skip)]
        #[semantic_eq(skip)]
        /// Source span for error reporting.
        span: Span,
    },
    /// +/. - interruption (interrupted by another speaker)
    /// Reference: <https://talkbank.org/0info/manuals/CHAT.html#Interruption_Terminator>
    Interruption {
        #[serde(skip)]
        #[schemars(skip)]
        #[semantic_eq(skip)]
        /// Source span for error reporting.
        span: Span,
    },
    /// +//. - self-interruption
    /// Reference: <https://talkbank.org/0info/manuals/CHAT.html#Self_Interruption_Terminator>
    SelfInterruption {
        #[serde(skip)]
        #[schemars(skip)]
        #[semantic_eq(skip)]
        /// Source span for error reporting.
        span: Span,
    },
    /// +/? - interrupted question
    /// Reference: <https://talkbank.org/0info/manuals/CHAT.html#Interrupted_Question_Terminator>
    InterruptedQuestion {
        #[serde(skip)]
        #[schemars(skip)]
        #[semantic_eq(skip)]
        /// Source span for error reporting.
        span: Span,
    },
    /// +!? - broken-off question
    /// Reference: <https://talkbank.org/0info/manuals/CHAT.html#BrokenQuestion_Terminator>
    BrokenQuestion {
        #[serde(skip)]
        #[schemars(skip)]
        #[semantic_eq(skip)]
        /// Source span for error reporting.
        span: Span,
    },
    /// +"/. - SUTNL - quoted utterance, next line (quote-slash-period)
    /// Reference: <https://talkbank.org/0info/manuals/CHAT.html#QuotedNewLine_Terminator>
    #[serde(rename = "quoted_new_line")]
    QuotedNewLine {
        #[serde(skip)]
        #[schemars(skip)]
        #[semantic_eq(skip)]
        /// Source span for error reporting.
        span: Span,
    },
    /// +". - SUTQP - quoted utterance with period (quote-period, no slash)
    /// Reference: <https://talkbank.org/0info/manuals/CHAT.html#QuotedPeriod_Terminator>
    #[serde(rename = "quoted_period_simple")]
    QuotedPeriodSimple {
        #[serde(skip)]
        #[schemars(skip)]
        #[semantic_eq(skip)]
        /// Source span for error reporting.
        span: Span,
    },
    /// +//? - self-interrupted question
    /// Reference: <https://talkbank.org/0info/manuals/CHAT.html#SelfInterruptedQuestion_Terminator>
    #[serde(rename = "self_interrupted_question")]
    SelfInterruptedQuestion {
        #[serde(skip)]
        #[schemars(skip)]
        #[semantic_eq(skip)]
        /// Source span for error reporting.
        span: Span,
    },
    /// +..? - trailing off question
    /// Reference: <https://talkbank.org/0info/manuals/CHAT.html#TrailingOffQuestion_Terminator>
    #[serde(rename = "trailing_off_question")]
    TrailingOffQuestion {
        #[serde(skip)]
        #[schemars(skip)]
        #[semantic_eq(skip)]
        /// Source span for error reporting.
        span: Span,
    },
    /// +. - break for coding
    /// Reference: <https://talkbank.org/0info/manuals/CHAT.html#BreakForCoding>
    #[serde(rename = "break_for_coding")]
    BreakForCoding {
        #[serde(skip)]
        #[schemars(skip)]
        #[semantic_eq(skip)]
        /// Source span for error reporting.
        span: Span,
    },

    // ===== CA (Conversation Analysis) Intonation Terminators =====
    /// ⇗ (U+21D7) - Rising to high intonation (Conversation Analysis)
    /// Reference: <https://talkbank.org/0info/manuals/CHAT.html#RisingToHigh>
    #[serde(rename = "ca_rising_to_high")]
    CaRisingToHigh {
        #[serde(skip)]
        #[schemars(skip)]
        #[semantic_eq(skip)]
        /// Source span for error reporting.
        span: Span,
    },
    /// ↗ (U+2197) - Rising to mid intonation (Conversation Analysis)
    /// Reference: <https://talkbank.org/0info/manuals/CHAT.html#RisingToMid>
    #[serde(rename = "ca_rising_to_mid")]
    CaRisingToMid {
        #[serde(skip)]
        #[schemars(skip)]
        #[semantic_eq(skip)]
        /// Source span for error reporting.
        span: Span,
    },
    /// → (U+2192) - Level/continuing intonation (Conversation Analysis)
    /// Reference: <https://talkbank.org/0info/manuals/CHAT.html#Level_Intonation>
    #[serde(rename = "ca_level")]
    CaLevel {
        #[serde(skip)]
        #[schemars(skip)]
        #[semantic_eq(skip)]
        /// Source span for error reporting.
        span: Span,
    },
    /// ↘ (U+2198) - Falling to mid intonation (Conversation Analysis)
    /// Reference: <https://talkbank.org/0info/manuals/CHAT.html#FallingToMid>
    #[serde(rename = "ca_falling_to_mid")]
    CaFallingToMid {
        #[serde(skip)]
        #[schemars(skip)]
        #[semantic_eq(skip)]
        /// Source span for error reporting.
        span: Span,
    },
    /// ⇘ (U+21D8) - Falling to low intonation (Conversation Analysis)
    /// Reference: <https://talkbank.org/0info/manuals/CHAT.html#FallingToLow>
    #[serde(rename = "ca_falling_to_low")]
    CaFallingToLow {
        #[serde(skip)]
        #[schemars(skip)]
        #[semantic_eq(skip)]
        /// Source span for error reporting.
        span: Span,
    },
    /// ≋ (U+224B) - Technical break TCU (Turn-Constructional Unit) - Conversation Analysis
    /// Reference: <https://talkbank.org/0info/manuals/CHAT.html#TCU_Technical_Break>
    #[serde(rename = "ca_technical_break")]
    CaTechnicalBreak {
        #[serde(skip)]
        #[schemars(skip)]
        #[semantic_eq(skip)]
        /// Source span for error reporting.
        span: Span,
    },
    /// +≋ (U+224B) - Technical break TCU linker/terminator - Conversation Analysis
    /// Reference: <https://talkbank.org/0info/manuals/CHAT.html#TCU_Continuation_Linker>
    #[serde(rename = "ca_technical_break_linker")]
    CaTechnicalBreakLinker {
        #[serde(skip)]
        #[schemars(skip)]
        #[semantic_eq(skip)]
        /// Source span for error reporting.
        span: Span,
    },
    /// ≈ (U+2248) - No break TCU - Conversation Analysis
    /// Reference: <https://talkbank.org/0info/manuals/CHAT.html#TCU_NoBreak>
    #[serde(rename = "ca_no_break")]
    CaNoBreak {
        #[serde(skip)]
        #[schemars(skip)]
        #[semantic_eq(skip)]
        /// Source span for error reporting.
        span: Span,
    },
    /// +≈ (U+2248) - No break TCU linker/terminator - Conversation Analysis
    /// Reference: <https://talkbank.org/0info/manuals/CHAT.html#TCU_NoBreak_Linker>
    #[serde(rename = "ca_no_break_linker")]
    CaNoBreakLinker {
        #[serde(skip)]
        #[schemars(skip)]
        #[semantic_eq(skip)]
        /// Source span for error reporting.
        span: Span,
    },
}

impl Terminator {
    /// Returns source span metadata associated with this terminator.
    pub fn span(&self) -> Span {
        match self {
            Terminator::Period { span }
            | Terminator::Question { span }
            | Terminator::Exclamation { span }
            | Terminator::TrailingOff { span }
            | Terminator::Interruption { span }
            | Terminator::SelfInterruption { span }
            | Terminator::InterruptedQuestion { span }
            | Terminator::BrokenQuestion { span }
            | Terminator::QuotedNewLine { span }
            | Terminator::QuotedPeriodSimple { span }
            | Terminator::SelfInterruptedQuestion { span }
            | Terminator::TrailingOffQuestion { span }
            | Terminator::BreakForCoding { span }
            | Terminator::CaRisingToHigh { span }
            | Terminator::CaRisingToMid { span }
            | Terminator::CaLevel { span }
            | Terminator::CaFallingToMid { span }
            | Terminator::CaFallingToLow { span }
            | Terminator::CaTechnicalBreak { span }
            | Terminator::CaTechnicalBreakLinker { span }
            | Terminator::CaNoBreak { span }
            | Terminator::CaNoBreakLinker { span } => *span,
        }
    }
}

impl WriteChat for Terminator {
    /// Serializes the canonical CHAT token for this terminator.
    fn write_chat<W: std::fmt::Write>(&self, w: &mut W) -> std::fmt::Result {
        match self {
            Terminator::Period { .. } => w.write_char('.'),
            Terminator::Question { .. } => w.write_char('?'),
            Terminator::Exclamation { .. } => w.write_char('!'),
            Terminator::TrailingOff { .. } => w.write_str("+..."),
            Terminator::Interruption { .. } => w.write_str("+/."),
            Terminator::SelfInterruption { .. } => w.write_str("+//."),
            Terminator::InterruptedQuestion { .. } => w.write_str("+/?"),
            Terminator::BrokenQuestion { .. } => w.write_str("+!?"),
            Terminator::QuotedNewLine { .. } => w.write_str("+\"/."),
            Terminator::QuotedPeriodSimple { .. } => w.write_str("+\"."),
            Terminator::SelfInterruptedQuestion { .. } => w.write_str("+//?"),
            Terminator::TrailingOffQuestion { .. } => w.write_str("+..?"),
            Terminator::BreakForCoding { .. } => w.write_str("+."),
            // CA terminators
            Terminator::CaRisingToHigh { .. } => w.write_char('\u{21D7}'), // ⇗
            Terminator::CaRisingToMid { .. } => w.write_char('\u{2197}'),  // ↗
            Terminator::CaLevel { .. } => w.write_char('\u{2192}'),        // →
            Terminator::CaFallingToMid { .. } => w.write_char('\u{2198}'), // ↘
            Terminator::CaFallingToLow { .. } => w.write_char('\u{21D8}'), // ⇘
            Terminator::CaTechnicalBreak { .. } => w.write_char('\u{224B}'), // ≋
            Terminator::CaTechnicalBreakLinker { .. } => w.write_str("+\u{224B}"), // +≋
            Terminator::CaNoBreak { .. } => w.write_char('\u{2248}'),      // ≈
            Terminator::CaNoBreakLinker { .. } => w.write_str("+\u{2248}"), // +≈
        }
    }
}

impl std::fmt::Display for Terminator {
    /// Formats the exact CHAT token for the current terminator variant.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.write_chat(f)
    }
}
