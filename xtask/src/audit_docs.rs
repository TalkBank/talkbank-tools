//! Doc-to-code provenance audit — `xtask audit-docs scan`.
//!
//! See `docs/release-doc-audit/audit-method.md` for the workflow this
//! catalogs into. This module is inventory + structural tracking only;
//! claim extraction and citation recording is human/Claude work done in
//! subsequent vetting sessions.

use std::collections::{BTreeSet, HashMap};
use std::env;
use std::fmt;
use std::path::{Path, PathBuf};

use chrono::{Local, NaiveDate};
use regex::Regex;
use rusqlite::{Connection, params};
use walkdir::WalkDir;

use crate::Result;

/// Cutoff: the date of the talkbank-tools / batchalign3 monorepo merge.
/// Docs whose `Last updated` predates this are 'pre-merge' staleness.
const POST_MERGE_BASELINE: &str = "2026-04-28";

const SKIP_DIRS: &[&str] = &[
    ".git",
    ".venv",
    ".venv-314t",
    "__pycache__",
    "build",
    "dist",
    "node_modules",
    "target",
    // Meta-repo embeds talkbank-tools as a sub-clone; skip the nested
    // copy so we don't double-count.
    "talkbank-tools",
    // Audit working files (catalog can't audit itself).
    "release-doc-audit",
    // Frozen review artifacts.
    "batchalign3-review-book",
    // Build outputs that may contain rendered markdown copies.
    "html",
    "site",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Repo {
    TalkbankTools,
    Meta,
}

impl Repo {
    fn as_str(self) -> &'static str {
        match self {
            Repo::TalkbankTools => "talkbank-tools",
            Repo::Meta => "meta",
        }
    }
}

impl fmt::Display for Repo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UpsertOutcome {
    Inserted,
    Updated,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Staleness {
    /// `Last updated` >= POST_MERGE_BASELINE.
    Fresh,
    /// `Last updated` < POST_MERGE_BASELINE; likeliest to reference
    /// deleted crates / moved book paths from the reorgs.
    PreMerge,
    /// No parsable `Last updated` header.
    Unknown,
}

impl Staleness {
    fn as_str(self) -> &'static str {
        match self {
            Staleness::Fresh => "fresh",
            Staleness::PreMerge => "pre-merge",
            Staleness::Unknown => "unknown",
        }
    }
}

#[derive(Debug)]
pub struct Args {
    pub db: PathBuf,
    pub talkbank_tools_root: PathBuf,
    pub meta_root: PathBuf,
}

pub fn run(args: Args) -> Result<()> {
    let mut conn = Connection::open(&args.db)?;
    // WAL + relaxed sync makes the per-doc transaction commits far
    // cheaper without giving up crash-safety against the OS losing
    // the last few writes (which we don't care about — the next scan
    // re-derives identical state from source).
    conn.execute_batch(
        "PRAGMA foreign_keys = ON;
         PRAGMA journal_mode = WAL;
         PRAGMA synchronous = NORMAL;",
    )?;
    apply_migrations(&conn)?;

    let scanned_at = iso_now();

    let roots: [(Repo, &Path); 2] = [
        (Repo::TalkbankTools, args.talkbank_tools_root.as_path()),
        (Repo::Meta, args.meta_root.as_path()),
    ];

    let mut total_docs = 0u64;
    let mut new_docs = 0u64;
    let mut total_sections = 0u64;
    let mut ba2_vs_ba3_docs = 0u64;

    for (repo, root) in roots {
        for path in walk_markdown(root) {
            let rel = match path.strip_prefix(root) {
                Ok(p) => p.to_path_buf(),
                Err(_) => continue,
            };
            let rel_str = rel.to_string_lossy().to_string();

            if rel_str.contains("release-doc-audit/") {
                continue;
            }

            let content = match std::fs::read_to_string(&path) {
                Ok(s) => s,
                Err(_) => continue,
            };

            let doc_hash = blake3_hex(content.as_bytes());
            let (status_label, last_modified_doc) = parse_meta_headers(&content);
            let audience = classify_audience(repo, &rel_str);
            let priority = classify_priority(repo, &rel_str, &content);
            let ba2_vs_ba3 = classify_ba2_vs_ba3(&rel_str, &content);
            let staleness = classify_staleness(last_modified_doc.as_deref());
            if ba2_vs_ba3 {
                ba2_vs_ba3_docs += 1;
            }

            // One transaction per doc keeps the upsert + section
            // reconciliation atomic and amortizes the WAL fsync.
            let tx = conn.transaction()?;
            let (doc_id, outcome) = upsert_doc(
                &tx,
                repo,
                &rel_str,
                &audience,
                priority,
                ba2_vs_ba3,
                staleness,
                status_label.as_deref(),
                last_modified_doc.as_deref(),
                &doc_hash,
                &scanned_at,
            )?;
            if outcome == UpsertOutcome::Inserted {
                new_docs += 1;
            }
            total_docs += 1;

            let sections = parse_sections(&content);
            total_sections += sections.len() as u64;
            sync_sections(&tx, doc_id, &sections)?;
            tx.commit()?;
        }
    }

    println!("xtask audit-docs scan complete:");
    println!("  scanned_at:        {scanned_at}");
    println!("  docs total:        {total_docs}");
    println!("  docs new:          {new_docs}");
    println!("  sections in tree:  {total_sections}");
    println!("  BA2-vs-BA3 docs:   {ba2_vs_ba3_docs}");
    println!();
    print_summary(&conn)?;

    Ok(())
}

