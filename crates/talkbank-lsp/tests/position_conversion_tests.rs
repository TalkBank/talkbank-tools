//! Integration tests for byte-offset and LSP position conversion helpers.

mod position_conversion;

mod ascii {
    include!("position_conversion/ascii.rs");
}

mod unicode {
    include!("position_conversion/unicode.rs");
}

mod bounds {
    include!("position_conversion/bounds.rs");
}

mod roundtrip {
    include!("position_conversion/roundtrip.rs");
}

mod chat {
    include!("position_conversion/chat.rs");
}
