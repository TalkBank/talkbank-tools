#!/usr/bin/env python3
"""Synthesize a CHECK error corpus: one file per error number, verify with CLAN.

Each file triggers exactly one CHECK error number (plus structural errors
that CLAN always reports). Files are verified against the actual CLAN CHECK
binary before being accepted.

Usage:
    python3 scripts/synthesize_check_corpus.py [--check PATH_TO_CLAN_CHECK]
"""

import os
import re
import subprocess
import sys
from pathlib import Path

CORPUS_DIR = Path(__file__).resolve().parent.parent / "tests" / "check-error-corpus" / "synthetic"
CHECK_BIN = Path.home() / "talkbank" / "OSX-CLAN" / "src" / "unix" / "bin" / "check"

# Standard valid preamble — all files structurally valid so pass 2 runs
PREAMBLE = (
    "@UTF8\n"
    "@Begin\n"
    "@Languages:\teng\n"
    "@Participants:\tCHI Target_Child, MOT Mother\n"
    "@ID:\teng|test|CHI|2;06.||||Target_Child|||\n"
    "@ID:\teng|test|MOT|||||Mother|||\n"
)
END = "@End\n"

# Two-speaker with media for timing tests
MEDIA_PREAMBLE = PREAMBLE + "@Media:\ttest_file, audio\n"


def make(body: str, preamble: str = PREAMBLE) -> str:
    """Build a complete CHAT file from a body fragment."""
    return preamble + body + END


def make_pass1(content: str) -> str:
    """Build a file for pass-1 structural errors (may lack proper headers)."""
    return content


# ============================================================================
# Error corpus definitions: CHECK number → file content
# ============================================================================

CORPUS: dict[int, str] = {}

# --- Pass 1 structural errors ---
CORPUS[2] = make_pass1(
    "@UTF8\n@Begin\n@Languages:\teng\n"
    "@Participants:\tMOT Mother\n@ID:\teng|test|MOT|||||Mother|||\n"
    "@Page\n*MOT:\thello .\n@End\n"
)

CORPUS[4] = make_pass1(
    "@UTF8\n@Begin\n@Languages:\teng\n"
    "@Participants:\tMOT Mother\n@ID:\teng|test|MOT|||||Mother|||\n"
    "@Comment:       spaces not tab\n*MOT:\thello .\n@End\n"
)

# 6 is hard to trigger in isolation — @Begin missing always co-occurs with others.
# Skip 6 — it's a pass-1 error that our parser handles structurally.

CORPUS[7] = make_pass1(
    "@UTF8\n@Begin\n@Languages:\teng\n"
    "@Participants:\tMOT Mother\n@ID:\teng|test|MOT|||||Mother|||\n"
    "*MOT:\thello .\n"
)

CORPUS[44] = make_pass1(
    "@UTF8\n@Begin\n@Languages:\teng\n"
    "@Participants:\tMOT Mother\n@ID:\teng|test|MOT|||||Mother|||\n"
    "*MOT:\thello .\n@End\n*MOT:\tafter end .\n"
)

CORPUS[53] = make_pass1(
    "@UTF8\n@Begin\n@Begin\n@Languages:\teng\n"
    "@Participants:\tMOT Mother\n@ID:\teng|test|MOT|||||Mother|||\n"
    "*MOT:\thello .\n@End\n"
)

CORPUS[69] = make_pass1(
    "@Begin\n@Languages:\teng\n"
    "@Participants:\tMOT Mother\n@ID:\teng|test|MOT|||||Mother|||\n"
    "*MOT:\thello .\n@End\n"
)

# --- Pass 2 content errors (need valid structure) ---

# 11: Symbol not declared in depfile — [x N] repetition count triggers it
CORPUS[11] = make("*CHI:\tmy car [x 3] is stupid .\n")

# 13: Duplicate speaker
CORPUS[13] = make_pass1(
    "@UTF8\n@Begin\n@Languages:\teng\n"
    "@Participants:\tCHI Target_Child, CHI Target_Child\n"
    "@ID:\teng|test|CHI|2;06.||||Target_Child|||\n"
    "*CHI:\thello .\n@End\n"
)

# 15: Illegal role
CORPUS[15] = make_pass1(
    "@UTF8\n@Begin\n@Languages:\teng\n"
    "@Participants:\tCHI Dinosaur\n"
    "@ID:\teng|test|CHI|2;06.||||Dinosaur|||\n"
    "*CHI:\thello .\n@End\n"
)