// ---------------------------------------------------------------------------
// Walk
// ---------------------------------------------------------------------------

fn walk_markdown(root: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    // SKIP_DIRS applies only at depth > 0 so the walker enters its own
    // root even when the root's last path component matches a skip-name
    // (e.g. walking `/Users/chen/talkbank/talkbank-tools`).
    let walker = WalkDir::new(root).into_iter().filter_entry(|e| {
        if e.depth() == 0 {
            return true;
        }
        let n = e.file_name().to_string_lossy();
        !SKIP_DIRS.iter().any(|s| n == *s)
    });
    for entry in walker.flatten() {
        if !entry.file_type().is_file() {
            continue;
        }
        if entry.path().extension().and_then(|s| s.to_str()) == Some("md") {
            out.push(entry.path().to_path_buf());
        }
    }
    out
}

// ---------------------------------------------------------------------------
// Section parsing
// ---------------------------------------------------------------------------

#[derive(Debug)]
struct ParsedSection {
    level: u8,
    heading: String,
    anchor: String,
    ordinal: i64,
    line_start: i64,
    line_end: i64,
    body_hash: String,
}

fn parse_sections(content: &str) -> Vec<ParsedSection> {
    let lines: Vec<&str> = content.lines().collect();

    // Single-pass: track code-fence state while collecting H2/H3 heads.
    let mut heads: Vec<(usize, u8, String)> = Vec::new();
    let mut in_code_fence = false;
    let mut fence_marker: Option<&'static str> = None;
    for (idx, line) in lines.iter().enumerate() {
        let trimmed = line.trim_start();
        if !in_code_fence {
            if trimmed.starts_with("```") {
                in_code_fence = true;
                fence_marker = Some("```");
                continue;
            }
            if trimmed.starts_with("~~~") {
                in_code_fence = true;
                fence_marker = Some("~~~");
                continue;
            }
            if let Some((level, heading)) = parse_heading(line) {
                if level == 2 || level == 3 {
                    heads.push((idx, level, heading));
                }
            }
        } else if let Some(marker) = fence_marker {
            if trimmed.starts_with(marker) {
                in_code_fence = false;
                fence_marker = None;
            }
        }
    }

    let mut anchor_counts: HashMap<String, usize> = HashMap::new();
    let mut out = Vec::with_capacity(heads.len());
    for (i, (line_idx, level, heading)) in heads.iter().enumerate() {
        // Section ends at the next heading of level <= current OR EOF.
        let mut end_line = lines.len();
        for (j_line, j_level, _) in heads.iter().skip(i + 1) {
            if *j_level <= *level {
                end_line = *j_line;
                break;
            }
        }
        let body_text = lines[(line_idx + 1)..end_line].join("\n");
        let body_hash = blake3_hex(body_text.as_bytes());

        let base_anchor = slugify(heading);
        let count = anchor_counts.entry(base_anchor.clone()).or_insert(0);
        let anchor = if *count == 0 {
            base_anchor.clone()
        } else {
            format!("{base_anchor}-{count}")
        };
        *count += 1;

        out.push(ParsedSection {
            level: *level,
            heading: heading.clone(),
            anchor,
            ordinal: i as i64,
            line_start: (*line_idx as i64) + 1,
            line_end: end_line as i64,
            body_hash,
        });
    }

    out
}

