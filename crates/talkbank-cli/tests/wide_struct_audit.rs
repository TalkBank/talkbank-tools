//! Audit wide Rust structs so field-bag growth stays explicit and reviewed.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

/// Number of named fields that triggers a wide-struct audit entry.
const WIDE_STRUCT_THRESHOLD: usize = 10;

/// Audit classification for one intentionally wide struct.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum WideStructDisposition {
    /// Flat only because a CLI, transport, or tool boundary wants it.
    BoundaryShim,
    /// Mirrors a report, schema, or other transport-facing shape.
    TransportRecord,
    /// A real aggregate with acceptable cohesion for now.
    RealAggregate,
    /// Known design smell that should be decomposed further.
    RefactorTarget,
}

impl WideStructDisposition {
    /// Render a short human-readable label for failure messages.
    fn label(self) -> &'static str {
        match self {
            Self::BoundaryShim => "boundary shim",
            Self::TransportRecord => "transport record",
            Self::RealAggregate => "real aggregate",
            Self::RefactorTarget => "refactor target",
        }
    }
}

/// One reviewed wide-struct entry in the repo audit.
#[derive(Clone, Copy, Debug)]
struct WideStructAllowance {
    /// Repo-relative Rust path containing the struct.
    path: &'static str,
    /// Struct name as written in source.
    struct_name: &'static str,
    /// Maximum reviewed named-field count.
    max_fields: usize,
    /// Maximum reviewed boolean-field count.
    max_bool_fields: usize,
    /// Audit classification for this shape.
    disposition: WideStructDisposition,
    /// Brief rationale for why it currently exists in this form.
    reason: &'static str,
}

/// Parsed metadata for one named Rust struct.
#[derive(Clone, Debug, Eq, PartialEq)]
struct NamedStructInfo {
    /// Repo-relative file path.
    path: String,
    /// One-based declaration line.
    line: usize,
    /// Struct identifier.
    struct_name: String,
    /// Number of named fields.
    field_count: usize,
    /// Number of fields whose type includes `bool`.
    bool_field_count: usize,
}

