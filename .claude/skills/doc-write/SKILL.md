---
name: doc-write
description: Write new documentation following TalkBank project conventions. Use when the user wants to create a new doc page, add a section to the mdBook, write a postmortem, add an error code doc, or create a new CLAUDE.md.
disable-model-invocation: true
allowed-tools: Bash, Read, Write, Edit, Glob, Grep
---

# Write New Documentation

Create new documentation following the appropriate conventions for the target location. `$ARGUMENTS` should describe what to document and where.

## Step 1: Determine the Doc Type and Location

| Doc Type | Where | Format |
|----------|-------|--------|
| CHAT format / parser architecture | `talkbank-tools/book/src/` | mdBook page |
| Batchalign user guide / architecture | `batchalign3/book/src/` | mdBook page |
| VS Code extension user docs | `talkbank-tools/vscode/` | Plain markdown |
| LSP / CLI developer docs | `talkbank-tools/crates/*/` or `docs/` | Plain markdown |
| Error code reference | `talkbank-tools/spec/errors/` (source) | Spec markdown (auto-generates doc) |
| Operations / infrastructure | project archive (talkbank-dev) | Plain markdown |
| Incident / postmortem | project archive (talkbank-dev) | Plain markdown |
| AI context | `<repo>/CLAUDE.md` or `<crate>/CLAUDE.md` | Plain markdown |
| Design decision (ADR) | `batchalign3/book/src/decisions/` | mdBook page |

## Step 2: Read Existing Examples

Before writing, read 2-3 existing docs in the same location to match conventions:

```bash
# mdBook page
ls $WORKSPACE/<repo>/book/src/<section>/
cat $WORKSPACE/<repo>/book/src/<section>/<existing>.md | head -40

# VS Code docs (book)
ls $REPO_ROOT/vscode/book/src/
cat $REPO_ROOT/vscode/book/src/getting-started/installation.md | head -40
```

## Step 3: Follow Conventions

### mdBook Pages

Structure:
```markdown
# Page Title

Brief introduction (1-2 paragraphs).

## Section Heading

Content with tables, code blocks, diagrams as needed.

## See Also

- [Related Page](../other-section/page.md)
```

After creating, register in SUMMARY.md:
```bash
# Read current SUMMARY
cat $WORKSPACE/<repo>/book/src/SUMMARY.md

# Add entry in appropriate section
# Format: - [Page Title](section/filename.md)
```

### VS Code Extension Docs

All user, developer, and integrator documentation for the VS Code
extension is an mdBook at `vscode/book/`. Do **not** create parallel
loose `.md` files alongside `README.md` in `vscode/`; every
non-trivial addition belongs in a book chapter.

- `vscode/book/src/introduction.md` — entry point
- `vscode/book/src/SUMMARY.md` — table of contents (add new pages here)
- `vscode/book/src/getting-started/` — installation, first-file, quick reference
- `vscode/book/src/editing/` · `navigation/` · `media/` · `analysis/` · `review/` · `coder/` · `workflows/` · `configuration/` · `troubleshooting/` — user-facing feature chapters
- `vscode/book/src/developer/` — architecture, LSP protocol, custom commands, testing, releasing, CLAN parity
- `vscode/README.md` — marketplace summary only; keep short and point at the book

Convention: every page starts with `**Status:**` + `**Last updated:** YYYY-MM-DD HH:MM TZ` headers (run `date '+%Y-%m-%d %H:%M %Z'`).

### Error Code Docs

**NEVER write error code docs directly.** Instead create a spec file:

```markdown
# E###: ErrorName

## Description
What this error means.

## Metadata
- **Error Code**: E###
- **Layer**: parser | validation
- **Status**: implemented | not_implemented

## Example 1
**Trigger**: Description of what triggers the error.

```chat
@UTF8
@Begin
...
@End
```
```

Then run `make test-gen` to generate the doc automatically.

### Postmortem / Incident Report

Convention (from existing postmortems):
```markdown
# Postmortem: <Title> (<Date>)

## Summary
One paragraph.

## Timeline
- **HH:MM UTC**: Event description
- **HH:MM UTC**: Next event

## Impact
- Duration: X minutes
- Jobs affected: N
- Data loss: none / description

## Root Cause
Detailed technical explanation.

## Remediation
- [x] Immediate fix applied
- [ ] Long-term fix planned

## Lessons Learned
What to improve.
```

### Design Decision (ADR)

Convention (from batchalign3 decisions):
```markdown
# Decision: <Title>

_Date: YYYY-MM-DD_

## Context
What prompted this decision.

## Options Considered
### Option A: ...
Pros/cons.

### Option B: ...
Pros/cons.

## Decision
What we chose and why.

## Consequences
What changes as a result.
```

### CLAUDE.md Files

Convention:
```markdown
# CLAUDE.md — <Component Name>

## Overview
One paragraph describing what this component does.

## Architecture
Key design points, dependency graph.

## Build & Test
```bash
# exact commands
```

## Coding Standards
Project-specific rules.

## Gotchas
Common pitfalls and non-obvious behavior.

_Last Updated: YYYY-MM-DD_
```

## Step 4: Write the Document

Create the file using the Write tool. Follow the conventions discovered in Step 2.

## Step 5: Register (if mdBook)

If adding an mdBook page, update SUMMARY.md:

```bash
# Edit the appropriate SUMMARY.md to add the new page
```

## Step 6: Cross-Reference

Check if other docs should link to the new page:
- README.md files
- CLAUDE.md files
- Related mdBook pages (add to "See Also" sections)
- Project archive (talkbank-dev) if significant new doc

## Step 7: Verify (if mdBook)

```bash
cd $WORKSPACE/<repo>/book && mdbook build
```

If mdbook is not installed, at minimum verify the markdown renders correctly and all links resolve.
