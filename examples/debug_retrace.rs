//! Debug retrace functionality for this subsystem.
//!

use std::error::Error;
use talkbank_parser::TreeSitterParser;

/// Entry point for this binary target.
fn main() -> Result<(), Box<dyn Error>> {
    let parser = TreeSitterParser::new()?;

    let test_lines = vec![
        "*CHI:\t<off a> [/] off a lorry .",
        "*CHI:\t<off a> [//] concrete girders .",
    ];

    for line in &test_lines {
        println!("\n=== Testing: {}", line);
        match parser.parse_main_tier(line) {
            Ok(main_tier) => {
                println!("✓ Parsed successfully");
                println!("  Speaker: {}", main_tier.speaker);
                println!("  Content items: {}", main_tier.content.content.len());

                let serialized = main_tier.to_chat();
                println!("  Serialized: {}", serialized);

                if serialized != *line {
                    println!("  ⚠️  Not equal to input!");
                }
            }
            Err(errors) => {
                println!("✗ Parse failed:");
                println!("  - {}", errors);
            }
        }
    }

    Ok(())
}
