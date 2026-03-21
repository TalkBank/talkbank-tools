//! Integration tests for template generation pipeline

use std::fs;
use spec_runtime_tools::bootstrap::fixture_parser::parse_extract_directive;
use spec_runtime_tools::bootstrap::template_generator::{
    generate_template, write_template_file, TemplateData,
};
use tempfile::tempdir;

/// Tests full pipeline standalone word.
#[test]
fn test_full_pipeline_standalone_word() {
    // Create temp directories
    let temp = tempdir().unwrap();
    let fixtures_dir = temp.path().join("fixtures");
    let templates_dir = temp.path().join("templates");

    fs::create_dir_all(&fixtures_dir).unwrap();
    fs::create_dir_all(&templates_dir).unwrap();

    // Create minimal fixture
    let fixture_path = fixtures_dir.join("test_word.cha");
    let fixture_content = "@UTF8\n\
                           @Begin\n\
                           @Languages:\teng\n\
                           @Participants:\tCHI Target_Child\n\
                           @ID:\teng|corpus|CHI|||||Target_Child|||\n\
                           @Comment:\textract=standalone_word\n\
                           *CHI:\thello .\n\
                           @End\n";
    fs::write(&fixture_path, fixture_content).unwrap();

    // Parse extract directive
    let target_node = parse_extract_directive(fixture_content).unwrap();
    assert_eq!(target_node, "standalone_word");

    // Generate template
    let template = generate_template(&fixture_path, fixture_content, &target_node).unwrap();

    // Verify template structure
    assert!(template.input_wrapper.contains("{input}"));
    assert!(template.cst_wrapper.contains("{fragment}"));
    assert_eq!(template.metadata.target_node, "standalone_word");
    assert!(template.metadata.nesting_level > 5);

    // Write template
    let template_path = templates_dir.join("standalone_word.yaml");
    write_template_file(&template, &template_path).unwrap();

    // Verify file exists and is valid YAML
    assert!(template_path.exists());
    let yaml_content = fs::read_to_string(&template_path).unwrap();
    let _: TemplateData = serde_yaml_ng::from_str(&yaml_content).unwrap();
}

/// Tests multiple fixtures.
#[test]
fn test_multiple_fixtures() {
    let temp = tempdir().unwrap();
    let fixtures_dir = temp.path().join("fixtures");
    let templates_dir = temp.path().join("templates");

    fs::create_dir_all(&fixtures_dir).unwrap();
    fs::create_dir_all(&templates_dir).unwrap();

    // Create multiple fixtures
    let fixtures = vec![
        ("word.cha", "standalone_word", "*CHI:\thello .\n"),
        ("main_tier.cha", "main_tier", "*CHI:\thello .\n"),
    ];

    for (filename, target, content) in fixtures {
        let fixture = format!(
            "@UTF8\n@Begin\n@Comment:\textract={}\n{}\n@End\n",
            target, content
        );
        fs::write(fixtures_dir.join(filename), fixture).unwrap();
    }

    // Process all fixtures
    for entry in fs::read_dir(&fixtures_dir).unwrap() {
        let path = entry.unwrap().path();
        let source = fs::read_to_string(&path).unwrap();
        let target = parse_extract_directive(&source).unwrap();

        let template = generate_template(&path, &source, &target).unwrap();

        let output = templates_dir.join(format!("{}.yaml", target));
        write_template_file(&template, &output).unwrap();

        assert!(output.exists());
    }

    // Verify we generated 2 templates
    let template_count = fs::read_dir(&templates_dir).unwrap().count();
    assert_eq!(template_count, 2);
}