fn parse_heading(line: &str) -> Option<(u8, String)> {
    let trimmed = line.trim_start();
    if !trimmed.starts_with('#') {
        return None;
    }
    let mut level = 0u8;
    let mut chars = trimmed.chars();
    for c in chars.by_ref() {
        if c == '#' {
            level += 1;
            if level > 6 {
                return None;
            }
        } else if c == ' ' {
            break;
        } else {
            return None;
        }
    }
    if level == 0 {
        return None;
    }
    let rest: String = chars.collect();
    let heading = rest.trim().trim_end_matches('#').trim().to_string();
    if heading.is_empty() {
        return None;
    }
    Some((level, heading))
}

/// GitHub-style anchor slug. Approximate; not byte-identical to mdBook
/// in every Unicode edge case. Sufficient for keying sections.
fn slugify(heading: &str) -> String {
    let mut out = String::with_capacity(heading.len());
    for c in heading.chars() {
        if c.is_ascii_alphanumeric() {
            out.push(c.to_ascii_lowercase());
        } else if c == ' ' || c == '-' || c == '_' {
            out.push('-');
        }
    }
    let mut collapsed = String::with_capacity(out.len());
    let mut prev_dash = false;
    for c in out.chars() {
        if c == '-' {
            if !prev_dash && !collapsed.is_empty() {
                collapsed.push(c);
            }
            prev_dash = true;
        } else {
            collapsed.push(c);
            prev_dash = false;
        }
    }
    collapsed.trim_end_matches('-').to_string()
}

// ---------------------------------------------------------------------------
// Metadata extraction
// ---------------------------------------------------------------------------

fn parse_meta_headers(content: &str) -> (Option<String>, Option<String>) {
    let mut status = None;
    let mut last_modified = None;
    for line in content.lines().take(30) {
        if status.is_none() {
            if let Some(v) = extract_field(line, "Status") {
                status = Some(v);
            }
        }
        if last_modified.is_none() {
            if let Some(v) = extract_field(line, "Last updated") {
                last_modified = Some(v);
            } else if let Some(v) = extract_field(line, "Last modified") {
                last_modified = Some(v);
            }
        }
        if status.is_some() && last_modified.is_some() {
            break;
        }
    }
    (status, last_modified)
}

fn extract_field(line: &str, name: &str) -> Option<String> {
    let bold = format!("**{}:**", name);
    if let Some(idx) = line.find(&bold) {
        let v = line[idx + bold.len()..].trim();
        if !v.is_empty() {
            return Some(v.to_string());
        }
    }
    let plain = format!("{}:", name);
    if line.trim_start().starts_with(&plain) {
        let v = line.trim_start()[plain.len()..].trim();
        if !v.is_empty() {
            return Some(v.to_string());
        }
    }
    None
}

// ---------------------------------------------------------------------------
// Classification heuristics
// ---------------------------------------------------------------------------

fn classify_audience(repo: Repo, rel: &str) -> String {
    if rel.ends_with("CLAUDE.md") {
        return "dev".to_string();
    }
    if repo == Repo::Meta {
        return "dev".to_string();
    }
    if rel.starts_with("book/src/")
        && (rel.contains("/user-guide/")
            || rel.contains("/getting-started/")
            || rel.contains("/install/")
            || rel.contains("/quickstart/")
            || rel.contains("/chat-format/"))
    {
        return "user".to_string();
    }
    if rel.starts_with("book/src/")
        && (rel.contains("/architecture/")
            || rel.contains("/developer/")
            || rel.contains("/contributing/")
            || rel.contains("/decisions/"))
    {
        return "dev".to_string();
    }
    if rel.starts_with("book/src/") {
        return "mixed".to_string();
    }
    if rel == "README.md"
        || rel.ends_with("/README.md")
        || rel == "CONTRIBUTING.md"
        || rel == "SECURITY.md"
        || rel == "INTERFACE_MAP.md"
    {
        return "mixed".to_string();
    }
    if rel.starts_with("docs/") {
        return "dev".to_string();
    }
    "mixed".to_string()
}

