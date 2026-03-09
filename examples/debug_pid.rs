//! Debug pid functionality for this subsystem.
//!

use std::error::Error;
use talkbank_parser::TreeSitterParser;

/// Entry point for this binary target.
fn main() -> Result<(), Box<dyn Error>> {
    let input = "@UTF8\n@PID:\t11312/c-00016447-1\n@Begin\n@Languages:\teng\n@Participants:\tCHI Child\n@ID:\teng|corpus|CHI|||||Child|||\n@End\n";

    println!("=== Input ===");
    println!("{}", input);

    let parser = TreeSitterParser::new()?;
    match parser.parse_chat_file(input) {
        Ok(chat_file) => {
            println!("\n=== Headers ===");
            for (i, header) in chat_file.headers().enumerate() {
                println!("{}: {}", i, header.name());
            }
        }
        Err(errors) => {
            println!("\n=== Parse Errors ===");
            for error in &errors.errors {
                println!("[{}] {}", error.code.as_str(), error.message);
            }
        }
    }

    Ok(())
}