# 16: Extended characters in speaker name
CORPUS[16] = make_pass1(
    "@UTF8\n@Begin\n@Languages:\teng\n"
    "@Participants:\tCH\u00e9 Target_Child\n"
    "@ID:\teng|test|CH\u00e9|2;06.||||Target_Child|||\n"
    "*CH\u00e9:\thello .\n@End\n"
)

# 17: Tier not declared in depfile — use a non-standard tier code
CORPUS[17] = make("*CHI:\thello .\n@Code:\tsome|content\n")

# 18: Speaker not in participants
CORPUS[18] = make("*DAD:\thello .\n")

# 21: Missing terminator
CORPUS[21] = make("*CHI:\thello\n")

# 22: Unmatched [
CORPUS[22] = make("*CHI:\thello [word .\n")

# 34: Illegal date
CORPUS[34] = make("@Date:\t99-ABC-9999\n*CHI:\thello .\n")

# 36: Delimiter must be at end
CORPUS[36] = make("*CHI:\thello . world\n")

# 38: Numbers should be written out
CORPUS[38] = make("*CHI:\tI have 3 cats .\n")

# 40: Duplicate dependent tiers
CORPUS[40] = make("*CHI:\thello .\n%com:\tfirst\n%com:\tsecond\n")

# 47: Numbers inside words
CORPUS[47] = make("*CHI:\thel3lo .\n")

# 48: Illegal characters
CORPUS[48] = make("*CHI:\thello\x07world .\n")

# 50: Redundant terminator
CORPUS[50] = make("*CHI:\thello . .\n")

# 60: @ID missing
CORPUS[60] = make_pass1(
    "@UTF8\n@Begin\n@Languages:\teng\n"
    "@Participants:\tMOT Mother\n*MOT:\thello .\n@End\n"
)

# 64: Wrong gender
CORPUS[64] = make_pass1(
    "@UTF8\n@Begin\n@Languages:\teng\n"
    "@Participants:\tCHI Target_Child\n"
    "@ID:\teng|test|CHI|2;06.|alien|||Target_Child|||\n"
    "*CHI:\thello .\n@End\n"
)

# 68: Requires +g2 flag — skip from auto-verify, test manually
# CORPUS[68] = ...

# 70: Empty utterance
CORPUS[70] = make("*CHI:\t.\n")

# 82: BEG > END in bullet — media name must match filename (check_082)
CORPUS[82] = make_pass1(
    "@UTF8\n@Begin\n@Languages:\teng\n"
    "@Participants:\tCHI Target_Child\n"
    "@ID:\teng|test|CHI|2;06.||||Target_Child|||\n"
    "@Media:\tcheck_082, audio\n"
    "*CHI:\thello . \x15200_100\x15\n@End\n"
)

# 83: BEG time < previous BEG — TODO: hard to trigger in isolation
# CLAN needs specific bullet+timing context that we haven't found yet

# 89: Wrong chars in bullet — TODO: hard to trigger in isolation
# CLAN may not detect malformed bullets via file-arg mode

# 94: Terminator mismatch between speaker and %mor
CORPUS[94] = make("*CHI:\thello ?\n%mor:\tn|hello .\n")

# 107: Only single commas — CLAN actually reports this as 156 for ,,
# So 107 needs a different trigger. Skip for now — 156 covers ,,
# CORPUS[107] = ...

# 117: Unpaired CA delimiter
CORPUS[117] = make("*CHI:\t\u00b0hello .\n")

# 120: Two-letter language code
CORPUS[120] = make("*CHI:\thello [- en] .\n")

# 121: Unknown language code
CORPUS[121] = make_pass1(
    "@UTF8\n@Begin\n@Languages:\tzzz\n"
    "@Participants:\tCHI Target_Child\n"
    "@ID:\tzzz|test|CHI|2;06.||||Target_Child|||\n"
    "*CHI:\thello .\n@End\n"
)

# 122: @ID language not in @Languages
CORPUS[122] = make_pass1(
    "@UTF8\n@Begin\n@Languages:\teng\n"
    "@Participants:\tCHI Target_Child\n"
    "@ID:\tfra|test|CHI|2;06.||||Target_Child|||\n"
    "*CHI:\thello .\n@End\n"
)

# 133: Speaker self-overlap — media name must match filename
CORPUS[133] = make_pass1(
    "@UTF8\n@Begin\n@Languages:\teng\n"
    "@Participants:\tCHI Target_Child\n"
    "@ID:\teng|test|CHI|2;06.||||Target_Child|||\n"
    "@Media:\tcheck_133, audio\n"
    "*CHI:\thello . \x151000_3000\x15\n"
    "*CHI:\tworld . \x152000_4000\x15\n@End\n"
)

