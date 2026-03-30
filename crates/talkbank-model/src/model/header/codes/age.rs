//! Typed age value for `@ID` header field 4.
//!
//! Format: `years;months.days` (e.g., `3;06.15`, `2;08`, `1;04.`).
//!
//! Reference: <https://talkbank.org/0info/manuals/CHAT.html#Age_Field>

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use smol_str::SmolStr;
use talkbank_derive::{SemanticEq, SpanShift, ValidationTagged};

/// Age string recorded in `@ID` (field 4, format `years;months.days`).
///
/// Successfully parsed ages store typed numeric components; malformed ages
/// are preserved as `Unsupported` so the validator can report actionable errors.
///
/// Reference: <https://talkbank.org/0info/manuals/CHAT.html#Age_Field>
#[derive(Debug, Clone, PartialEq, Eq, Hash, SemanticEq, SpanShift, ValidationTagged)]
pub enum AgeValue {
    /// Successfully parsed age.
    Valid {
        /// Years component.
        #[span_shift(skip)]
        years: u16,
        /// Months component (0–11).
        #[span_shift(skip)]
        months: Option<u8>,
        /// Days component (0–30).
        #[span_shift(skip)]
        days: Option<u8>,
        /// Original text preserved for exact roundtrip.
        #[semantic_eq(skip)]
        #[span_shift(skip)]
        raw: SmolStr,
    },
    /// Unrecognized value preserved for validation.
    Unsupported(String),
}

impl AgeValue {
    /// Parse a CHAT age string (`years;months.days`).
    ///
    /// Returns `Valid` for well-formed ages, `Unsupported` otherwise.
    pub fn from_text(value: &str) -> Self {
        let Some((years_str, rest)) = value.split_once(';') else {
            return Self::Unsupported(value.to_string());
        };

        if years_str.is_empty() || !years_str.bytes().all(|b| b.is_ascii_digit()) {
            return Self::Unsupported(value.to_string());
        }

        let Ok(years) = years_str.parse::<u16>() else {
            return Self::Unsupported(value.to_string());
        };

        let (months, days) = if let Some((months_str, days_str)) = rest.split_once('.') {
            let months = if months_str.is_empty() {
                None
            } else if months_str.bytes().all(|b| b.is_ascii_digit()) {
                Some(months_str.parse::<u8>().unwrap_or(0))
            } else {
                return Self::Unsupported(value.to_string());
            };

            let days = if days_str.is_empty() {
                None
            } else if days_str.bytes().all(|b| b.is_ascii_digit()) {
                Some(days_str.parse::<u8>().unwrap_or(0))
            } else {
                return Self::Unsupported(value.to_string());
            };

            (months, days)
        } else if rest.is_empty() {
            (None, None)
        } else if rest.bytes().all(|b| b.is_ascii_digit()) {
            (Some(rest.parse::<u8>().unwrap_or(0)), None)
        } else {
            return Self::Unsupported(value.to_string());
        };

        Self::Valid {
            years,
            months,
            days,
            raw: SmolStr::from(value),
        }
    }

    /// Returns the age as a string.
    ///
    /// Returns the original text for both valid and unsupported ages.
    pub fn as_str(&self) -> &str {
        match self {
            Self::Valid { raw, .. } => raw.as_str(),
            Self::Unsupported(s) => s.as_str(),
        }
    }

    /// Returns true if months or days are single-digit without zero-padding.
    ///
    /// CLAN CHECK error 153: "Age's month or day are missing initial zero."
    ///
    /// CLAN only checks this when the age contains a period (`.`), meaning
    /// the days component is present. Without a period, single-digit months
    /// like `2;6` are accepted. With a period, both month and day must be
    /// two digits: `1;8.` → `1;08.`, `3;0.5` → `3;00.05`.
    pub fn needs_zero_padding(&self) -> bool {
        match self {
            Self::Valid { raw, .. } => {
                let Some((_years, rest)) = raw.split_once(';') else {
                    return false;
                };
                // Only check when period is present (days component exists)
                let Some((months_str, days_str)) = rest.split_once('.') else {
                    return false;
                };
                // Month needs padding if single digit (0-9)
                if months_str.len() == 1 && months_str.as_bytes()[0].is_ascii_digit() {
                    return true;
                }
                // Day needs padding if non-empty single digit (0-9)
                if days_str.len() == 1 && days_str.as_bytes()[0].is_ascii_digit() {
                    return true;
                }
                false
            }
            Self::Unsupported(_) => false,
        }
    }

    /// Backward-compatible constructor matching the old `string_newtype` API.
    pub fn new(value: impl AsRef<str>) -> Self {
        Self::from_text(value.as_ref())
    }
}

impl std::fmt::Display for AgeValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl crate::model::WriteChat for AgeValue {
    fn write_chat<W: std::fmt::Write>(&self, w: &mut W) -> std::fmt::Result {
        w.write_str(self.as_str())
    }
}

impl Serialize for AgeValue {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for AgeValue {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        Ok(Self::from_text(&s))
    }
}

impl JsonSchema for AgeValue {
    fn schema_name() -> std::borrow::Cow<'static, str> {
        "AgeValue".into()
    }

    fn json_schema(_generator: &mut schemars::SchemaGenerator) -> schemars::Schema {
        schemars::json_schema!({ "type": "string" })
    }
}

impl From<String> for AgeValue {
    fn from(value: String) -> Self {
        Self::from_text(&value)
    }
}

impl From<&str> for AgeValue {
    fn from(value: &str) -> Self {
        Self::from_text(value)
    }
}
