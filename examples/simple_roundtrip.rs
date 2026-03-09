//! Simple roundtrip functionality for this subsystem.
//!

use std::error::Error;
use std::io::{self, Read};

/// Entry point for this binary target.
fn main() -> Result<(), Box<dyn Error>> {
    let mut input = String::new();
    io::stdin().read_to_string(&mut input)?;

    let chat_file = talkbank_parser::parse_chat_file(&input)?;
    let output = chat_file.to_chat();
    print!("{}", output);

    Ok(())
}