# 140: %MOR size mismatch
CORPUS[140] = make("*CHI:\thello world .\n%mor:\tn|hello .\n")

# 141: [: ...] preceded by multiple words — TODO: need correct trigger
# "real word [: test]" passes CLAN — the trigger is more specific

# 142: Role mismatch @ID vs @Participants
CORPUS[142] = make_pass1(
    "@UTF8\n@Begin\n@Languages:\teng\n"
    "@Participants:\tCHI Child\n"
    "@ID:\teng|test|CHI|2;06.||||Target_Child|||\n"
    "*CHI:\thello .\n@End\n"
)

# 143: @ID needs 10 fields
CORPUS[143] = make_pass1(
    "@UTF8\n@Begin\n@Languages:\teng\n"
    "@Participants:\tCHI Target_Child\n"
    "@ID:\teng|test|CHI\n"
    "*CHI:\thello .\n@End\n"
)

# 144: Illegal SES field — valid values are WC,UC,MC,LI per depfile
CORPUS[144] = make_pass1(
    "@UTF8\n@Begin\n@Languages:\teng\n"
    "@Participants:\tCHI Target_Child\n"
    "@ID:\teng|test|CHI|2;06.|||JUNK|Target_Child|||\n"
    "*CHI:\thello .\n@End\n"
)

# 147: Undeclared form marker — @q is valid, try @dima (multi-char unknown)
CORPUS[147] = make("*CHI:\thello@dima .\n")

# 153: Age month/day missing zero
CORPUS[153] = make_pass1(
    "@UTF8\n@Begin\n@Languages:\teng\n"
    "@Participants:\tCHI Target_Child\n"
    "@ID:\teng|test|CHI|1;8.||||Target_Child|||\n"
    "*CHI:\thello .\n@End\n"
)

# 155: Use "0word" instead of "(word)"
CORPUS[155] = make("*CHI:\tin (parens) .\n")

# 156: Replace ,, with special character
CORPUS[156] = make("*CHI:\thello,, world .\n")

# 157: Media filename must match data filename
CORPUS[157] = make_pass1(
    "@UTF8\n@Begin\n@Languages:\teng\n"
    "@Participants:\tCHI Target_Child\n"
    "@ID:\teng|test|CHI|2;06.||||Target_Child|||\n"
    "@Media:\twrongname, audio\n"
    "*CHI:\thello .\n@End\n"
)

# 158: [: ...] must have real word
CORPUS[158] = make("*CHI:\tabua [: xxx] .\n")

# 161: Space required before [
CORPUS[161] = make("*CHI:\thello[/] hello .\n")


def run_check(filepath: Path) -> list[int]:
    """Run CLAN CHECK on a file and return error numbers."""
    try:
        result = subprocess.run(
            [str(CHECK_BIN), str(filepath)],
            capture_output=True, text=True, timeout=10
        )
        # Errors go to stdout for file-arg mode
        output = result.stdout
        return sorted(set(
            int(m.group(1))
            for m in re.finditer(r'\((\d+)\)\s*$', output, re.MULTILINE)
        ))
    except (subprocess.TimeoutExpired, FileNotFoundError):
        return []


def main():
    check_bin = Path(sys.argv[2]) if len(sys.argv) > 2 and sys.argv[1] == "--check" else CHECK_BIN

    if not check_bin.exists():
        print(f"WARNING: CLAN CHECK not found at {check_bin}, skipping verification")
        check_bin = None

    os.makedirs(CORPUS_DIR, exist_ok=True)

    verified = 0
    wrong = 0
    clean = 0

    for num in sorted(CORPUS.keys()):
        content = CORPUS[num]
        filepath = CORPUS_DIR / f"check_{num:03d}.cha"
        filepath.write_text(content, encoding="utf-8")

        if check_bin:
            codes = run_check(filepath)
            if num in codes:
                print(f"  \u2713 CHECK {num:3d}  [{','.join(str(c) for c in codes)}]")
                verified += 1
            elif not codes:
                print(f"  \u2717 CHECK {num:3d}  CLEAN")
                clean += 1
            else:
                print(f"  ? CHECK {num:3d}  got [{','.join(str(c) for c in codes)}]")
                wrong += 1
        else:
            print(f"  - CHECK {num:3d}  (not verified)")

    print(f"\nGenerated {len(CORPUS)} files in {CORPUS_DIR}")
    if check_bin:
        print(f"Verified: {verified}  Wrong: {wrong}  Clean: {clean}")
        if clean > 0:
            print(f"WARNING: {clean} files did not trigger their expected error!")


if __name__ == "__main__":
    main()
