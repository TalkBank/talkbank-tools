//! JSON conversion commands (to-json, from-json).
//!
//! `chat_to_json` optionally runs validation/alignment and schema checking before
//! serializing to JSON. `json_to_chat` parses the JSON back into a `ChatFile`
//! and writes canonical CHAT text. Keeping both conversions in one module keeps
//! command-level concerns centralized.
//!
//! # Related CHAT Manual Sections
//!
//! - <https://talkbank.org/0info/manuals/CHAT.html#File_Format>
//! - <https://talkbank.org/0info/manuals/CHAT.html#File_Headers>
//! - <https://talkbank.org/0info/manuals/CHAT.html#Main_Tier>
//! - <https://talkbank.org/0info/manuals/CHAT.html#Dependent_Tiers>

use std::fs;
use std::path::PathBuf;
use talkbank_model::model::{ChatFile, WriteChat};
use tracing::{Level, debug, info, span, warn};

use crate::output::print_errors;

/// Convert the CHAT file into a JSON representation, optionally running validation/alignment.
///
/// This function reads the file, configures the pipeline options (validation + `%wor` alignment),
/// and routes through `talkbank_transform::chat_to_json` so that the resulting JSON matches the
/// structure described in the CHAT manual's File Format and Main Tier sections. Schema checks
/// mirror the CHAT manual’s requirements when not explicitly skipped, and any validation failures
/// emit the same diagnostic codes the manual discusses before exiting with a failure status.
pub fn chat_to_json(
    input: &PathBuf,
    output: Option<&PathBuf>,
    pretty: bool,
    validate: bool,
    alignment: bool,
    skip_schema_validation: bool,
) {
    let _span = span!(Level::INFO, "chat_to_json", input = %input.display()).entered();
    info!("Converting CHAT to JSON");

    // Read CHAT file
    let content = {
        let _span = span!(Level::DEBUG, "read_file").entered();
        match fs::read_to_string(input) {
            Ok(c) => {
                debug!("Read {} bytes from file", c.len());
                c
            }
            Err(e) => {
                warn!("Failed to read file: {}", e);
                eprintln!("Error reading file {:?}: {}", input, e);
                std::process::exit(1);
            }
        }
    };

    // Build pipeline options
    let mut options = talkbank_model::ParseValidateOptions::default();
    if validate {
        options = options.with_validation();
    }
    if alignment {
        options = options.with_alignment();
    }

    // Use pipeline function to parse, validate, and serialize to JSON
    // Schema validation is now integrated into the pipeline (unless skipped)
    let json = {
        let _span = span!(Level::DEBUG, "pipeline").entered();
        let result = if skip_schema_validation {
            debug!("Skipping JSON Schema validation (--skip-schema-validation)");
            talkbank_transform::chat_to_json_unvalidated(&content, options, pretty)
        } else {
            talkbank_transform::chat_to_json(&content, options, pretty)
        };
        match result {
            Ok(json_str) => {
                debug!("Pipeline successful, {} bytes", json_str.len());
                if validate || alignment {
                    info!("✓ Validation passed");
                    eprintln!("✓ Validation passed");
                }
                if !skip_schema_validation {
                    info!("✓ JSON schema validation passed");
                }
                json_str
            }
            Err(e) => {
                match e {
                    talkbank_transform::PipelineError::Validation(errors) => {
                        warn!("Validation found {} errors", errors.len());
                        eprintln!("✗ Validation errors found:");
                        print_errors(input, &content, &errors);
                    }
                    talkbank_transform::PipelineError::JsonSerialization(msg) => {
                        warn!("JSON serialization/validation error: {}", msg);
                        eprintln!("✗ JSON error: {}", msg);
                    }
                    _ => {
                        warn!("Pipeline error: {}", e);
                        eprintln!("Error: {}", e);
                    }
                }
                std::process::exit(1);
            }
        }
    };

    // Write or print JSON
    if let Some(output_path) = output {
        let _span = span!(Level::DEBUG, "write_output").entered();
        if let Err(e) = fs::write(output_path, &json) {
            warn!("Failed to write output: {}", e);
            eprintln!("Error writing JSON to {:?}: {}", output_path, e);
            std::process::exit(1);
        }
        info!("Converted {} to {}", input.display(), output_path.display());
        eprintln!(
            "✓ Converted {} to {}",
            input.display(),
            output_path.display()
        );
    } else {
        println!("{}", json);
    }
}

/// Convert a JSON representation back into canonical CHAT text.
///
/// The deserialization/serialization cycle mirrors the chat format described in the manual's
/// File Format and Dependent Tier sections, and errors bubble up so callers receive clear
/// CHAT-aligned diagnostics when the JSON is malformed or cannot be emitted.
pub fn json_to_chat(input: &PathBuf, output: Option<&PathBuf>) {
    let _span = span!(Level::INFO, "json_to_chat", input = %input.display()).entered();
    info!("Converting JSON to CHAT");

    // Read JSON file
    let content = {
        let _span = span!(Level::DEBUG, "read_file").entered();
        match fs::read_to_string(input) {
            Ok(c) => {
                debug!("Read {} bytes from file", c.len());
                c
            }
            Err(e) => {
                warn!("Failed to read file: {}", e);
                eprintln!("Error reading file {:?}: {}", input, e);
                std::process::exit(1);
            }
        }
    };

    // Deserialize JSON to ChatFile
    let chat_file: ChatFile = {
        let _span = span!(Level::DEBUG, "deserialize_json").entered();
        match serde_json::from_str(&content) {
            Ok(cf) => {
                info!("Deserialized ChatFile successfully");
                cf
            }
            Err(e) => {
                warn!("JSON parse error: {}", e);
                eprintln!("Error parsing JSON: {}", e);
                std::process::exit(1);
            }
        }
    };

    // Serialize to CHAT format
    let chat_text = {
        let _span = span!(Level::DEBUG, "serialize_to_chat").entered();
        let result = chat_file.to_chat_string();
        debug!("Serialized to {} bytes", result.len());
        result
    };

    // Write or print CHAT
    if let Some(output_path) = output {
        let _span = span!(Level::DEBUG, "write_output").entered();
        if let Err(e) = fs::write(output_path, &chat_text) {
            warn!("Failed to write output: {}", e);
            eprintln!("Error writing CHAT to {:?}: {}", output_path, e);
            std::process::exit(1);
        }
        info!("Converted {} to {}", input.display(), output_path.display());
        eprintln!(
            "✓ Converted {} to {}",
            input.display(),
            output_path.display()
        );
    } else {
        print!("{}", chat_text);
    }
}
