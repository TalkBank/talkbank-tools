//! Debug headers functionality for this subsystem.
//!

use std::error::Error;
use talkbank_model::model::Line;
use talkbank_parser::TreeSitterParser;

/// Entry point for this binary target.
fn main() -> Result<(), Box<dyn Error>> {
    let input = r#"@UTF8
@Begin
@Languages:	eng
@Participants:	CHI Child
@ID:	eng|corpus|CHI|||||Child|||
*CHI:	hello .
*FAT:	hi there .
@End
"#;

    println!("=== Parsing Input ===");
    println!("{}", input);

    let parser = TreeSitterParser::new()?;
    match parser.parse_chat_file(input) {
        Ok(chat_file) => {
            println!("\n=== Parse Successful ===");
            println!("Total lines: {}", chat_file.lines.len());

            println!("\n=== Headers ===");
            for (i, line) in chat_file.lines.iter().enumerate() {
                match line {
                    Line::Header { header, .. } => {
                        println!("Line {}: {:?}", i, header);
                    }
                    Line::Utterance(utt) => {
                        println!("Line {}: Utterance from {:?}", i, utt.main.speaker);
                    }
                }
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