/// Reviewed wide structs in `talkbank-tools`.
const WIDE_STRUCT_ALLOWANCES: &[WideStructAllowance] = &[
    WideStructAllowance {
        path: "crates/talkbank-clan/src/commands/eval.rs",
        struct_name: "SpeakerEval",
        max_fields: 25,
        max_bool_fields: 0,
        disposition: WideStructDisposition::TransportRecord,
        reason: "metric record for one EVAL speaker report",
    },
    WideStructAllowance {
        path: "crates/talkbank-clan/src/commands/kideval.rs",
        struct_name: "SpeakerKideval",
        max_fields: 21,
        max_bool_fields: 0,
        disposition: WideStructDisposition::TransportRecord,
        reason: "metric record for one KIDEVAL speaker report",
    },
    WideStructAllowance {
        path: "crates/talkbank-clan/src/commands/complexity.rs",
        struct_name: "SpeakerAccum",
        max_fields: 19,
        max_bool_fields: 2,
        disposition: WideStructDisposition::RefactorTarget,
        reason: "accumulator mixes counters and mode flags",
    },
    WideStructAllowance {
        path: "src/test_dashboard/app.rs",
        struct_name: "AppState",
        max_fields: 19,
        max_bool_fields: 2,
        disposition: WideStructDisposition::RefactorTarget,
        reason: "dashboard state still mixes corpus progress, global totals, render flags, and timing",
    },
    WideStructAllowance {
        path: "crates/talkbank-clan/src/commands/complexity.rs",
        struct_name: "SpeakerComplexity",
        max_fields: 18,
        max_bool_fields: 0,
        disposition: WideStructDisposition::TransportRecord,
        reason: "complexity report record",
    },
    WideStructAllowance {
        path: "crates/talkbank-cli/src/ui/validation_tui/state.rs",
        struct_name: "TuiState",
        max_fields: 15,
        max_bool_fields: 1,
        disposition: WideStructDisposition::RefactorTarget,
        reason: "mixes widget state, progress counters, discovery flags, and final summary state",
    },
    WideStructAllowance {
        path: "crates/talkbank-clan/src/framework/mor.rs",
        struct_name: "MorPosCount",
        max_fields: 14,
        max_bool_fields: 0,
        disposition: WideStructDisposition::TransportRecord,
        reason: "report record for morphology counts",
    },
    WideStructAllowance {
        path: "crates/talkbank-model/src/model/alignment_set.rs",
        struct_name: "AlignmentUnits",
        max_fields: 14,
        max_bool_fields: 0,
        disposition: WideStructDisposition::RealAggregate,
        reason: "cohesive alignment domain aggregate",
    },
    WideStructAllowance {
        path: "spec/tools/src/bin/extract_corpus_candidates.rs",
        struct_name: "Candidate",
        max_fields: 13,
        max_bool_fields: 4,
        disposition: WideStructDisposition::BoundaryShim,
        reason: "spec-tool report row at a tooling boundary",
    },
    WideStructAllowance {
        path: "crates/talkbank-clan/src/commands/flucalc.rs",
        struct_name: "SpeakerFluency",
        max_fields: 12,
        max_bool_fields: 0,
        disposition: WideStructDisposition::TransportRecord,
        reason: "fluency report record",
    },
    WideStructAllowance {
        path: "spec/tools/src/bin/extract_corpus_candidates.rs",
        struct_name: "Args",
        max_fields: 11,
        max_bool_fields: 4,
        disposition: WideStructDisposition::BoundaryShim,
        reason: "tool-only clap boundary type",
    },
    WideStructAllowance {
        path: "crates/talkbank-clan/src/database/entry.rs",
        struct_name: "DbMetadata",
        max_fields: 10,
        max_bool_fields: 0,
        disposition: WideStructDisposition::TransportRecord,
        reason: "database metadata row shape",
    },
    WideStructAllowance {
        path: "crates/talkbank-cli/src/cli/args/clan_common.rs",
        struct_name: "CommonAnalysisArgs",
        max_fields: 10,
        max_bool_fields: 2,
        disposition: WideStructDisposition::BoundaryShim,
        reason: "shared clap boundary shape for analysis commands",
    },
    WideStructAllowance {
        path: "crates/talkbank-clan/src/service.rs",
        struct_name: "AnalysisOptions",
        max_fields: 25,
        max_bool_fields: 5,
        disposition: WideStructDisposition::BoundaryShim,
        reason: "raw adapter-to-library option bag consumed by AnalysisRequestBuilder before defaults and validation",
    },
    WideStructAllowance {
        path: "crates/talkbank-lsp/src/backend/contracts.rs",
        struct_name: "AnalysisOptionsPayload",
        max_fields: 22,
        max_bool_fields: 5,
        disposition: WideStructDisposition::BoundaryShim,
        reason: "public editor/server JSON payload for talkbank/analyze kept flat for schema generation and transport clarity",
    },
    WideStructAllowance {
        path: "crates/talkbank-cli/src/ui/theme.rs",
        struct_name: "Theme",
        max_fields: 10,
        max_bool_fields: 0,
        disposition: WideStructDisposition::RealAggregate,
        reason: "cohesive color palette aggregate",
    },
    WideStructAllowance {
        path: "crates/talkbank-lsp/src/backend/participants.rs",
        struct_name: "ParticipantFields",
        max_fields: 10,
        max_bool_fields: 0,
        disposition: WideStructDisposition::TransportRecord,
        reason: "participant edit payload for the editor boundary",
    },
    WideStructAllowance {
        path: "crates/talkbank-lsp/src/backend/state.rs",
        struct_name: "Backend",
        max_fields: 10,
        max_bool_fields: 1,
        disposition: WideStructDisposition::RefactorTarget,
        reason: "service-root aggregate that still wants cache and service grouping",
    },
    WideStructAllowance {
        path: "crates/talkbank-model/src/model/alignment_set.rs",
        struct_name: "AlignmentSet",
        max_fields: 10,
        max_bool_fields: 0,
        disposition: WideStructDisposition::RealAggregate,
        reason: "cohesive alignment container",
    },
    WideStructAllowance {
        path: "crates/talkbank-model/src/model/content/word/word_serialize.rs",
        struct_name: "WordJsonSchema",
        max_fields: 10,
        max_bool_fields: 0,
        disposition: WideStructDisposition::TransportRecord,
        reason: "JSON schema boundary for word serialization",
    },
    WideStructAllowance {
        path: "crates/talkbank-model/src/model/content/word/word_type.rs",
        struct_name: "Word",
        max_fields: 10,
        max_bool_fields: 0,
        disposition: WideStructDisposition::RealAggregate,
        reason: "core word domain aggregate",
    },
    WideStructAllowance {
        path: "crates/talkbank-model/src/model/header/id.rs",
        struct_name: "IDHeader",
        max_fields: 10,
        max_bool_fields: 0,
        disposition: WideStructDisposition::TransportRecord,
        reason: "header record mirrors one CHAT ID line",
    },
    WideStructAllowance {
        path: "crates/talkbank-parser-tests/src/bin/audit_error_codes.rs",
        struct_name: "Analysis",
        max_fields: 10,
        max_bool_fields: 0,
        disposition: WideStructDisposition::TransportRecord,
        reason: "audit tool report row",
    },
    WideStructAllowance {
        path: "spec/tools/src/bin/corpus_node_coverage.rs",
        struct_name: "CoverageReport",
        max_fields: 10,
        max_bool_fields: 0,
        disposition: WideStructDisposition::TransportRecord,
        reason: "spec-tool coverage report shape",
    },
];

