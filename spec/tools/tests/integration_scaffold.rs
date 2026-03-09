//! Integration tests for scaffold system

use std::fs;
use tempfile::tempdir;

/// Tests full scaffold pipeline.
#[test]
fn test_full_scaffold_pipeline() {
    let dir = tempdir().unwrap();

    // 1. Create node_config.yaml
    let config_yaml = r#"
nodes:
  standalone_word:
    test: true
    template: "word_level"
    priority: high
    reason: "Important word construct"

  word_segment:
    test: false
    template: null
    priority: low
    reason: "Too granular"

  main_tier:
    test: true
    template: "tier_level"
    priority: high
    reason: "Main tier structure"
"#;
    fs::write(dir.path().join("config.yaml"), config_yaml).unwrap();

    // 2. Create templates
    let templates_dir = dir.path().join("templates");
    fs::create_dir(&templates_dir).unwrap();

    let word_template = r#"
input_wrapper: |
  @UTF8
  @Begin
  *CHI:\t{input} .
  @End

cst_wrapper: |
  (document
    {fragment})

metadata:
  nesting_level: 2
  example_input: "hello"
  description: "Word-level construct in main tier"
"#;
    fs::write(templates_dir.join("word_level.yaml"), word_template).unwrap();

    let tier_template = r#"
input_wrapper: |
  @UTF8
  @Begin
  *CHI:\ttest .
  @End

cst_wrapper: "(document {fragment})"

metadata:
  nesting_level: 1
  example_input: "*CHI:\\ttest ."
  description: "Tier-level construct"
"#;
    fs::write(templates_dir.join("tier_level.yaml"), tier_template).unwrap();

    // 3. Create scaffolder
    let output_dir = dir.path().join("output");
    fs::create_dir(&output_dir).unwrap();

    let scaffolder = generators::bootstrap::scaffold::Scaffolder {
        config_path: dir.path().join("config.yaml"),
        template_dir: templates_dir,
        output_dir: output_dir.clone(),
    };

    // 4. Run scaffold
    let report = scaffolder.scaffold(false).unwrap();

    // 5. Verify results
    assert_eq!(report.generated, 2); // standalone_word, main_tier
    assert_eq!(report.skipped, 0);
    assert_eq!(report.errors.len(), 0);

    // 6. Verify files exist
    let word_spec = output_dir.join("standalone_word_example.md");
    assert!(word_spec.exists());

    let tier_spec = output_dir.join("main_tier_example.md");
    assert!(tier_spec.exists());

    // 7. Verify content
    let word_content = fs::read_to_string(&word_spec).unwrap();
    assert!(word_content.contains("# standalone_word_example"));
    assert!(word_content.contains("hello"));
    assert!(word_content.contains("(standalone_word)"));

    let tier_content = fs::read_to_string(&tier_spec).unwrap();
    assert!(tier_content.contains("# main_tier_example"));
    assert!(tier_content.contains("(main_tier)"));
}

/// Tests scaffold with template inheritance.
#[test]
fn test_scaffold_with_template_inheritance() {
    let dir = tempdir().unwrap();

    // Create config
    let config_yaml = r#"
nodes:
  test_node:
    test: true
    template: "child"
    priority: high
    reason: "Test"
"#;
    fs::write(dir.path().join("config.yaml"), config_yaml).unwrap();

    // Create templates with inheritance
    let templates_dir = dir.path().join("templates");
    fs::create_dir(&templates_dir).unwrap();

    let base_template = r#"
input_wrapper: |
  @UTF8
  @Begin
  BASE
  @End

cst_wrapper: "BASE_CST"

metadata:
  nesting_level: 0
  example_input: "base"
  description: "Base description"
"#;
    fs::write(templates_dir.join("_base.yaml"), base_template).unwrap();

    let child_template = r#"
extends: "_base"

input_wrapper: |
  @UTF8
  @Begin
  CHILD
  @End

metadata:
  nesting_level: 5
  example_input: "child"
"#;
    fs::write(templates_dir.join("child.yaml"), child_template).unwrap();

    // Create output dir
    let output_dir = dir.path().join("output");
    fs::create_dir(&output_dir).unwrap();

    // Run scaffold
    let scaffolder = generators::bootstrap::scaffold::Scaffolder {
        config_path: dir.path().join("config.yaml"),
        template_dir: templates_dir,
        output_dir: output_dir.clone(),
    };

    let report = scaffolder.scaffold(false).unwrap();

    assert_eq!(report.generated, 1);
    assert_eq!(report.skipped, 0);

    // Verify inheritance resolved correctly
    let spec = fs::read_to_string(output_dir.join("test_node_example.md")).unwrap();
    assert!(spec.contains("CHILD")); // Child overrides input_wrapper
    assert!(spec.contains("BASE_CST")); // Inherited cst_wrapper
    assert!(spec.contains("Base description")); // Inherited description
}
