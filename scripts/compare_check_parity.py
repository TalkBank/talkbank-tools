#!/usr/bin/env python3
"""Compare CLAN CHECK output vs chatter clan check output.

Usage:
    python3 scripts/compare_check_parity.py ~/check.log ~/chatter-check.log

Parses both outputs, extracts per-file error codes, and reports matches/mismatches.
"""

import re
import sys
from collections import defaultdict
from pathlib import Path


def parse_clan_output(text: str) -> dict[str, list[int]]:
    """Parse CLAN CHECK output: extract error numbers per file."""
    errors: dict[str, list[int]] = defaultdict(list)
    current_file = None

    for line in text.splitlines():
        # *** File "filename.cha": line N.
        m = re.match(r'\*\*\* File "([^"]+)"', line)
        if m:
            current_file = Path(m.group(1)).name
            continue

        # error message text.(N)
        m = re.search(r'\((\d+)\)\s*$', line)
        if m and current_file:
            errors[current_file].append(int(m.group(1)))

    # Deduplicate and sort
    return {f: sorted(set(nums)) for f, nums in errors.items()}


def parse_chatter_output(text: str) -> dict[str, list[int]]:
    """Parse chatter clan check output: extract error numbers per file."""
    errors: dict[str, list[int]] = defaultdict(list)
    current_file = None

    for line in text.splitlines():
        # *** File "filename.cha": line N.
        m = re.match(r'\*\*\* File "([^"]+)"', line)
        if m:
            current_file = Path(m.group(1)).name
            continue

        # message(N) — CLAN-style error number
        m = re.search(r'\((\d+)\)\s*$', line)
        if m and current_file:
            errors[current_file].append(int(m.group(1)))
            continue

        # message [E316] — unmapped error code
        m = re.search(r'\[([EW]\d+)\]\s*$', line)
        if m and current_file:
            errors[current_file].append(0)  # unmapped

    return {f: sorted(set(nums)) for f, nums in errors.items()}


def main():
    if len(sys.argv) != 3:
        print(f"Usage: {sys.argv[0]} <clan-check.log> <chatter-check.log>")
        sys.exit(1)

    clan_text = Path(sys.argv[1]).read_text()
    chatter_text = Path(sys.argv[2]).read_text()

    clan_errors = parse_clan_output(clan_text)
    chatter_errors = parse_chatter_output(chatter_text)

    all_files = sorted(set(clan_errors.keys()) | set(chatter_errors.keys()))

    match = 0
    mismatch = 0
    clan_only = 0
    chatter_only = 0
    mismatches = []

    for f in all_files:
        clan = clan_errors.get(f, [])
        chatter = chatter_errors.get(f, [])

        if clan == chatter:
            match += 1
        elif f in clan_errors and f not in chatter_errors:
            clan_only += 1
            mismatches.append((f, clan, [], "CLAN only"))
        elif f not in clan_errors and f in chatter_errors:
            chatter_only += 1
            mismatches.append((f, [], chatter, "chatter only"))
        else:
            mismatch += 1
            mismatches.append((f, clan, chatter, "different"))

    print(f"=== CHECK Parity Report ===")
    print(f"Files with errors in CLAN:    {len(clan_errors)}")
    print(f"Files with errors in chatter: {len(chatter_errors)}")
    print(f"")
    print(f"Exact match:   {match}")
    print(f"Mismatch:      {mismatch}")
    print(f"CLAN only:     {clan_only}")
    print(f"Chatter only:  {chatter_only}")
    print()

    if mismatches:
        print("=== Mismatches ===")
        for f, clan, chatter, kind in mismatches:
            print(f"  {f}")
            print(f"    CLAN:    {clan}")
            print(f"    chatter: {chatter}")
            print(f"    ({kind})")
            print()

    # Summary: unique error codes
    all_clan_codes = set()
    all_chatter_codes = set()
    for nums in clan_errors.values():
        all_clan_codes.update(nums)
    for nums in chatter_errors.values():
        all_chatter_codes.update(nums)

    print(f"=== Error Code Coverage ===")
    print(f"CLAN codes seen:    {sorted(all_clan_codes)}")
    print(f"Chatter codes seen: {sorted(all_chatter_codes)}")
    print(f"In CLAN but not chatter: {sorted(all_clan_codes - all_chatter_codes)}")
    print(f"In chatter but not CLAN: {sorted(all_chatter_codes - all_clan_codes)}")


if __name__ == "__main__":
    main()