fn classify_priority(repo: Repo, rel: &str, content: &str) -> i64 {
    if classify_ba2_vs_ba3(rel, content) {
        return 1;
    }
    if rel == "README.md" || rel == "CONTRIBUTING.md" || rel == "SECURITY.md" {
        return 1;
    }
    if rel.ends_with("CLAUDE.md") {
        return 2;
    }
    if rel.starts_with("book/src/")
        && (rel.contains("/user-guide/")
            || rel.contains("/getting-started/")
            || rel.contains("/quickstart/")
            || rel.contains("/install/"))
    {
        return 2;
    }
    if rel.starts_with("book/src/") {
        return 3;
    }
    if rel.starts_with("docs/") {
        if repo == Repo::Meta {
            return 5;
        }
        return 4;
    }
    4
}

/// Compare the doc's `Last updated:` header to the post-merge
/// baseline. Header format is operator-controlled (typically
/// `YYYY-MM-DD HH:MM TZ`); we look for an ISO-style date prefix and
/// fall back to `Unknown` for anything else.
fn classify_staleness(last_modified: Option<&str>) -> Staleness {
    let raw = match last_modified {
        Some(s) => s.trim(),
        None => return Staleness::Unknown,
    };
    let prefix = raw.get(..10).unwrap_or("");
    let parsed = NaiveDate::parse_from_str(prefix, "%Y-%m-%d");
    let cutoff = NaiveDate::parse_from_str(POST_MERGE_BASELINE, "%Y-%m-%d");
    match (parsed, cutoff) {
        (Ok(d), Ok(c)) if d >= c => Staleness::Fresh,
        (Ok(_), Ok(_)) => Staleness::PreMerge,
        _ => Staleness::Unknown,
    }
}

fn classify_ba2_vs_ba3(rel: &str, content: &str) -> bool {
    if rel.contains("/migration/") || rel.contains("ba2-") || rel.contains("BA2") {
        return true;
    }
    let mut limit = content.len().min(4096);
    while limit > 0 && !content.is_char_boundary(limit) {
        limit -= 1;
    }
    let head = &content[..limit];
    head.contains("BA2 ") || head.contains("Batchalign2") || head.contains("batchalign2")
}

// ---------------------------------------------------------------------------
// SQLite upserts
// ---------------------------------------------------------------------------

