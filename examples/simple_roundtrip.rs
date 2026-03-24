//! Simple roundtrip: read CHAT from stdin, parse, serialize back to stdout.

use std::error::Error;
use std::io::{self, Read};
use talkbank_parser::TreeSitterParser;

fn main() -> Result<(), Box<dyn Error>> {
    let mut input = String::new();
    io::stdin().read_to_string(&mut input)?;

    let parser = TreeSitterParser::new()?;
    let chat_file = parser.parse_chat_file(&input)?;
    let output = chat_file.to_chat();
    print!("{}", output);

    Ok(())
}
