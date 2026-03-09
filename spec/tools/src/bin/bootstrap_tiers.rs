//! Bootstrap tier-level construct examples from reference corpus in Markdown format
//!
//! Extracts examples of dependent tiers (%mor, %gra, %pho, etc.) from
//! the reference corpus to create tier-level construct specifications.

use anyhow::Result;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

use talkbank_model::model::dependent_tier::{DependentTier, GraTier, MorTier, PhoTier};
use talkbank_model::model::{MainTier, WriteChat};
use talkbank_parser::TreeSitterParser;

/// Extracts dependent tier examples (%mor, %gra, %pho, etc.) from the reference corpus to create tier-level specs.
fn main() -> Result<()> {
    let corpus_dir = PathBuf::from(match std::env::args().nth(1) {
        Some(path) => path,
        None => "corpus/reference".to_string(),
    });
    let output_dir = PathBuf::from("spec/constructs/tiers");

    println!(
        "Extracting tier examples (Markdown) from: {}",
        corpus_dir.display()
    );
    println!("Output directory: {}", output_dir.display());
    println!();

    // Collect tier examples
    let mut tier_examples = TierExamples::new();

    let parser = TreeSitterParser::new()?;

    for entry in WalkDir::new(&corpus_dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().is_some_and(|ext| ext == "cha"))
    {
        let path = entry.path().to_path_buf();
        let content = match fs::read_to_string(&path) {
            Ok(content) => content,
            Err(err) => {
                eprintln!("Warning: Failed to read {}: {}", path.display(), err);
                continue;
            }
        };
        match parser.parse_chat_file(&content) {
            Ok(chat_file) => tier_examples.extract_from_chat_file(&chat_file, &path),
            Err(errs) => {
                eprintln!("Warning: Failed to parse {}: {}", path.display(), errs);
            }
        }
    }

    println!("Extraction complete:");
    tier_examples.print_summary();
    println!();

    // Generate Markdown specs
    fs::create_dir_all(&output_dir)?;

    generate_markdown_specs(&tier_examples.mor_examples, "mor", &output_dir)?;
    generate_markdown_specs(&tier_examples.gra_examples, "gra", &output_dir)?;
    generate_markdown_specs(&tier_examples.pho_examples, "pho", &output_dir)?;

    for (tier_name, examples) in &tier_examples.text_tier_examples {
        generate_markdown_specs(examples, tier_name, &output_dir)?;
    }

    println!("\n✓ Bootstrap complete");
    Ok(())
}

struct TierExamples {
    mor_examples: Vec<TierExample>,
    gra_examples: Vec<TierExample>,
    pho_examples: Vec<TierExample>,
    text_tier_examples: HashMap<String, Vec<TierExample>>,
    seen_patterns: HashSet<String>,
}

struct TierExample {
    name: String,
    description: String,
    input: String,
    source_file: PathBuf,
}

impl TierExamples {
    fn new() -> Self {
        Self {
            mor_examples: Vec::new(),
            gra_examples: Vec::new(),
            pho_examples: Vec::new(),
            text_tier_examples: HashMap::new(),
            seen_patterns: HashSet::new(),
        }
    }

    fn extract_from_chat_file(&mut self, chat_file: &talkbank_model::model::ChatFile, path: &Path) {
        for utterance in chat_file.utterances() {
            let main = &utterance.main;

            if let Some(mor_tier) = utterance.mor() {
                if mor_tier.is_mor() {
                    self.extract_mor_example(main, &mor_tier, path);
                }
            }
            if let Some(gra_tier) = utterance.gra() {
                if gra_tier.is_gra() {
                    if let Some(mor_tier) = utterance.mor() {
                        self.extract_gra_example(main, &mor_tier, &gra_tier, path);
                    }
                }
            }
            if let Some(pho_tier) = utterance.pho() {
                if pho_tier.is_pho() {
                    self.extract_pho_example(main, &pho_tier, path);
                }
            }
            // Handle other DependentTier types as before
            for tier in &utterance.dependent_tiers {
                match tier {
                    DependentTier::Com(_)
                    | DependentTier::Exp(_)
                    | DependentTier::Spa(_)
                    | DependentTier::Sit(_)
                    | DependentTier::Add(_)
                    | DependentTier::Eng(_)
                    | DependentTier::UserDefined(_) => {
                        self.extract_text_tier_example(main, tier, path);
                    }
                    _ => {}
                }
            }
        }
    }

