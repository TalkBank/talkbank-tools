//! Heuristic classifier that assigns test priority to tree-sitter grammar nodes.
//!
//! Given a [`NodeInfo`] extracted from `grammar.js`, the analyzer applies a
//! rule-based decision tree to decide whether the node is critical enough to
//! require a dedicated spec test (`MustTest`/`ShouldTest`) or can be safely
//! skipped (`CouldTest`/`Skip`).  The classification drives downstream
//! scaffolding -- only nodes marked for testing get generated spec files.

use super::grammar::NodeInfo;
use talkbank_parser::node_types;

/// Priority level recommending whether a grammar node needs a dedicated spec test.
///
/// The levels mirror RFC 2119 language and map directly to the `priority`
/// field written into `all_nodes_annotated.yaml`.  Only `MustTest` and
/// `ShouldTest` cause [`NodeClassification::test_flag`] to return `true`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TestSuggestion {
    /// Core structural nodes (document, line, utterance, main/dependent tier)
    /// that define the skeleton of every CHAT file.  A grammar regression in
    /// any of these breaks all downstream processing.
    MustTest,
    /// Semantically important constructs (headers, dependent tier types, word
    /// nodes, annotations, events) that carry linguistic content.  Regressions
    /// here silently corrupt data.
    ShouldTest,
    /// Constructs that are syntactically valid but rarely appear in real corpora
    /// or whose correct handling is already implied by higher-level tests.
    /// Testing is optional but harmless.
    CouldTest,
    /// Nodes too granular to test in isolation (punctuation tokens, whitespace,
    /// sub-word components) or whose correctness is a validation concern rather
    /// than a parsing concern.  Generating specs for these adds noise without
    /// meaningful coverage.
    Skip,
}

/// Full classification result produced by [`classify_node`] for a single grammar node.
///
/// Downstream consumers use this to decide whether to generate a spec file and
/// which template to apply.  The struct is also serialized into the bootstrap
/// YAML config by [`super::config_gen::generate_bootstrap_config`].
#[derive(Debug, Clone)]
pub struct NodeClassification {
    /// Tree-sitter node kind name (e.g. `"standalone_word"`, `"mor_dependent_tier"`).
    pub name: String,
    /// Recommended test priority.  Only `MustTest` and `ShouldTest` produce a
    /// `true` test flag.
    pub suggestion: TestSuggestion,
    /// Human-readable explanation of why this priority was assigned, written
    /// into the YAML config as a `reason:` field for the user to review.
    pub reason: String,
    /// Name of the scaffold template to use (e.g. `"word_level"`, `"document"`).
    /// `None` for nodes classified as `Skip` since no spec is generated.
    pub template: Option<String>,
    /// Free-form priority label (`"critical"`, `"high"`, `"medium"`, `"low"`)
    /// that appears as a YAML field and drives inline comments in the generated
    /// config.
    pub priority: String,
}

impl NodeClassification {
    /// Convert TestSuggestion to test: true/false flag
    pub fn test_flag(&self) -> bool {
        matches!(
            self.suggestion,
            TestSuggestion::MustTest | TestSuggestion::ShouldTest
        )
    }
}

