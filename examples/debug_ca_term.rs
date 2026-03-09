//! Debug ca term functionality for this subsystem.
//!

use std::error::Error;
use talkbank_parser::TreeSitterParser;

/// Entry point for this binary target.
fn main() -> Result<(), Box<dyn Error>> {
    let parser = TreeSitterParser::new()?;

    // Test with space before terminator (canonical form)
    let test_line = "*M2:\tmet him → \u{15}94531_95651\u{15}";

    println!("=== Testing: {:?}", test_line);
    println!("Hex: {:02X?}", test_line.as_bytes());

    match parser.parse_main_tier(test_line) {
        Ok(main_tier) => {
            println!("✓ Parsed successfully");
            println!("  Terminator: {:?}", main_tier.content.terminator);
            println!("  Bullet: {:?}", main_tier.content.bullet);

            let serialized = main_tier.to_chat();
            println!("  Serialized: {:?}", serialized);
            println!("  Hex: {:02X?}", serialized.as_bytes());

            if serialized != test_line {
                println!("  ⚠️  Not equal to input!");
            }
        }
        Err(errors) => {
            println!("✗ Parse failed:");
            println!("  {}", errors);
        }
    }

    Ok(())
}