    fn extract_mor_example(&mut self, main: &MainTier, mor: &MorTier, path: &Path) {
        if self.mor_examples.len() >= 15 {
            return;
        }
        let pattern = extract_mor_pattern(mor);
        if self.seen_patterns.contains(&pattern) {
            return;
        }
        self.seen_patterns.insert(pattern.clone());
        let input = match create_minimal_file(main, &[DependentTier::Mor(mor.clone())]) {
            Ok(input) => input,
            Err(err) => {
                eprintln!(
                    "Warning: Failed to render %mor example from {}: {}",
                    path.display(),
                    err
                );
                return;
            }
        };
        self.mor_examples.push(TierExample {
            name: format!("mor_example_{}", self.mor_examples.len() + 1),
            description: format!("Example %mor tier with pattern: {}", pattern),
            input,
            source_file: path.to_path_buf(),
        });
    }

    fn extract_gra_example(&mut self, main: &MainTier, mor: &MorTier, gra: &GraTier, path: &Path) {
        if self.gra_examples.len() >= 15 {
            return;
        }
        let pattern = extract_gra_pattern(gra);
        if self.seen_patterns.contains(&pattern) {
            return;
        }
        self.seen_patterns.insert(pattern.clone());
        let input = match create_minimal_file(
            main,
            &[
                DependentTier::Mor(mor.clone()),
                DependentTier::Gra(gra.clone()),
            ],
        ) {
            Ok(input) => input,
            Err(err) => {
                eprintln!(
                    "Warning: Failed to render %gra example from {}: {}",
                    path.display(),
                    err
                );
                return;
            }
        };
        self.gra_examples.push(TierExample {
            name: format!("gra_example_{}", self.gra_examples.len() + 1),
            description: format!("Example %gra tier with pattern: {}", pattern),
            input,
            source_file: path.to_path_buf(),
        });
    }

    fn extract_pho_example(&mut self, main: &MainTier, pho: &PhoTier, path: &Path) {
        if self.pho_examples.len() >= 10 {
            return;
        }
        let pattern = extract_pho_pattern(pho);
        if self.seen_patterns.contains(&pattern) {
            return;
        }
        self.seen_patterns.insert(pattern.clone());
        let input = match create_minimal_file(main, &[DependentTier::Pho(pho.clone())]) {
            Ok(input) => input,
            Err(err) => {
                eprintln!(
                    "Warning: Failed to render %pho example from {}: {}",
                    path.display(),
                    err
                );
                return;
            }
        };
        self.pho_examples.push(TierExample {
            name: ex_name("pho", self.pho_examples.len()),
            description: format!("Example %pho tier: {}", pattern),
            input,
            source_file: path.to_path_buf(),
        });
    }

    fn extract_text_tier_example(&mut self, main: &MainTier, tier: &DependentTier, path: &Path) {
        let tier_name = match text_tier_name(tier) {
            Some(name) => name,
            None => return,
        };
        let examples = self
            .text_tier_examples
            .entry(tier_name.to_string())
            .or_default();
        if examples.len() >= 5 {
            return;
        }
        let input = match create_minimal_file(main, std::slice::from_ref(tier)) {
            Ok(input) => input,
            Err(err) => {
                eprintln!(
                    "Warning: Failed to render {} example from {}: {}",
                    tier_name,
                    path.display(),
                    err
                );
                return;
            }
        };
        examples.push(TierExample {
            name: ex_name(tier_name, examples.len()),
            description: format!("Example {} tier", tier_name),
            input,
            source_file: path.to_path_buf(),
        });
    }

