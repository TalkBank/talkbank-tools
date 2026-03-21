//! Bootstrap spec files from existing corpus
//!
//! Parses corpus/reference/ using TreeSitterParser and extracts
//! examples at each construct level to generate initial spec files.

use clap::Parser;
use std::collections::HashMap;
use std::path::PathBuf;
use talkbank_model::ErrorCollector;
use talkbank_model::{ChatFile, Word, WordCategory, WordContent};
use talkbank_parser::TreeSitterParser;
use walkdir::WalkDir;

/// CLI arguments: corpus path, output directory, max examples per category, and words-only filter.
#[derive(Parser)]
#[command(name = "bootstrap")]
#[command(about = "Bootstrap specifications from corpus")]
struct Args {
    /// Path to corpus/reference directory
    #[arg(short, long, default_value = "corpus/reference")]
    corpus: PathBuf,

    /// Output directory for generated specs
    #[arg(short, long, default_value = "spec")]
    output: PathBuf,

    /// Maximum examples per category
    #[arg(short, long, default_value = "50")]
    max_examples: usize,

    /// Only extract word-level examples
    #[arg(long)]
    words_only: bool,
}

/// Parses the reference corpus and extracts word-level examples to bootstrap initial spec files.
fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    println!("Bootstrapping from corpus: {}", args.corpus.display());
    println!("Output directory: {}", args.output.display());
    println!();

    let parser =
        TreeSitterParser::new().map_err(|e| anyhow::anyhow!("Failed to create parser: {}", e))?;
    let mut word_patterns: HashMap<String, Vec<WordExample>> = HashMap::new();
    let mut file_count = 0;
    let mut error_count = 0;

    // Walk corpus directory
    for entry in WalkDir::new(&args.corpus)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().is_some_and(|ext| ext == "cha"))
    {
        file_count += 1;
        let path = entry.path();

        if file_count % 10 == 0 {
            print!("\rProcessed {} files...", file_count);
        }

        // Read and parse file
        let content = match std::fs::read_to_string(path) {
            Ok(c) => c,
            Err(e) => {
                eprintln!(
                    "
Warning: Failed to read {}: {}",
                    path.display(),
                    e
                );
                continue;
            }
        };

        let errors = ErrorCollector::new();
        let chat_file = parser.parse_chat_file_streaming(&content, &errors);

        // Skip if there were parse errors
        if !errors.into_vec().is_empty() {
            error_count += 1;
            continue;
        }

        // Extract word examples
        extract_word_examples(&chat_file, &mut word_patterns, args.max_examples);
    }

    println!(
        "\n
Processed {} files ({} with errors)",
        file_count, error_count
    );
    println!();

    // Generate word specs
    generate_word_specs(&word_patterns, &args.output)?;

    println!("\n✓ Bootstrap complete");
    println!("\nNext steps:");
    println!(
        "  1. Review generated spec files in {}",
        args.output.display()
    );
    println!("  2. Manually curate and organize examples");
    println!("  3. Add descriptions and expected CST trees");
    println!("  4. Run: cargo run --bin validate_spec");

    Ok(())
}

/// Extract word-level examples
fn extract_word_examples(
    chat_file: &ChatFile,
    patterns: &mut HashMap<String, Vec<WordExample>>,
    max_per_category: usize,
) {
    for utterance in chat_file.utterances() {
        for content in &utterance.main.content.content.0 {
            if let talkbank_model::UtteranceContent::Word(word) = content {
                let pattern = classify_word(word);
                let entry = patterns.entry(pattern.clone()).or_default();

                // Skip if we have enough examples
                if entry.len() >= max_per_category {
                    continue;
                }

                // Check for duplicates by raw text
                if entry.iter().any(|e| e.raw_text == word.raw_text()) {
                    continue;
                }

                entry.push(WordExample {
                    raw_text: word.raw_text().to_string(),
                    pattern,
                });
            }
        }
    }
}

