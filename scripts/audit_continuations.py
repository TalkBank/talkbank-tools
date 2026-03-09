#!/usr/bin/env python3
r"""Audit grammar regexes to ensure continuations are not swallowed.

Rules enforced:
- Only the rules named 'continuation' and 'newline' may match \r or \n.
- No regex may include \s unless it is inside a *negated* character class.
- \r or \n inside a negated character class is allowed (it excludes newlines).
"""

from __future__ import annotations

import re
import sys
from dataclasses import dataclass
from pathlib import Path
from typing import Iterable


GRAMMAR_PATH = Path("grammar/grammar.js")
ALLOWED_RULES = {"continuation", "newline"}


@dataclass(frozen=True)
class RegexLiteral:
    """Type representing RegexLiteral."""
    name: str
    pattern: str
    line: int
    column: int


def find_rule_definitions(lines: list[str]) -> list[tuple[int, str]]:
    """Perform find rule definitions."""
    rule_defs: list[tuple[int, str]] = []
    for idx, line in enumerate(lines, 1):
        match = re.match(r"\s*([a-zA-Z_][a-zA-Z0-9_]*)\s*:\s*\$\s*=>", line)
        if match:
            rule_defs.append((idx, match.group(1)))
    return rule_defs


def nearest_rule(rule_defs: list[tuple[int, str]], line_no: int) -> str:
    """Perform nearest rule."""
    rule_name = "<unknown>"
    for def_line, name in rule_defs:
        if def_line <= line_no:
            rule_name = name
        else:
            break
    return rule_name


def extract_regex_literals(text: str, lines: list[str]) -> list[RegexLiteral]:
    """Perform extract regex literals."""
    literals: list[RegexLiteral] = []

    # Find rule definitions to associate regex literals with rule names.
    rule_defs = find_rule_definitions(lines)

    # Scan for JS regex literals.
    in_single = False
    in_double = False
    in_template = False
    in_line_comment = False
    in_block_comment = False
    escaped = False
    prev_non_ws = ""

    idx = 0
    line = 1
    col = 1

    def advance(ch: str) -> None:
        """Perform advance."""
        nonlocal line, col
        if ch == "\n":
            line += 1
            col = 1
        else:
            col += 1

    while idx < len(text):
        ch = text[idx]
        nxt = text[idx + 1] if idx + 1 < len(text) else ""

        if in_line_comment:
            if ch == "\n":
                in_line_comment = False
            advance(ch)
            idx += 1
            continue

        if in_block_comment:
            if ch == "*" and nxt == "/":
                in_block_comment = False
                advance(ch)
                advance(nxt)
                idx += 2
                continue
            advance(ch)
            idx += 1
            continue

        if in_single:
            if not escaped and ch == "\\":
                escaped = True
            elif escaped:
                escaped = False
            elif ch == "'":
                in_single = False
            advance(ch)
            idx += 1
            continue

        if in_double:
            if not escaped and ch == "\\":
                escaped = True
            elif escaped:
                escaped = False
            elif ch == '"':
                in_double = False
            advance(ch)
            idx += 1
            continue

        if in_template:
            if not escaped and ch == "\\":
                escaped = True
            elif escaped:
                escaped = False
            elif ch == "`":
                in_template = False
            advance(ch)
            idx += 1
            continue

        if ch == "/" and nxt == "/":
            in_line_comment = True
            advance(ch)
            advance(nxt)
            idx += 2
            continue

        if ch == "/" and nxt == "*":
            in_block_comment = True
            advance(ch)
            advance(nxt)
            idx += 2
            continue

        if ch in "'\"`":
            if ch == "'":
                in_single = True
            elif ch == '"':
                in_double = True
            else:
                in_template = True
            advance(ch)
            idx += 1
            continue

        if ch == "/":
            if prev_non_ws == "" or prev_non_ws in "([{:;,=!?&|^~<>+-*%\n":
                start_line, start_col = line, col
                idx += 1
                advance("/")
                in_class = False
                esc = False
                literal = ""
                while idx < len(text):
                    c = text[idx]
                    if not esc:
                        if c == "\\":
                            esc = True
                            literal += c
                            advance(c)
                            idx += 1
                            continue
                        if c == "[":
                            in_class = True
                            literal += c
                            advance(c)
                            idx += 1
                            continue
                        if c == "]" and in_class:
                            in_class = False
                            literal += c
                            advance(c)
                            idx += 1
                            continue
                        if c == "/" and not in_class:
                            advance(c)
                            idx += 1
                            rule_name = nearest_rule(rule_defs, start_line)
                            literals.append(
                                RegexLiteral(
                                    name=rule_name,
                                    pattern=literal,
                                    line=start_line,
                                    column=start_col,
                                )
                            )
                            break
                        literal += c
                        advance(c)
                        idx += 1
                        continue
                    else:
                        esc = False
                        literal += c
                        advance(c)
                        idx += 1
                        continue
                continue

        if not ch.isspace():
            prev_non_ws = ch
        advance(ch)
        idx += 1

    # Also capture new RegExp(...) constants for completeness
    for idx, line_text in enumerate(lines, 1):
        match = re.search(r"^\s*const\s+([a-zA-Z_][a-zA-Z0-9_]*)\s*=\s*new\s+RegExp\(`(.*)`\)", line_text)
        if match:
            name = match.group(1)
            pattern = match.group(2)
            literals.append(RegexLiteral(name=name, pattern=pattern, line=idx, column=1))

    return literals