/// Resolve the repo root from the test crate.
fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("tests live under crates/")
        .parent()
        .expect("repo root lives above crates/")
        .to_path_buf()
}

/// Return the Rust source roots covered by this audit.
fn rust_scan_roots(root: &Path) -> Vec<PathBuf> {
    ["src", "crates", "tests", "spec/tools", "examples", "fuzz"]
        .iter()
        .map(|relative| root.join(relative))
        .collect()
}

/// Recursively walk one directory without pulling in extra test dependencies.
fn walkdir(dir: &Path) -> Vec<PathBuf> {
    let mut result = Vec::new();
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                let name = path
                    .file_name()
                    .and_then(|value| value.to_str())
                    .unwrap_or("");
                if !matches!(name, ".git" | "target" | "grammar" | "__pycache__") {
                    result.extend(walkdir(&path));
                }
            } else if path.extension().and_then(|value| value.to_str()) == Some("rs") {
                result.push(path);
            }
        }
    }
    result
}

/// Parse all named Rust structs under the audit roots.
fn scan_named_structs(root: &Path) -> Vec<NamedStructInfo> {
    let mut structs = Vec::new();

    for base in rust_scan_roots(root) {
        if !base.exists() {
            continue;
        }
        for path in walkdir(&base) {
            let relative = path
                .strip_prefix(root)
                .expect("scan path should be inside repo")
                .to_string_lossy()
                .into_owned();
            let text = match std::fs::read_to_string(&path) {
                Ok(text) => text,
                Err(_) => continue,
            };
            structs.extend(parse_named_structs_in_file(&relative, &text));
        }
    }

    structs.sort_by(|left, right| {
        left.path
            .cmp(&right.path)
            .then(left.struct_name.cmp(&right.struct_name))
    });
    structs
}

/// Parse named structs from one Rust source file using a lightweight line scan.
fn parse_named_structs_in_file(relative_path: &str, text: &str) -> Vec<NamedStructInfo> {
    let lines: Vec<&str> = text.lines().collect();
    let mut result = Vec::new();
    let mut index = 0;

    while index < lines.len() {
        let line = lines[index].trim();
        let Some(struct_name) = struct_name_from_declaration(line) else {
            index += 1;
            continue;
        };

        let mut depth = brace_delta(line);
        let mut field_count = 0;
        let mut bool_field_count = 0;
        let start_line = index + 1;
        index += 1;

        while index < lines.len() && depth > 0 {
            let current = lines[index];
            let trimmed = current.trim();
            if depth == 1 && is_named_field(trimmed) {
                field_count += 1;
                if field_type(trimmed).is_some_and(|value| value.contains("bool")) {
                    bool_field_count += 1;
                }
            }
            depth += brace_delta(current);
            index += 1;
        }

        result.push(NamedStructInfo {
            path: relative_path.to_string(),
            line: start_line,
            struct_name,
            field_count,
            bool_field_count,
        });
    }

    result
}

