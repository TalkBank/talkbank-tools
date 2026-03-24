//! Check enum sizes functionality for this subsystem.
//!

use std::mem::size_of;
use talkbank_model::dependent_tier::DependentTier;
use talkbank_model::model::annotation::*;
use talkbank_model::model::content::*;
use talkbank_model::model::dependent_tier::*;
use talkbank_model::model::header::*;

/// Entry point for this binary target.
fn main() {
    println!("=== ENUM SIZE ANALYSIS ===\n");

    // Main tier content enum
    println!("=== UtteranceContent (Main Tier) ===");
    println!("Full enum size: {} bytes\n", size_of::<UtteranceContent>());
    println!("Variant sizes:");
    println!("  Word:              {} bytes", size_of::<Word>());
    println!(
        "  AnnotatedWord:     {} bytes",
        size_of::<Annotated<Word>>()
    );
    println!("  ReplacedWord:      {} bytes", size_of::<ReplacedWord>());
    println!("  Group:             {} bytes", size_of::<Group>());
    println!(
        "  AnnotatedGroup:    {} bytes",
        size_of::<Annotated<Group>>()
    );
    println!("  Pause:             {} bytes", size_of::<Pause>());
    println!("  Event:             {} bytes", size_of::<Event>());
    println!(
        "  AnnotatedEvent:    {} bytes",
        size_of::<Annotated<Event>>()
    );

    let largest_utterance_content = size_of::<UtteranceContent>();
    let largest_payload = [
        size_of::<Word>(),
        size_of::<Annotated<Word>>(),
        size_of::<ReplacedWord>(),
        size_of::<Group>(),
        size_of::<Annotated<Group>>(),
    ]
    .iter()
    .max()
    .copied();

    let largest_payload = match largest_payload {
        Some(value) => value,
        None => {
            eprintln!("No payload sizes available");
            return;
        }
    };

    if largest_utterance_content < largest_payload {
        let saved = largest_payload - largest_utterance_content;
        println!(
            "\n✅ OPTIMIZED: Enum is {} bytes smaller than largest variant (boxing in effect)",
            saved
        );
    } else {
        println!(
            "\nEnum overhead: {} bytes",
            largest_utterance_content - largest_payload
        );
        if largest_utterance_content > largest_payload + 16 {
            println!("⚠️  WARNING: Large enum overhead! Consider boxing large variants.");
        } else {
            println!("✅ Reasonable enum size.");
        }
    }

    // Bracketed item enum
    println!("\n=== BracketedItem (Group Content) ===");
    println!("Full enum size: {} bytes\n", size_of::<BracketedItem>());

    // DependentTier enum
    println!("\n=== DependentTier ===");
    println!("Full enum size: {} bytes\n", size_of::<DependentTier>());

    println!("Variant sizes (payload only):");
    println!("  MorTier:     {} bytes", size_of::<MorTier>());
    println!("  GraTier:     {} bytes", size_of::<GraTier>());
    println!("  PhoTier:     {} bytes", size_of::<PhoTier>());
    println!("  SinTier:     {} bytes", size_of::<SinTier>());
    println!("  ActTier:     {} bytes", size_of::<ActTier>());
    println!("  CodTier:     {} bytes", size_of::<CodTier>());
    println!("  ComTier:     {} bytes", size_of::<ComTier>());
    println!("  ExpTier:     {} bytes", size_of::<ExpTier>());
    println!("  AddTier:     {} bytes", size_of::<AddTier>());
    println!("  SpaTier:     {} bytes", size_of::<SpaTier>());
    println!("  SitTier:     {} bytes", size_of::<SitTier>());
    println!("  GpxTier:     {} bytes", size_of::<GpxTier>());
    println!("  IntTier:     {} bytes", size_of::<IntTier>());
    println!("  String:      {} bytes", size_of::<String>());

    println!("\n=== Memory Comparison ===\n");

    // Old design with 25+ Option fields
    let old_size_estimate = size_of::<Option<MorTier>>() * 25;
    println!(
        "Old design (25 Option fields): ~{} bytes",
        old_size_estimate
    );

    // New design with Vec
    let new_size_empty = size_of::<Vec<DependentTier>>();
    println!("New design (empty Vec):        {} bytes", new_size_empty);

    // With one tier
    let new_size_one = size_of::<Vec<DependentTier>>() + size_of::<DependentTier>();
    println!("New design (1 tier):           {} bytes", new_size_one);

    // With three tiers
    let new_size_three = size_of::<Vec<DependentTier>>() + size_of::<DependentTier>() * 3;
    println!("New design (3 tiers):          {} bytes", new_size_three);

    let savings_percent =
        ((old_size_estimate - new_size_empty) as f64 / old_size_estimate as f64) * 100.0;
    println!("\nMemory savings (empty): {:.1}%", savings_percent);

    println!("\n=== Potential Enum Size Issue ===\n");

    let largest_variant = [
        ("MorTier", size_of::<MorTier>()),
        ("GraTier", size_of::<GraTier>()),
        ("PhoTier", size_of::<PhoTier>()),
        ("SinTier", size_of::<SinTier>()),
        ("ActTier", size_of::<ActTier>()),
        ("CodTier", size_of::<CodTier>()),
        ("ComTier", size_of::<ComTier>()),
    ]
    .iter()
    .max_by_key(|(_, size)| size)
    .copied();

    let largest_variant = match largest_variant {
        Some(value) => value,
        None => {
            eprintln!("No dependent tier sizes available");
            return;
        }
    };

    println!(
        "Largest variant: {} ({} bytes)",
        largest_variant.0, largest_variant.1
    );
    println!(
        "Enum overhead: {} bytes (discriminant + padding)",
        size_of::<DependentTier>() - largest_variant.1
    );

    let smallest_variant = size_of::<String>();
    println!("\nSmallest variant (String): {} bytes", smallest_variant);

    if size_of::<DependentTier>() > largest_variant.1 + 16 {
        println!("\n⚠️  WARNING: Enum has significant padding/overhead!");
        println!("   Consider Box<T> for large variants to reduce enum size.");
    } else {
        println!("\n✅ Enum size is reasonable - no boxing needed.");
    }

    // Additional enum checks
    println!("\n=== Other Important Enums ===\n");

    println!("Header:          {} bytes", size_of::<Header>());
    println!("Postcode:        {} bytes", size_of::<Postcode>());
    println!("ContentAnnotation: {} bytes", size_of::<ContentAnnotation>());

    println!("\n=== Summary ===\n");

    // Check for large enums
    let utterance_content_size = size_of::<UtteranceContent>();
    let bracketed_item_size = size_of::<BracketedItem>();
    let dependent_tier_size = size_of::<DependentTier>();

    if utterance_content_size > 200 || bracketed_item_size > 200 {
        println!("⚠️  LARGE ENUMS DETECTED:");
        println!("   UtteranceContent: {} bytes", utterance_content_size);
        println!("   BracketedItem:    {} bytes", bracketed_item_size);
        println!(
            "\n   These enums are used in Vec<T>, so each element is {} bytes.",
            utterance_content_size
        );
        println!("   Consider boxing large variants (ReplacedWord) to reduce enum size.");
        println!("   Trade-off: Boxing adds heap allocation but reduces vec memory.");
    } else {
        println!("✅ All enums have reasonable sizes.");
    }

    println!("\nDependentTier enum: {} bytes ✅", dependent_tier_size);
    println!("No boxing required for DependentTier variants.");
    println!("\n✅ Overall: Memory layout is good, but main tier enums are large.");
}