def contains_literal(sequence: str, needle: str) -> bool:
    """Perform contains literal."""
    return needle in sequence


def has_positive_newline_match(pattern: str) -> bool:
    # Returns True if pattern can match \r or \n outside a negated class.
    """Return whether positive newline match."""
    in_class = False
    in_negated_class = False
    escaped = False
    i = 0
    while i < len(pattern):
        c = pattern[i]
        if not escaped:
            if c == "\\":
                escaped = True
                i += 1
                continue
            if c == "[":
                in_class = True
                in_negated_class = i + 1 < len(pattern) and pattern[i + 1] == "^"
                i += 1
                continue
            if c == "]" and in_class:
                in_class = False
                in_negated_class = False
                i += 1
                continue
            if c == "r" and i > 0 and pattern[i - 1] == "\\":
                if not in_class or not in_negated_class:
                    return True
            if c == "n" and i > 0 and pattern[i - 1] == "\\":
                if not in_class or not in_negated_class:
                    return True
        else:
            escaped = False
        i += 1
    return False


def has_s_outside_negated_class(pattern: str) -> bool:
    """Return whether s outside negated class."""
    in_class = False
    in_negated_class = False
    escaped = False
    i = 0
    while i < len(pattern):
        c = pattern[i]
        if not escaped:
            if c == "\\":
                escaped = True
                i += 1
                continue
            if c == "[":
                in_class = True
                in_negated_class = i + 1 < len(pattern) and pattern[i + 1] == "^"
                i += 1
                continue
            if c == "]" and in_class:
                in_class = False
                in_negated_class = False
                i += 1
                continue
            if c == "s" and i > 0 and pattern[i - 1] == "\\":
                if not in_class or not in_negated_class:
                    return True
        else:
            escaped = False
        i += 1
    return False


def audit_regexes(regexes: Iterable[RegexLiteral]) -> list[str]:
    """Perform audit regexes."""
    failures: list[str] = []
    for literal in regexes:
        name = literal.name
        pattern = literal.pattern
        if name in ALLOWED_RULES:
            continue

        if has_positive_newline_match(pattern):
            failures.append(
                f"{GRAMMAR_PATH}:{literal.line}:{literal.column}: regex for '{name}' can match newline: /{pattern}/"
            )
            continue

        if has_s_outside_negated_class(pattern):
            failures.append(
                f"{GRAMMAR_PATH}:{literal.line}:{literal.column}: regex for '{name}' uses \\s outside negated class: /{pattern}/"
            )
    return failures


def main() -> int:
    """Perform main."""
    if not GRAMMAR_PATH.exists():
        print(f"Missing grammar file: {GRAMMAR_PATH}", file=sys.stderr)
        return 2

    text = GRAMMAR_PATH.read_text()
    lines = text.splitlines()
    regexes = extract_regex_literals(text, lines)

    failures = audit_regexes(regexes)
    if failures:
        print("Continuation regex audit failed:")
        for failure in failures:
            print(f"- {failure}")
        return 1

    print("Continuation regex audit passed.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