/// Returns `true` if `name` appears in the static list `items`.
fn is_one_of(name: &str, items: &[&'static str]) -> bool {
    items.contains(&name)
}

/// Classify a node using heuristic rules
///
/// # Heuristic Rules
///
/// ## MustTest (Critical)
/// - Core structure: document, line, utterance, main_tier, dependent_tier
///
/// ## ShouldTest (High Priority)
/// - Tier types: explicit dependent tiers
/// - Word structures: explicit word nodes
/// - Annotation nodes: explicit annotation nodes
///
/// ## Skip (Too Granular / Trivial / Validation)
/// - Trivial terminals (punctuation/whitespace)
/// - Word sub-components
/// - Individual CA/pause markers (handled at higher level)
///
/// ## CouldTest (Medium/Low Priority)
/// - Everything else
pub fn classify_node(node: &NodeInfo) -> NodeClassification {
    let name = &node.name;

    const WORD_SUB_COMPONENTS: &[&str] = &[
        // WORD_BODY removed — coarsened into standalone_word token (Phase 2)
    ];

    const RETRACE_NODES: &[&str] = &[
        node_types::RETRACE_COMPLETE,
        node_types::RETRACE_PARTIAL,
        node_types::RETRACE_REFORMULATION,
        node_types::RETRACE_MULTIPLE,
        node_types::RETRACE_UNCERTAIN,
    ];

    const SCOPED_NODES: &[&str] = &[
        node_types::SCOPED_STRESSING,
        node_types::SCOPED_CONTRASTIVE_STRESSING,
        // SCOPED_SYMBOL — removed (wrapper inlined into base_annotation, Phase 5)
        node_types::SCOPED_BEST_GUESS,
        node_types::SCOPED_UNCERTAIN,
    ];

    const LONG_FEATURE_NODES: &[&str] = &[
        node_types::LONG_FEATURE,
        node_types::LONG_FEATURE_BEGIN,
        node_types::LONG_FEATURE_END,
        node_types::LONG_FEATURE_LABEL,
    ];

    const NONVOCAL_NODES: &[&str] = &[
        node_types::NONVOCAL,
        node_types::NONVOCAL_SIMPLE,
        node_types::NONVOCAL_BEGIN,
        node_types::NONVOCAL_END,
    ];

    const WHITESPACE_UNITS: &[&str] = &[
        node_types::INTERRUPTION,
        node_types::SELF_INTERRUPTION,
        node_types::BROKEN_QUESTION,
        node_types::FINAL_CODES,
        node_types::POSTCODE,
        node_types::FREECODE,
        node_types::CONTENT_ITEM,
        node_types::BASE_CONTENT_ITEM,
        node_types::TERMINATOR,
        node_types::UTTERANCE_END,
        node_types::REPLACEMENT,
        node_types::QUOTATION,
        node_types::MEDIA_URL,
        node_types::LINKER_QUOTATION_FOLLOWS,
        node_types::FREE_TEXT,
        node_types::REST_OF_LINE,
    ];

    const CORE_STRUCTURES: &[&str] = &[
        node_types::DOCUMENT,
        node_types::LINE,
        node_types::UTTERANCE,
        node_types::MAIN_TIER,
        node_types::DEPENDENT_TIER,
    ];

    const TRIVIAL_TERMINALS: &[&str] = &[
        node_types::PERIOD,
        node_types::QUESTION,
        node_types::EXCLAMATION,
        node_types::INTERRUPTED_QUESTION,
        node_types::TRAILING_OFF,
        node_types::TRAILING_OFF_QUESTION,
        node_types::COMMA,
        node_types::SPACE,
        node_types::TAB,
        node_types::NEWLINE,
        node_types::CONTINUATION,
        node_types::STAR,
        node_types::HYPHEN,
        node_types::COLON,
        node_types::SEMICOLON,
        node_types::PLUS,
        node_types::TILDE,
        node_types::AMPERSAND,
        // node_types::PERCENT_SIGN — removed (orphaned after annotation coarsening, Phase 5)
    ];

    const CA_IMPORTANT: &[&str] = &[
        // ca_delimiter and ca_element removed — coarsened into standalone_word token (Phase 2)
        node_types::CA_CONTINUATION_MARKER,
        node_types::CA_NO_BREAK,
        node_types::CA_NO_BREAK_LINKER,
        node_types::CA_TECHNICAL_BREAK,
        node_types::CA_TECHNICAL_BREAK_LINKER,
    ];

    const PAUSE_IMPORTANT: &[&str] = &[node_types::PAUSE_TOKEN];

    const DEPENDENT_TIERS: &[&str] = &[
        node_types::ACT_DEPENDENT_TIER,
        node_types::ADD_DEPENDENT_TIER,
        node_types::ALT_DEPENDENT_TIER,
        node_types::COD_DEPENDENT_TIER,
        node_types::COH_DEPENDENT_TIER,
        node_types::COM_DEPENDENT_TIER,
        node_types::DEF_DEPENDENT_TIER,
        node_types::ENG_DEPENDENT_TIER,
        node_types::ERR_DEPENDENT_TIER,
        node_types::EXP_DEPENDENT_TIER,
        node_types::FAC_DEPENDENT_TIER,
        node_types::FLO_DEPENDENT_TIER,
        node_types::GLS_DEPENDENT_TIER,
        node_types::GPX_DEPENDENT_TIER,
        node_types::GRA_DEPENDENT_TIER,
        node_types::GRA_DEPENDENT_TIER,
        node_types::INT_DEPENDENT_TIER,
        node_types::MOD_DEPENDENT_TIER,
        node_types::MOR_DEPENDENT_TIER,
        node_types::ORT_DEPENDENT_TIER,
        node_types::PAR_DEPENDENT_TIER,
        node_types::PHO_DEPENDENT_TIER,
        node_types::SIN_DEPENDENT_TIER,
        node_types::SIT_DEPENDENT_TIER,
        node_types::SPA_DEPENDENT_TIER,
        node_types::TIM_DEPENDENT_TIER,
        node_types::ERR_DEPENDENT_TIER,
        node_types::GRA_DEPENDENT_TIER,
        node_types::MOR_DEPENDENT_TIER,
        node_types::WOR_DEPENDENT_TIER,
        node_types::X_DEPENDENT_TIER,
    ];

    const HEADER_TYPES: &[&str] = &[
        node_types::ACTIVITIES_HEADER,
        node_types::BCK_HEADER,
        node_types::BEGIN_HEADER,
        node_types::BG_HEADER,
        node_types::BIRTH_OF_HEADER,
        node_types::BIRTHPLACE_OF_HEADER,
        node_types::BLANK_HEADER,
        node_types::COLOR_WORDS_HEADER,
        node_types::COMMENT_HEADER,
        node_types::DATE_HEADER,
        node_types::EG_HEADER,
        node_types::END_HEADER,
        node_types::FONT_HEADER,
        node_types::G_HEADER,
        node_types::ID_HEADER,
        node_types::L1_OF_HEADER,
        node_types::LANGUAGES_HEADER,
        node_types::LOCATION_HEADER,
        node_types::MEDIA_HEADER,
        node_types::NEW_EPISODE_HEADER,
        node_types::NUMBER_HEADER,
        node_types::OPTIONS_HEADER,
        node_types::PAGE_HEADER,
        node_types::PARTICIPANTS_HEADER,
        node_types::PID_HEADER,
        node_types::PRE_BEGIN_HEADER,
        node_types::RECORDING_QUALITY_HEADER,
        node_types::ROOM_LAYOUT_HEADER,
        node_types::SITUATION_HEADER,
        node_types::T_HEADER,
        node_types::TAPE_LOCATION_HEADER,
        node_types::THUMBNAIL_HEADER,
        node_types::TIME_DURATION_HEADER,
        node_types::TIME_START_HEADER,
        node_types::TRANSCRIBER_HEADER,
        node_types::TRANSCRIPTION_HEADER,
        node_types::TYPES_HEADER,
        node_types::UTF8_HEADER,
        node_types::VIDEOS_HEADER,
        node_types::WARNING_HEADER,
        node_types::WINDOW_HEADER,
    ];

    const SKIP_PRE_BEGIN_HEADERS: &[&str] = &[
        node_types::PID_HEADER,
        node_types::COLOR_WORDS_HEADER,
        node_types::WINDOW_HEADER,
        node_types::FONT_HEADER,
    ];

    const WORD_NODES: &[&str] = &[
        node_types::STANDALONE_WORD,
        node_types::WORD_WITH_OPTIONAL_ANNOTATIONS,
        // WORD_BODY removed — coarsened into standalone_word token (Phase 2)
        node_types::MOR_WORD,
    ];

    const ANNOTATION_NODES: &[&str] = &[
        node_types::ALT_ANNOTATION,
        node_types::BASE_ANNOTATION,
        node_types::DURATION_ANNOTATION,
        node_types::ERROR_MARKER_ANNOTATION,
        node_types::EXPLANATION_ANNOTATION,
        node_types::PARA_ANNOTATION,
        node_types::PERCENT_ANNOTATION,
    ];

    const EVENT_NODES: &[&str] = &[node_types::EVENT, node_types::OTHER_SPOKEN_EVENT];

    if is_one_of(name, WORD_SUB_COMPONENTS) {
        return NodeClassification {
            name: name.clone(),
            suggestion: TestSuggestion::Skip,
            reason: "Sub-word component - not whitespace-separated (internal word structure)"
                .to_string(),
            template: None,
            priority: "low".to_string(),
        };
    }

    if is_one_of(name, RETRACE_NODES) {
        return NodeClassification {
            name: name.clone(),
            suggestion: TestSuggestion::ShouldTest,
            reason: "Error correction - whitespace-separated unit".to_string(),
            template: Some("word_level".to_string()),
            priority: "high".to_string(),
        };
    }

    if is_one_of(name, SCOPED_NODES) {
        return NodeClassification {
            name: name.clone(),
            suggestion: TestSuggestion::ShouldTest,
            reason: "Prosodic feature - whitespace-separated unit".to_string(),
            template: Some("word_level".to_string()),
            priority: "high".to_string(),
        };
    }

    if is_one_of(name, LONG_FEATURE_NODES) {
        return NodeClassification {
            name: name.clone(),
            suggestion: TestSuggestion::ShouldTest,
            reason: "Multi-word feature - whitespace-separated unit".to_string(),
            template: Some("word_level".to_string()),
            priority: "high".to_string(),
        };
    }

    if is_one_of(name, NONVOCAL_NODES) {
        return NodeClassification {
            name: name.clone(),
            suggestion: TestSuggestion::ShouldTest,
            reason: "Action - whitespace-separated unit".to_string(),
            template: Some("word_level".to_string()),
            priority: "high".to_string(),
        };
    }

    if is_one_of(name, WHITESPACE_UNITS) {
        return NodeClassification {
            name: name.clone(),
            suggestion: TestSuggestion::ShouldTest,
            reason: "Whitespace-separated parsing unit".to_string(),
            template: Some("word_level".to_string()),
            priority: "high".to_string(),
        };
    }

    if is_one_of(name, CORE_STRUCTURES) {
        return NodeClassification {
            name: name.clone(),
            suggestion: TestSuggestion::MustTest,
            reason: "Core CHAT file structure".to_string(),
            template: Some("document".to_string()),
            priority: "critical".to_string(),
        };
    }

    if is_one_of(name, TRIVIAL_TERMINALS) {
        return NodeClassification {
            name: name.clone(),
            suggestion: TestSuggestion::Skip,
            reason: "Trivial terminal - tree-sitter handles correctly".to_string(),
            template: None,
            priority: "low".to_string(),
        };
    }

    if is_one_of(name, CA_IMPORTANT) {
        return NodeClassification {
            name: name.clone(),
            suggestion: TestSuggestion::ShouldTest,
            reason: "CA annotation construct - important for conversation analysis".to_string(),
            template: Some("word_level".to_string()),
            priority: "high".to_string(),
        };
    }

    if is_one_of(name, PAUSE_IMPORTANT) {
        return NodeClassification {
            name: name.clone(),
            suggestion: TestSuggestion::ShouldTest,
            reason: "Pause annotation - important timing construct".to_string(),
            template: Some("word_level".to_string()),
            priority: "high".to_string(),
        };
    }

    if is_one_of(name, DEPENDENT_TIERS) {
        return NodeClassification {
            name: name.clone(),
            suggestion: TestSuggestion::ShouldTest,
            reason: "Dependent tier type - important CHAT construct".to_string(),
            template: Some("tier_level".to_string()),
            priority: "high".to_string(),
        };
    }

    if is_one_of(name, HEADER_TYPES) {
        if is_one_of(name, SKIP_PRE_BEGIN_HEADERS) {
            return NodeClassification {
                name: name.clone(),
                suggestion: TestSuggestion::Skip,
                reason: "Pre-begin header - test as part of pre_begin_header".to_string(),
                template: None,
                priority: "low".to_string(),
            };
        }

        return NodeClassification {
            name: name.clone(),
            suggestion: TestSuggestion::ShouldTest,
            reason: "CHAT header type - document structure".to_string(),
            template: Some("document".to_string()),
            priority: "high".to_string(),
        };
    }

    if is_one_of(name, ANNOTATION_NODES) {
        return NodeClassification {
            name: name.clone(),
            suggestion: TestSuggestion::ShouldTest,
            reason: "Annotation construct - important metadata".to_string(),
            template: Some("word_level".to_string()),
            priority: "high".to_string(),
        };
    }

    if is_one_of(name, EVENT_NODES) {
        return NodeClassification {
            name: name.clone(),
            suggestion: TestSuggestion::ShouldTest,
            reason: "Event construct - important CHAT element".to_string(),
            template: Some("word_level".to_string()),
            priority: "high".to_string(),
        };
    }

    if is_one_of(name, WORD_NODES) {
        return NodeClassification {
            name: name.clone(),
            suggestion: TestSuggestion::ShouldTest,
            reason: "Word-level construct - core parsing unit".to_string(),
            template: Some("word_level".to_string()),
            priority: "high".to_string(),
        };
    }

    NodeClassification {
        name: name.clone(),
        suggestion: TestSuggestion::CouldTest,
        reason: "Additional construct - optional testing".to_string(),
        template: Some(name.clone()),
        priority: "medium".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::Result;

    /// Tests classify core nodes.
    #[test]
    fn test_classify_core_nodes() -> Result<()> {
        let node = NodeInfo {
            name: node_types::DOCUMENT.to_string(),
            rule_definition: "...".to_string(),
        };
        let classification = classify_node(&node);
        assert_eq!(classification.suggestion, TestSuggestion::MustTest);
        assert_eq!(classification.priority, "critical");
        assert!(classification.test_flag());

        let node = NodeInfo {
            name: node_types::LINE.to_string(),
            rule_definition: "...".to_string(),
        };
        let classification = classify_node(&node);
        assert_eq!(classification.suggestion, TestSuggestion::MustTest);
        Ok(())
    }

    /// Tests classify trivial terminals.
    #[test]
    fn test_classify_trivial_terminals() -> Result<()> {
        let node = NodeInfo {
            name: node_types::PERIOD.to_string(),
            rule_definition: "...".to_string(),
        };
        let classification = classify_node(&node);
        assert_eq!(classification.suggestion, TestSuggestion::Skip);
        assert!(!classification.test_flag());

        let node = NodeInfo {
            name: node_types::SPACE.to_string(),
            rule_definition: "...".to_string(),
        };
        let classification = classify_node(&node);
        assert_eq!(classification.suggestion, TestSuggestion::Skip);
        Ok(())
    }

    // test_classify_word_sub_components removed — Phase 2 coarsened all word
    // sub-components into standalone_word token, so WORD_SUB_COMPONENTS is empty.

    /// Tests classify mor nodes.
    #[test]
    fn test_classify_mor_nodes() -> Result<()> {
        let node = NodeInfo {
            name: node_types::MOR_DEPENDENT_TIER.to_string(),
            rule_definition: "...".to_string(),
        };
        let classification = classify_node(&node);
        assert_eq!(classification.suggestion, TestSuggestion::ShouldTest);
        assert!(classification.test_flag());
        Ok(())
    }

    /// Tests classify word nodes.
    #[test]
    fn test_classify_word_nodes() -> Result<()> {
        let node = NodeInfo {
            name: node_types::STANDALONE_WORD.to_string(),
            rule_definition: "...".to_string(),
        };
        let classification = classify_node(&node);
        assert_eq!(classification.suggestion, TestSuggestion::ShouldTest);
        assert_eq!(classification.template, Some("word_level".to_string()));
        Ok(())
    }

    /// Tests classify ca nodes.
    #[test]
    fn test_classify_ca_nodes() -> Result<()> {
        let node = NodeInfo {
            name: node_types::ALT_ANNOTATION.to_string(),
            rule_definition: "...".to_string(),
        };
        let classification = classify_node(&node);
        assert_eq!(classification.suggestion, TestSuggestion::ShouldTest);
        Ok(())
    }

    /// Tests classify dependent tiers.
    #[test]
    fn test_classify_dependent_tiers() -> Result<()> {
        let tiers = vec![
            node_types::MOR_DEPENDENT_TIER,
            node_types::GRA_DEPENDENT_TIER,
            node_types::PHO_DEPENDENT_TIER,
            node_types::SIN_DEPENDENT_TIER,
        ];

        for tier in tiers {
            let node = NodeInfo {
                name: tier.to_string(),
                rule_definition: "...".to_string(),
            };
            let classification = classify_node(&node);
            assert_eq!(
                classification.suggestion,
                TestSuggestion::ShouldTest,
                "Tier {} should be ShouldTest",
                tier
            );
            assert!(classification.test_flag());
        }
        Ok(())
    }

    /// Tests promoted whitespace units.
    #[test]
    fn test_promoted_whitespace_units() -> Result<()> {
        let promoted_nodes = vec![
            node_types::INTERRUPTION,
            node_types::SELF_INTERRUPTION,
            node_types::BROKEN_QUESTION,
            node_types::FINAL_CODES,
            node_types::POSTCODE,
            node_types::FREECODE,
            node_types::CONTENT_ITEM,
            node_types::BASE_CONTENT_ITEM,
            node_types::TERMINATOR,
            node_types::UTTERANCE_END,
            node_types::REPLACEMENT,
            node_types::QUOTATION,
            node_types::MEDIA_URL,
        ];

        for node_name in promoted_nodes {
            let node = NodeInfo {
                name: node_name.to_string(),
                rule_definition: "...".to_string(),
            };
            let classification = classify_node(&node);
            assert_eq!(
                classification.suggestion,
                TestSuggestion::ShouldTest,
                "Node {} should be ShouldTest (whitespace-separated)",
                node_name
            );
            assert!(
                classification.test_flag(),
                "Node {} should have test_flag = true",
                node_name
            );
        }
        Ok(())
    }
}
