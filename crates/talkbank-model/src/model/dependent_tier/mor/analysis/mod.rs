//! Newtypes for `%mor` analysis atoms (POS categories, stems, features).
//!
//! These wrappers keep `%mor` internals strongly typed while preserving exact
//! CHAT/CLAN payload strings. They are intentionally lightweight and are
//! composed by higher-level `%mor` word and tier structures.
//! Keeping these atom types separate from tier structs reduces accidental
//! string-mixing bugs and makes parser/serializer tests easier to localize.
//!
//! References:
//! - <https://talkbank.org/0info/manuals/CHAT.html#Morphological_Tier>

mod newtypes;

pub use newtypes::{MorFeature, MorStem, PosCategory};
