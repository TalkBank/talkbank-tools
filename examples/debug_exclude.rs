//! Debug exclude functionality for this subsystem.
//!

use std::error::Error;
use talkbank_parser::TreeSitterParser;

/// Entry point for this binary target.
fn main() -> Result<(), Box<dyn Error>> {
    let parser = TreeSitterParser::new()?;

    let test_line = "*CHI:\tthis is a mor [e] exclude .";

    println!("=== Testing: {}", test_line);
    match parser.parse_main_tier(test_line) {
        Ok(main_tier) => {
            println!("✓ Parsed successfully");
            println!("  Speaker: {}", main_tier.speaker);
            println!("  Content items: {}", main_tier.content.content.len());

            let serialized = main_tier.to_chat();
            println!("  Serialized: {}", serialized);

            if serialized != test_line {
                println!("  ⚠️  Not equal to input!");
            }
        }
        Err(errors) => {
            println!("✗ Parse failed:");
            println!("  - {}", errors);
        }
    }

    Ok(())
}