#[allow(clippy::too_many_arguments)]
fn upsert_doc(
    conn: &Connection,
    repo: Repo,
    path: &str,
    audience: &str,
    priority: i64,
    ba2_vs_ba3: bool,
    staleness: Staleness,
    status_label: Option<&str>,
    last_modified_doc: Option<&str>,
    content_hash: &str,
    scanned_at: &str,
) -> Result<(i64, UpsertOutcome)> {
    // Two-step rather than RETURNING: SQLite has no direct "was-this-an-
    // insert-or-an-update" signal, so we look up the prior id once and
    // report new-vs-existing from that.
    let existing: Option<i64> = conn
        .query_row(
            "SELECT id FROM docs WHERE repo = ? AND path = ?",
            params![repo.as_str(), path],
            |row| row.get(0),
        )
        .ok();
    let outcome = match existing {
        Some(_) => UpsertOutcome::Updated,
        None => UpsertOutcome::Inserted,
    };
    conn.execute(
        "INSERT INTO docs
            (repo, path, audience, priority, ba2_vs_ba3, staleness,
             status_label, last_modified_doc, content_hash, scanned_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
         ON CONFLICT(repo, path) DO UPDATE SET
            audience = excluded.audience,
            priority = excluded.priority,
            ba2_vs_ba3 = excluded.ba2_vs_ba3,
            staleness = excluded.staleness,
            status_label = excluded.status_label,
            last_modified_doc = excluded.last_modified_doc,
            content_hash = excluded.content_hash,
            scanned_at = excluded.scanned_at",
        params![
            repo.as_str(),
            path,
            audience,
            priority,
            i64::from(ba2_vs_ba3),
            staleness.as_str(),
            status_label,
            last_modified_doc,
            content_hash,
            scanned_at,
        ],
    )?;
    let id: i64 = conn.query_row(
        "SELECT id FROM docs WHERE repo = ? AND path = ?",
        params![repo.as_str(), path],
        |row| row.get(0),
    )?;
    Ok((id, outcome))
}

/// Reconcile section rows for one doc against the freshly-parsed list.
/// Anchors absent from the new parse are deleted (cascading their claims
/// and citations). Anchors with unchanged `content_hash` keep their
/// `vet_state`; changed content resets to `unvetted` while preserving
/// the row id so attached claims/citations remain available for diffing.
fn sync_sections(conn: &Connection, doc_id: i64, sections: &[ParsedSection]) -> Result<()> {
    // One SELECT per doc fetches every existing (anchor, content_hash);
    // the per-section logic then runs entirely from this map. Avoids
    // the previous N+1 SELECT pattern.
    let existing: HashMap<String, String> = {
        let mut stmt =
            conn.prepare("SELECT anchor, content_hash FROM sections WHERE doc_id = ?")?;
        let rows = stmt.query_map([doc_id], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?;
        rows.collect::<rusqlite::Result<HashMap<String, String>>>()?
    };
    let existing_anchors: BTreeSet<&str> = existing.keys().map(String::as_str).collect();
    let current_anchors: BTreeSet<&str> = sections.iter().map(|s| s.anchor.as_str()).collect();

    for anchor in existing_anchors.difference(&current_anchors) {
        conn.execute(
            "DELETE FROM sections WHERE doc_id = ? AND anchor = ?",
            params![doc_id, anchor],
        )?;
    }

    for s in sections {
        match existing.get(&s.anchor) {
            None => {
                conn.execute(
                    "INSERT INTO sections
                        (doc_id, level, heading, anchor, ordinal,
                         line_start, line_end, content_hash)
                     VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
                    params![
                        doc_id,
                        i64::from(s.level),
                        s.heading,
                        s.anchor,
                        s.ordinal,
                        s.line_start,
                        s.line_end,
                        s.body_hash,
                    ],
                )?;
            }
            Some(prev_hash) if *prev_hash == s.body_hash => {
                conn.execute(
                    "UPDATE sections SET
                        level = ?, heading = ?, ordinal = ?,
                        line_start = ?, line_end = ?
                     WHERE doc_id = ? AND anchor = ?",
                    params![
                        i64::from(s.level),
                        s.heading,
                        s.ordinal,
                        s.line_start,
                        s.line_end,
                        doc_id,
                        s.anchor,
                    ],
                )?;
            }
            Some(_) => {
                conn.execute(
                    "UPDATE sections SET
                        level = ?, heading = ?, ordinal = ?,
                        line_start = ?, line_end = ?,
                        content_hash = ?,
                        vet_state = 'unvetted',
                        reviewer = NULL,
                        reviewed_at = NULL
                     WHERE doc_id = ? AND anchor = ?",
                    params![
                        i64::from(s.level),
                        s.heading,
                        s.ordinal,
                        s.line_start,
                        s.line_end,
                        s.body_hash,
                        doc_id,
                        s.anchor,
                    ],
                )?;
            }
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Migrations
// ---------------------------------------------------------------------------

/// Apply additive schema migrations on every run. Idempotent: each
/// step checks the current state via `pragma_table_info` before
/// adding columns, and `CREATE TABLE IF NOT EXISTS` covers new tables.
///
/// This lets a databases created with an older version of the schema
/// pick up newer columns without a manual migration step.
fn apply_migrations(conn: &Connection) -> Result<()> {
    if !column_exists(conn, "docs", "staleness")? {
        conn.execute_batch(
            "ALTER TABLE docs ADD COLUMN staleness TEXT NOT NULL DEFAULT 'unknown';",
        )?;
    }
    if !column_exists(conn, "sections", "fix_commit_hash")? {
        conn.execute_batch("ALTER TABLE sections ADD COLUMN fix_commit_hash TEXT;")?;
    }
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS staleness_flags (
            id INTEGER PRIMARY KEY,
            section_id INTEGER NOT NULL REFERENCES sections(id) ON DELETE CASCADE,
            pattern_name TEXT NOT NULL,
            pattern_severity TEXT NOT NULL,
            match_line INTEGER,
            match_excerpt TEXT,
            flagged_at TEXT NOT NULL
         );
         CREATE INDEX IF NOT EXISTS idx_staleness_flags_section
            ON staleness_flags(section_id);
         CREATE INDEX IF NOT EXISTS idx_staleness_flags_pattern
            ON staleness_flags(pattern_name);",
    )?;
    Ok(())
}

fn column_exists(conn: &Connection, table: &str, column: &str) -> Result<bool> {
    let mut stmt = conn.prepare("SELECT 1 FROM pragma_table_info(?) WHERE name = ?")?;
    let exists: Option<i64> = stmt
        .query_row(params![table, column], |row| row.get(0))
        .ok();
    Ok(exists.is_some())
}

// ---------------------------------------------------------------------------
// Staleness flagging (xtask audit-docs flag-staleness)
// ---------------------------------------------------------------------------

struct FlagPattern {
    name: &'static str,
    severity: &'static str,
    /// Regex to match against each line of every section body.
    pattern: &'static str,
}

const FLAG_PATTERNS: &[FlagPattern] = &[
    FlagPattern {
        name: "deleted-crate-chat-ops",
        severity: "high",
        pattern: r"\bbatchalign-chat-ops\b",
    },
    FlagPattern {
        name: "deleted-crate-batchalign-app",
        severity: "high",
        pattern: r"\bbatchalign-app\b",
    },
    FlagPattern {
        name: "deleted-crate-batchalign-revai",
        severity: "high",
        pattern: r"\bbatchalign-revai\b",
    },
    FlagPattern {
        name: "deleted-crate-talkbank-redact",
        severity: "high",
        pattern: r"\btalkbank-redact\b",
    },
    FlagPattern {
        name: "renamed-crate-talkbank-re2c-parser",
        severity: "medium",
        pattern: r"\btalkbank-re2c-parser\b",
    },
    FlagPattern {
        name: "moved-book-path-batchalign-book",
        severity: "high",
        pattern: r"\bbatchalign-book/",
    },
    FlagPattern {
        name: "moved-book-path-vscode-book",
        severity: "high",
        pattern: r"\bvscode/book/",
    },
    FlagPattern {
        name: "moved-book-path-clan-book",
        severity: "high",
        pattern: r"\bcrates/talkbank-clan/book/",
    },
    FlagPattern {
        name: "absolute-crate-count",
        severity: "medium",
        pattern: r"\b(\d+) crates\b",
    },
    FlagPattern {
        name: "absolute-test-count",
        severity: "low",
        pattern: r"\b(\d+) tests\b",
    },
    FlagPattern {
        name: "absolute-file-count",
        severity: "low",
        pattern: r"\b(\d+) (?:files|sections|chapters|pages)\b",
    },
    FlagPattern {
        name: "phase-still-pending",
        severity: "medium",
        // P1-P10 have all landed; treat phase mentions as suspect.
        pattern: r"\b(?:P[1-9]|P10)\s+(?:remains|is|will be|TBD|pending)\b",
    },
];

pub fn run_flag_staleness(args: Args) -> Result<()> {
    let conn = Connection::open(&args.db)?;
    conn.execute_batch(
        "PRAGMA foreign_keys = ON;
         PRAGMA journal_mode = WAL;
         PRAGMA synchronous = NORMAL;",
    )?;
    apply_migrations(&conn)?;

    let regexes: Vec<(usize, Regex)> = FLAG_PATTERNS
        .iter()
        .enumerate()
        .map(|(i, p)| Ok((i, Regex::new(p.pattern)?)))
        .collect::<Result<Vec<_>>>()?;

    // Wipe and rebuild — patterns can change between runs and
    // ambiguous "what was here last run?" semantics aren't worth
    // keeping. Cheap operation against an indexed table.
    conn.execute("DELETE FROM staleness_flags", [])?;

    // Walk every section, re-read its source file, scan body lines.
    let now = iso_now();
    let mut stmt = conn.prepare(
        "SELECT s.id, s.line_start, s.line_end, d.repo, d.path
         FROM sections s JOIN docs d ON d.id = s.doc_id",
    )?;
    let rows: Vec<(i64, i64, i64, String, String)> = stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, i64>(1)?,
                row.get::<_, i64>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, String>(4)?,
            ))
        })?
        .collect::<rusqlite::Result<_>>()?;
    drop(stmt);

    // Cache file contents per (repo, path) so we don't re-read for
    // every section in the same file.
    let mut file_cache: HashMap<(String, String), Option<Vec<String>>> = HashMap::new();

    let tx = conn.unchecked_transaction()?;
    let mut total_flags = 0u64;
    for (section_id, line_start, line_end, repo, rel_path) in rows {
        let key = (repo.clone(), rel_path.clone());
        let lines = file_cache.entry(key).or_insert_with(|| {
            let root: &Path = match repo.as_str() {
                "talkbank-tools" => args.talkbank_tools_root.as_path(),
                "meta" => args.meta_root.as_path(),
                _ => return None,
            };
            std::fs::read_to_string(root.join(&rel_path))
                .ok()
                .map(|s| s.lines().map(str::to_owned).collect())
        });
        let Some(lines) = lines else { continue };
        if lines.is_empty() {
            continue;
        }

        // Bound to body lines only: heading is at line_start (1-based);
        // body runs from line_start+1 to line_end (inclusive).
        let body_start = line_start as usize; // 0-based offset of the line AFTER the heading
        let body_end = (line_end as usize).min(lines.len());
        if body_start >= body_end {
            continue;
        }

        for body_idx in body_start..body_end {
            let line = &lines[body_idx];
            for (i, regex) in &regexes {
                let pat = &FLAG_PATTERNS[*i];
                if let Some(m) = regex.find(line) {
                    let mut excerpt = m.as_str().to_string();
                    if excerpt.len() > 200 {
                        // char-boundary safe truncate
                        let mut end = 200;
                        while end > 0 && !excerpt.is_char_boundary(end) {
                            end -= 1;
                        }
                        excerpt.truncate(end);
                    }
                    tx.execute(
                        "INSERT INTO staleness_flags
                            (section_id, pattern_name, pattern_severity,
                             match_line, match_excerpt, flagged_at)
                         VALUES (?, ?, ?, ?, ?, ?)",
                        params![
                            section_id,
                            pat.name,
                            pat.severity,
                            (body_idx as i64) + 1,
                            excerpt,
                            now,
                        ],
                    )?;
                    total_flags += 1;
                }
            }
        }
    }
    tx.commit()?;

    println!("xtask audit-docs flag-staleness complete:");
    println!("  flags inserted: {total_flags}");
    println!();
    print_flag_summary(&conn)?;
    Ok(())
}

