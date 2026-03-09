//! Analyze boxing options functionality for this subsystem.
//!

use std::mem::size_of;
use talkbank_model::model::annotation::*;
use talkbank_model::model::content::*;

/// Entry point for this binary target.
fn main() {
    println!("=== Boxing Analysis for UtteranceContent ===\n");

    // Current sizes
    println!("Current variant sizes:");
    println!("  ReplacedWord:      {} bytes", size_of::<ReplacedWord>());
    println!(
        "  AnnotatedWord:     {} bytes",
        size_of::<Annotated<Word>>()
    );
    println!("  Word:              {} bytes", size_of::<Word>());
    println!(
        "  AnnotatedGroup:    {} bytes",
        size_of::<Annotated<Group>>()
    );
    println!("  Group:             {} bytes", size_of::<Group>());
    println!("  Other variants:    ~32-64 bytes");

    println!(
        "\nCurrent enum size: {} bytes\n",
        size_of::<UtteranceContent>()
    );

    // Boxed sizes
    println!("=== Boxing Options ===\n");

    println!("Option 1: Box ReplacedWord only");
    println!(
        "  Box<ReplacedWord>: {} bytes",
        size_of::<Box<ReplacedWord>>()
    );
    println!("  New enum size would be: ~216 bytes (sized by AnnotatedWord)");
    println!(
        "  Memory savings: {} bytes per element ({}%)",
        248 - 216,
        ((248 - 216) as f64 / 248.0 * 100.0)
    );

    println!("\nOption 2: Box ReplacedWord + AnnotatedWord");
    println!(
        "  Box<ReplacedWord>:     {} bytes",
        size_of::<Box<ReplacedWord>>()
    );
    println!(
        "  Box<Annotated<Word>>:  {} bytes",
        size_of::<Box<Annotated<Word>>>()
    );
    println!("  New enum size would be: ~184 bytes (sized by Word)");
    println!(
        "  Memory savings: {} bytes per element ({}%)",
        248 - 184,
        ((248 - 184) as f64 / 248.0 * 100.0)
    );

    println!("\nOption 3: Box all large variants (>100 bytes)");
    println!(
        "  Box<ReplacedWord>:     {} bytes",
        size_of::<Box<ReplacedWord>>()
    );
    println!(
        "  Box<Annotated<Word>>:  {} bytes",
        size_of::<Box<Annotated<Word>>>()
    );
    println!("  Box<Word>:             {} bytes", size_of::<Box<Word>>());
    println!("  New enum size would be: ~88 bytes (sized by AnnotatedGroup)");
    println!(
        "  Memory savings: {} bytes per element ({}%)",
        248 - 88,
        ((248 - 88) as f64 / 248.0 * 100.0)
    );

    println!("\n=== Analysis ===\n");
    println!("Words are THE most common element in utterances.");
    println!("Boxing Word itself would add heap allocations to the hot path.");
    println!(
        "
Recommendation: Option 2 (Box ReplacedWord + AnnotatedWord)"
    );
    println!("  - 26% memory reduction");
    println!("  - Only adds heap allocation for less common cases");
    println!("  - Keeps plain Word on stack (hot path optimization)");

    // Check BracketedItem too
    println!("\n=== BracketedItem Analysis ===\n");
    println!("Current size: {} bytes", size_of::<BracketedItem>());
    println!("Same optimization applies - box ReplacedWord variant.");
}
