#!/usr/bin/env python3
"""
Audit documentation coverage in talkbank-model/src/model.

Identifies:
1. All public structs/enums
2. Which have CHAT manual references
3. Which need documentation
"""

import re
import sys
from pathlib import Path
from dataclasses import dataclass
from typing import List, Optional

@dataclass
class RustType:
    """Type representing RustType."""
    name: str
    kind: str  # 'struct' or 'enum'
    file: Path
    line_num: int
    has_doc: bool
    has_chat_ref: bool
    doc_preview: Optional[str] = None

def extract_types_from_file(file_path: Path) -> List[RustType]:
    """Extract all public struct/enum definitions with their documentation."""

    types = []

    with open(file_path, 'r', encoding='utf-8') as f:
        lines = f.readlines()

    i = 0
    while i < len(lines):
        line = lines[i]

        # Look for pub struct/enum
        match = re.match(r'^pub\s+(struct|enum)\s+(\w+)', line)
        if match:
            kind = match.group(1)
            name = match.group(2)

            # Look backwards for doc comments
            doc_lines = []
            j = i - 1
            while j >= 0 and (lines[j].strip().startswith('///') or
                             lines[j].strip().startswith('#[')):
                if lines[j].strip().startswith('///'):
                    doc_lines.insert(0, lines[j].strip()[3:].strip())
                j -= 1

            has_doc = len(doc_lines) > 0
            has_chat_ref = any('talkbank.org' in line or 'CHAT.html' in line
                              for line in doc_lines)

            doc_preview = ' '.join(doc_lines[:2]) if doc_lines else None

            types.append(RustType(
                name=name,
                kind=kind,
                file=file_path,
                line_num=i + 1,
                has_doc=has_doc,
                has_chat_ref=has_chat_ref,
                doc_preview=doc_preview
            ))

        i += 1

    return types

def audit_model_directory(model_dir: Path) -> List[RustType]:
    """Scan all Rust files in model directory."""

    all_types = []

    for rs_file in model_dir.rglob('*.rs'):
        # Skip test files
        if 'tests' in rs_file.parts or rs_file.name == 'tests.rs':
            continue

        types = extract_types_from_file(rs_file)
        all_types.extend(types)

    return all_types

def print_audit_report(types: List[RustType], model_dir: Path):
    """Print comprehensive audit report."""

    print("=" * 80)
    print("DOCUMENTATION AUDIT: talkbank-model/src/model")
    print("=" * 80)
    print()

    total = len(types)
    with_doc = sum(1 for t in types if t.has_doc)
    with_chat_ref = sum(1 for t in types if t.has_chat_ref)

    print(f"Total public types: {total}")
    print(f"With documentation: {with_doc} ({with_doc*100//total if total else 0}%)")
    print(f"With CHAT manual refs: {with_chat_ref} ({with_chat_ref*100//total if total else 0}%)")
    print()

    # Group by category
    categories = {}
    for t in types:
        rel_path = t.file.relative_to(model_dir)
        category = rel_path.parts[0] if len(rel_path.parts) > 1 else 'root'
        if category not in categories:
            categories[category] = []
        categories[category].append(t)

    # Print by category
    for category in sorted(categories.keys()):
        cat_types = categories[category]
        cat_with_chat = sum(1 for t in cat_types if t.has_chat_ref)

        print(f"\n## {category.upper()} ({len(cat_types)} types, {cat_with_chat} with CHAT refs)")
        print("-" * 80)

        # Prioritize types without CHAT refs
        needs_doc = [t for t in cat_types if not t.has_chat_ref]
        has_chat = [t for t in cat_types if t.has_chat_ref]

        if needs_doc:
            print("\n  NEEDS CHAT MANUAL REFERENCE:")
            for t in sorted(needs_doc, key=lambda x: x.name):
                status = "📝" if t.has_doc else "❌"
                rel_file = t.file.relative_to(model_dir)
                print(f"    {status} {t.name:30} ({t.kind:6}) {rel_file}:{t.line_num}")
                if t.doc_preview:
                    preview = t.doc_preview[:70] + "..." if len(t.doc_preview) > 70 else t.doc_preview
                    print(f"       → {preview}")

        if has_chat:
            print("\n  ✅ HAS CHAT REFERENCE:")
            for t in sorted(has_chat, key=lambda x: x.name)[:5]:  # Show first 5
                rel_file = t.file.relative_to(model_dir)
                print(f"       {t.name:30} ({t.kind:6}) {rel_file}:{t.line_num}")
            if len(has_chat) > 5:
                print(f"       ... and {len(has_chat) - 5} more")

    print("\n\n" + "=" * 80)
    print("PRIORITY LIST: Types needing CHAT manual references")
    print("=" * 80)
    print()

    # High priority types (common/important)
    high_priority_names = [
        'Word', 'Group', 'Pause', 'Event', 'Action',
        'MorTier', 'GraTier', 'PhoTier', 'SinTier', 'ActTier', 'CodTier',
        'ComTier', 'ExpTier', 'AddTier',
        'Participant', 'ReplacedWord', 'Replacement',
        'ScopedAnnotation', 'SpecialForm', 'Terminator'
    ]

    high_priority = [t for t in types if t.name in high_priority_names and not t.has_chat_ref]

    if high_priority:
        print("HIGH PRIORITY (core types):")
        for t in sorted(high_priority, key=lambda x: x.name):
            rel_file = t.file.relative_to(model_dir)
            print(f"  • {t.name:30} {rel_file}:{t.line_num}")
        print()

    # All others without CHAT refs
    others = [t for t in types if t not in high_priority and not t.has_chat_ref]
    if others:
        print(f"\nOTHER TYPES ({len(others)} total):")
        for t in sorted(others, key=lambda x: (x.file, x.name))[:20]:
            rel_file = t.file.relative_to(model_dir)
            print(f"  • {t.name:30} {rel_file}:{t.line_num}")
        if len(others) > 20:
            print(f"  ... and {len(others) - 20} more")

def main():
    """Perform main."""
    model_dir = Path(__file__).parent.parent / "crates" / "talkbank-model" / "src" / "model"

    if not model_dir.exists():
        print(f"Error: Model directory not found at {model_dir}", file=sys.stderr)
        sys.exit(1)

    types = audit_model_directory(model_dir)
    print_audit_report(types, model_dir)

    # Write detailed report
    output_path = Path(__file__).parent.parent / "docs" / "reference" / "DOCUMENTATION_AUDIT.md"
    output_path.parent.mkdir(parents=True, exist_ok=True)

    with open(output_path, 'w') as f:
        original_stdout = sys.stdout
        sys.stdout = f
        print_audit_report(types, model_dir)
        sys.stdout = original_stdout

    print(f"\n\nDetailed report written to: {output_path}")

if __name__ == "__main__":
    main()
