use std::env;
use std::path::Path;
use std::process::Command;

/// Source of the re2c lexer.
const LEXER_SOURCE: &str = "src/lexer.re";

/// Generated Rust file from re2c.
const LEXER_RUST_FILE: &str = "lexer.rs";

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Re-run if the lexer definition changes.
    println!("cargo:rerun-if-changed={LEXER_SOURCE}");

    let out_dir = env::var("OUT_DIR").expect("OUT_DIR not set by Cargo");
    let out_dir_path = Path::new(&out_dir);
    let out_path = out_dir_path.join(LEXER_RUST_FILE);

    // Execute re2rust to generate the lexer.
    let output = Command::new("re2rust")
        .args([
            "-W",
            "--input-encoding",
            "utf8",
            "--utf8",
            "--conditions",
            "-o",
            out_path
                .to_str()
                .ok_or("Failed to convert out_path to str")?,
            LEXER_SOURCE,
        ])
        .output()?;

    if !output.status.success() {
        panic!(
            "re2rust failed:\n{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    Ok(())
}