/// Classify a word into a pattern category
fn classify_word(word: &Word) -> String {
    if word.form_type.is_some() {
        return "special_forms".to_string();
    }

    if word.lang.is_some() {
        return "special_forms".to_string();
    }

    if word.untranscribed().is_some() {
        return "special_forms".to_string();
    }

    if let Some(category) = &word.category {
        if category.is_omission() {
            return "shortenings".to_string();
        }
        match category {
            WordCategory::Nonword | WordCategory::Filler | WordCategory::PhonologicalFragment => {
                return "special_forms".to_string();
            }
            WordCategory::CAOmission | WordCategory::Omission => {
                return "shortenings".to_string();
            }
        }
    }

    if word_has_content(word, |content| {
        matches!(content, WordContent::CompoundMarker(_))
    }) {
        return "compounds".to_string();
    }

    if word_has_content(word, |content| {
        matches!(content, WordContent::Lengthening(_))
    }) {
        return "lengthening".to_string();
    }

    if word_has_content(word, |content| {
        matches!(
            content,
            WordContent::CAElement(_) | WordContent::CADelimiter(_)
        )
    }) {
        return "ca_markers".to_string();
    }

    if word_has_content(word, |content| {
        matches!(
            content,
            WordContent::StressMarker(_) | WordContent::SyllablePause(_)
        )
    }) {
        return "phonology".to_string();
    }

    if word_has_content(word, |content| {
        matches!(content, WordContent::Shortening(_))
    }) {
        return "shortenings".to_string();
    }

    "basic".to_string()
}

fn word_has_content<F>(word: &Word, predicate: F) -> bool
where
    F: Fn(&WordContent) -> bool,
{
    word.content.iter().any(predicate)
}

/// Generate word specification files in Markdown format
fn generate_word_specs(
    patterns: &HashMap<String, Vec<WordExample>>,
    output_dir: &std::path::Path,
) -> anyhow::Result<()> {
    let word_root = output_dir.join("constructs/word");
    std::fs::create_dir_all(&word_root)?;

    println!("Generating word specifications (Markdown):");

    for (category, examples) in patterns {
        if examples.is_empty() {
            continue;
        }

        let category_dir = word_root.join(category);
        std::fs::create_dir_all(&category_dir)?;

        // Take up to 20 examples
        let sample: Vec<_> = examples.iter().take(20).collect();

        for (i, example) in sample.iter().enumerate() {
            let name = sanitize_name(&example.raw_text, i);
            let filename = category_dir.join(format!("{}.md", name));

            let mut content = String::new();
            content.push_str(&format!("# {}\n\n", name));
            content.push_str("TODO: Add description\n\n");
            content.push_str("## Input\n\n");
            content.push_str("```standalone_word\n");
            content.push_str(&example.raw_text);
            content.push_str("\n```\n\n");
            content.push_str("## Expected CST\n\n");
            content.push_str("```cst\n");
            content.push_str("TODO: Add expected CST tree\n");
            content.push_str("```\n\n");
            content.push_str("## Metadata\n\n");
            content.push_str("- **Level**: word\n");
            content.push_str(&format!("- **Category**: {}\n", category));

            std::fs::write(&filename, content)?;
        }

        println!("  ✓ {} ({} examples)", category, sample.len());
    }

    println!();
    println!("Total: {} categories", patterns.len());

    Ok(())
}

#[derive(Debug, Clone)]
struct WordExample {
    raw_text: String,
    #[allow(dead_code)]
    pattern: String,
}

#[allow(dead_code)]
fn capitalize(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        None => String::new(),
        Some(first) => first.to_uppercase().chain(chars).collect(),
    }
}

fn sanitize_name(text: &str, index: usize) -> String {
    let clean: String = text
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == '_')
        .collect();

    if clean.is_empty() {
        format!("example_{}", index)
    } else {
        format!("{}_{}", clean, index)
    }
}