    fn print_summary(&self) {
        println!(
            "  %mor: {}, %gra: {}, %pho: {}",
            self.mor_examples.len(),
            self.gra_examples.len(),
            self.pho_examples.len()
        );
        for (tier, examples) in &self.text_tier_examples {
            println!("    {}: {}", tier, examples.len());
        }
    }
}

fn ex_name(prefix: &str, count: usize) -> String {
    format!("{}_example_{}", prefix, count + 1)
}

fn text_tier_name(tier: &DependentTier) -> Option<&'static str> {
    match tier {
        DependentTier::Com(_) => Some("com"),
        DependentTier::Exp(_) => Some("exp"),
        DependentTier::Spa(_) => Some("spa"),
        DependentTier::Eng(_) => Some("eng"),
        DependentTier::Sit(_) => Some("sit"),
        DependentTier::Add(_) => Some("add"),
        DependentTier::UserDefined(tier) => {
            let label = tier.label.as_str();
            if label == "xcom" {
                Some("xcom")
            } else {
                None
            }
        }
        _ => None,
    }
}

fn extract_mor_pattern(mor: &MorTier) -> String {
    let mut parts = Vec::new();
    for item in mor.items.iter() {
        if parts.len() >= 3 {
            break;
        }
        parts.push(item.main.pos.as_str().to_string());
    }
    parts.join(" ")
}

fn extract_gra_pattern(gra: &GraTier) -> String {
    gra.relations
        .iter()
        .take(3)
        .map(|rel| rel.relation.as_str().to_string())
        .collect::<Vec<_>>()
        .join(" ")
}

fn extract_pho_pattern(pho: &PhoTier) -> String {
    let mut content = String::new();
    for (idx, item) in pho.items.iter().enumerate() {
        if idx > 0 {
            content.push(' ');
        }
        let _ = item.write_chat(&mut content);
    }
    content
}

fn create_minimal_file(
    main: &MainTier,
    tiers: &[DependentTier],
) -> Result<String, std::fmt::Error> {
    let mut output = String::new();
    output.push_str("@UTF8\n@Begin\n@Languages:\teng\n@Participants:\tCHI Target_Child\n@ID:\teng|corpus|CHI|||||Target_Child|||\n");
    main.write_chat(&mut output)?;
    output.push('\n');
    for tier in tiers {
        tier.write_chat(&mut output)?;
        output.push('\n');
    }
    output.push_str("@End");
    Ok(output)
}

#[allow(dead_code)]
fn write_to_string(value: &impl WriteChat) -> String {
    let mut out = String::new();
    let _ = value.write_chat(&mut out);
    out
}

fn generate_markdown_specs(
    examples: &[TierExample],
    subcategory: &str,
    output_dir: &Path,
) -> Result<()> {
    for ex in examples {
        let filename = output_dir.join(format!("{}.md", ex.name));
        let mut content = String::new();
        content.push_str(&format!(
            "# {}
\n",
            ex.name
        ));
        content.push_str(&format!("{}\n\n", ex.description));
        content.push_str("## Input\n\n");
        content.push_str("```chat-file\n");
        content.push_str(&ex.input);
        content.push_str("\n```\n\n");
        content.push_str("## Expected CST\n\n");
        content.push_str("```cst\n(TODO)\n```\n\n");
        content.push_str("## Metadata\n\n");
        content.push_str("- **Level**: tier\n");
        content.push_str(&format!("- **Category**: {}\n", subcategory));
        content.push_str(&format!("- **Source**: {}\n", ex.source_file.display()));
        fs::write(&filename, content)?;
    }
    println!(
        "✓ Generated {} examples for {}",
        examples.len(),
        subcategory
    );
    Ok(())
}