fn print_flag_summary(conn: &Connection) -> Result<()> {
    println!("By pattern (top hits):");
    let mut stmt = conn.prepare(
        "SELECT pattern_name, pattern_severity, COUNT(*) AS hits,
                COUNT(DISTINCT section_id) AS sections
         FROM staleness_flags
         GROUP BY pattern_name
         ORDER BY hits DESC",
    )?;
    let rows = stmt.query_map([], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, i64>(2)?,
            row.get::<_, i64>(3)?,
        ))
    })?;
    for row in rows {
        let (pattern, severity, hits, sections) = row?;
        println!("  {hits:>5} hits in {sections:>4} sections  [{severity}]  {pattern}");
    }
    println!();

    println!("Top 10 sections by flag count:");
    let mut stmt = conn.prepare(
        "SELECT d.repo, d.path, s.heading, COUNT(*) AS flag_count
         FROM staleness_flags f
         JOIN sections s ON s.id = f.section_id
         JOIN docs d ON d.id = s.doc_id
         GROUP BY f.section_id
         ORDER BY flag_count DESC
         LIMIT 10",
    )?;
    let rows = stmt.query_map([], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
            row.get::<_, i64>(3)?,
        ))
    })?;
    for row in rows {
        let (repo, path, heading, flag_count) = row?;
        println!("  {flag_count:>3}  [{repo}] {path} :: {heading}");
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Reporting
// ---------------------------------------------------------------------------

fn print_summary(conn: &Connection) -> Result<()> {
    println!("By audience / priority bucket:");
    let mut stmt = conn.prepare(
        "SELECT audience, priority, COUNT(*)
         FROM docs
         GROUP BY audience, priority
         ORDER BY priority, audience",
    )?;
    let rows = stmt.query_map([], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, i64>(1)?,
            row.get::<_, i64>(2)?,
        ))
    })?;
    for row in rows {
        let (audience, priority, count) = row?;
        println!("  P{priority}  {audience:<10}  {count:>5} docs");
    }
    println!();

    let total_sections: i64 =
        conn.query_row("SELECT COUNT(*) FROM sections", [], |row| row.get(0))?;
    let unvetted: i64 = conn.query_row(
        "SELECT COUNT(*) FROM sections WHERE vet_state = 'unvetted'",
        [],
        |row| row.get(0),
    )?;
    println!("Sections: {total_sections} total, {unvetted} unvetted");

    let ba2_docs: i64 = conn.query_row(
        "SELECT COUNT(*) FROM docs WHERE ba2_vs_ba3 = 1",
        [],
        |row| row.get(0),
    )?;
    println!("BA2-vs-BA3 docs flagged: {ba2_docs}");

    println!();
    println!("Top 20 BA2-vs-BA3 sections by body line count:");
    let mut stmt = conn.prepare(
        "SELECT d.repo, d.path, s.heading, (s.line_end - s.line_start) AS body_lines
         FROM sections s
         JOIN docs d ON d.id = s.doc_id
         WHERE d.ba2_vs_ba3 = 1
         ORDER BY body_lines DESC
         LIMIT 20",
    )?;
    let rows = stmt.query_map([], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
            row.get::<_, i64>(3)?,
        ))
    })?;
    for row in rows {
        let (repo, path, heading, body_lines) = row?;
        println!("  {body_lines:>4}  [{repo}] {path} :: {heading}");
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn blake3_hex(bytes: &[u8]) -> String {
    blake3::hash(bytes).to_hex().to_string()
}

fn iso_now() -> String {
    Local::now().format("%Y-%m-%d %H:%M %Z").to_string()
}

// ---------------------------------------------------------------------------
// CLI dispatch
// ---------------------------------------------------------------------------

/// Parse `audit-docs` CLI flags. Path defaults are read from environment
/// variables — the public-bound `talkbank-tools` repo must not carry
/// hardcoded operator-local paths in source. Required env vars:
///   TB_AUDIT_DB         — path to the SQLite catalog
///   TB_AUDIT_TT_ROOT    — path to the talkbank-tools clone
///   TB_AUDIT_META_ROOT  — path to the meta-repo workspace
/// Each can be overridden by the matching --flag on the command line.
pub fn parse_and_run(rest: Vec<String>) -> Result<()> {
    let usage = "usage: cargo run -q -p xtask -- audit-docs <scan|flag-staleness> \
         [--db PATH] [--talkbank-tools PATH] [--meta PATH]\n\
         (or set TB_AUDIT_DB / TB_AUDIT_TT_ROOT / TB_AUDIT_META_ROOT)";

    let sub = rest.first().map(|s| s.as_str()).ok_or(usage)?;
    if sub != "scan" && sub != "flag-staleness" {
        return Err(usage.into());
    }

    let mut db: Option<PathBuf> = env::var_os("TB_AUDIT_DB").map(PathBuf::from);
    let mut tt_root: Option<PathBuf> = env::var_os("TB_AUDIT_TT_ROOT").map(PathBuf::from);
    let mut meta_root: Option<PathBuf> = env::var_os("TB_AUDIT_META_ROOT").map(PathBuf::from);

    let mut iter = rest.iter().skip(1);
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--db" => {
                db = Some(PathBuf::from(iter.next().ok_or("--db requires a value")?));
            }
            "--talkbank-tools" => {
                tt_root = Some(PathBuf::from(
                    iter.next().ok_or("--talkbank-tools requires a value")?,
                ));
            }
            "--meta" => {
                meta_root = Some(PathBuf::from(iter.next().ok_or("--meta requires a value")?));
            }
            other => return Err(format!("unknown audit-docs flag: {other}").into()),
        }
    }

    let db = db.ok_or("audit-docs: --db or TB_AUDIT_DB is required (no default)")?;
    let tt_root = tt_root.ok_or("audit-docs: --talkbank-tools or TB_AUDIT_TT_ROOT is required")?;
    let meta_root = meta_root.ok_or("audit-docs: --meta or TB_AUDIT_META_ROOT is required")?;

    let args = Args {
        db,
        talkbank_tools_root: tt_root,
        meta_root,
    };
    match sub {
        "scan" => run(args),
        "flag-staleness" => run_flag_staleness(args),
        _ => unreachable!(),
    }
}