/// Return the struct name if the line starts a named-struct declaration.
fn struct_name_from_declaration(line: &str) -> Option<String> {
    let declaration = line
        .strip_prefix("pub struct ")
        .or_else(|| line.strip_prefix("struct "))?;
    if !declaration.contains('{') {
        return None;
    }

    let name = declaration.split('{').next()?.trim();
    let name = name.split('<').next()?.trim();
    let name = name.split_whitespace().next()?.trim();
    if name.is_empty() {
        None
    } else {
        Some(name.to_string())
    }
}

/// Return the brace delta for one source line.
fn brace_delta(line: &str) -> isize {
    line.chars().fold(0isize, |delta, ch| match ch {
        '{' => delta + 1,
        '}' => delta - 1,
        _ => delta,
    })
}

/// Determine whether one trimmed line looks like a named struct field.
fn is_named_field(line: &str) -> bool {
    if line.is_empty()
        || line.starts_with("//")
        || line.starts_with("///")
        || line.starts_with("#[")
        || line.starts_with("pub use ")
    {
        return false;
    }
    line.contains(':') && !line.starts_with("fn ") && !line.starts_with("where ")
}

/// Extract the field type from a simple named-field line.
fn field_type(line: &str) -> Option<&str> {
    let (_, ty) = line.split_once(':')?;
    Some(ty.trim().trim_end_matches(','))
}

/// Ensure every wide Rust struct is explicitly classified in the audit allowlist.
#[test]
fn wide_structs_are_reviewed_and_capped() {
    let root = repo_root();
    let wide_structs: Vec<NamedStructInfo> = scan_named_structs(&root)
        .into_iter()
        .filter(|info| info.field_count >= WIDE_STRUCT_THRESHOLD)
        .collect();

    let actual_by_key: BTreeMap<(String, String), NamedStructInfo> = wide_structs
        .iter()
        .cloned()
        .map(|info| ((info.path.clone(), info.struct_name.clone()), info))
        .collect();
    let expected_keys: BTreeSet<(String, String)> = WIDE_STRUCT_ALLOWANCES
        .iter()
        .map(|entry| (entry.path.to_string(), entry.struct_name.to_string()))
        .collect();

    let mut failures = Vec::new();

    for info in &wide_structs {
        let key = (info.path.clone(), info.struct_name.clone());
        let Some(allowance) = WIDE_STRUCT_ALLOWANCES
            .iter()
            .find(|entry| entry.path == info.path && entry.struct_name == info.struct_name)
        else {
            failures.push(format!(
                "{}:{}: {} has {} fields and {} bool fields but no audit entry",
                info.path, info.line, info.struct_name, info.field_count, info.bool_field_count
            ));
            continue;
        };

        if info.field_count > allowance.max_fields {
            failures.push(format!(
                "{}:{}: {} grew from reviewed max {} fields to {} ({}, {})",
                info.path,
                info.line,
                info.struct_name,
                allowance.max_fields,
                info.field_count,
                allowance.disposition.label(),
                allowance.reason
            ));
        }

        if info.bool_field_count > allowance.max_bool_fields {
            failures.push(format!(
                "{}:{}: {} grew from reviewed max {} bool fields to {} ({}, {})",
                info.path,
                info.line,
                info.struct_name,
                allowance.max_bool_fields,
                info.bool_field_count,
                allowance.disposition.label(),
                allowance.reason
            ));
        }

        if !expected_keys.contains(&key) {
            failures.push(format!(
                "{}:{}: unexpected wide struct audit state for {}",
                info.path, info.line, info.struct_name
            ));
        }
    }

    for allowance in WIDE_STRUCT_ALLOWANCES {
        let key = (
            allowance.path.to_string(),
            allowance.struct_name.to_string(),
        );
        if !actual_by_key.contains_key(&key) {
            failures.push(format!(
                "{}: stale audit entry for {} ({}, {})",
                allowance.path,
                allowance.struct_name,
                allowance.disposition.label(),
                allowance.reason
            ));
        }
    }

    if !failures.is_empty() {
        panic!("wide struct audit failures:\n- {}", failures.join("\n- "));
    }
}
