#!/usr/bin/env python3
"""Generate minimal CHAT files that trigger each CLAN CHECK error number.

Each file is designed to trigger exactly one CHECK error. Files are named
check_NNN.cha where NNN is the CHECK error number (zero-padded).

Usage:
    python3 scripts/generate_check_error_corpus.py
    # Then run capture_check_golden.sh to get CLAN output
"""

import os
from pathlib import Path

CORPUS_DIR = Path(__file__).resolve().parent.parent / "tests" / "check-error-corpus"

# Valid preamble used by most files
VALID = """\
\ufeff@UTF8
@Begin
@Languages:\teng
@Participants:\tCHI Target_Child
@ID:\teng|test|CHI|2;0.||||Target_Child|||
"""

# Minimal valid file for reference
VALID_FILE = VALID + "*CHI:\thello .\n@End\n"

# Two-speaker preamble
TWO_SPEAKERS = """\
\ufeff@UTF8
@Begin
@Languages:\teng
@Participants:\tCHI Target_Child, MOT Mother
@ID:\teng|test|CHI|2;0.||||Target_Child|||
@ID:\teng|test|MOT|||||Mother|||
"""


def generate():
    """Generate all CHECK error corpus files."""
    os.makedirs(CORPUS_DIR, exist_ok=True)

    files = {}

    # --- Error 1: Expected characters are: @ or % or *. ---
    # Line that doesn't start with @, %, or *
    files[1] = (
        "\ufeff@UTF8\n@Begin\n@Languages:\teng\n"
        "@Participants:\tCHI Target_Child\n"
        "@ID:\teng|test|CHI|2;0.||||Target_Child|||\n"
        "Hello this is not a tier\n"
        "*CHI:\thello .\n@End\n"
    )

    # --- Error 2: Missing ':' character and argument. ---
    # Speaker tier without colon
    files[2] = (
        "\ufeff@UTF8\n@Begin\n@Languages:\teng\n"
        "@Participants:\tCHI Target_Child\n"
        "@ID:\teng|test|CHI|2;0.||||Target_Child|||\n"
        "*CHI hello .\n@End\n"
    )

    # --- Error 3: Missing either TAB or SPACE character. ---
    files[3] = (
        "\ufeff@UTF8\n@Begin\n@Languages:\teng\n"
        "@Participants:\tCHI Target_Child\n"
        "@ID:\teng|test|CHI|2;0.||||Target_Child|||\n"
        "*CHI:hello .\n@End\n"
    )

    # --- Error 4: Found a space character instead of TAB ---
    files[4] = (
        "\ufeff@UTF8\n@Begin\n@Languages:\teng\n"
        "@Participants:\tCHI Target_Child\n"
        "@ID:\teng|test|CHI|2;0.||||Target_Child|||\n"
        "*CHI: hello .\n@End\n"
    )

    # --- Error 5: Colon (:) character is illegal. ---
    files[5] = (
        "\ufeff@UTF8\n@Begin:\n@Languages:\teng\n"
        "@Participants:\tCHI Target_Child\n"
        "@ID:\teng|test|CHI|2;0.||||Target_Child|||\n"
        "*CHI:\thello .\n@End\n"
    )

    # --- Error 6: @Begin is missing ---
    files[6] = (
        "\ufeff@UTF8\n@Languages:\teng\n"
        "@Participants:\tCHI Target_Child\n"
        "@ID:\teng|test|CHI|2;0.||||Target_Child|||\n"
        "*CHI:\thello .\n@End\n"
    )

    # --- Error 7: @End is missing ---
    files[7] = (
        "\ufeff@UTF8\n@Begin\n@Languages:\teng\n"
        "@Participants:\tCHI Target_Child\n"
        "@ID:\teng|test|CHI|2;0.||||Target_Child|||\n"
        "*CHI:\thello .\n"
    )

    # --- Error 8: Expected characters are: @ % * TAB. ---
    files[8] = (
        "\ufeff@UTF8\n@Begin\n@Languages:\teng\n"
        "@Participants:\tCHI Target_Child\n"
        "@ID:\teng|test|CHI|2;0.||||Target_Child|||\n"
        "123 bad line\n"
        "*CHI:\thello .\n@End\n"
    )

    # --- Error 12: Missing speaker name and/or role. ---
    files[12] = (
        "\ufeff@UTF8\n@Begin\n@Languages:\teng\n"
        "@Participants:\t, Mother\n"
        "@ID:\teng|test|MOT|||||Mother|||\n"
        "*MOT:\thello .\n@End\n"
    )

    # --- Error 13: Duplicate speaker declaration. ---
    files[13] = (
        "\ufeff@UTF8\n@Begin\n@Languages:\teng\n"
        "@Participants:\tCHI Target_Child, CHI Target_Child\n"
        "@ID:\teng|test|CHI|2;0.||||Target_Child|||\n"
        "*CHI:\thello .\n@End\n"
    )

    # --- Error 14: Spaces before tier code. ---
    files[14] = (
        "\ufeff@UTF8\n@Begin\n@Languages:\teng\n"
        "@Participants:\tCHI Target_Child\n"
        "@ID:\teng|test|CHI|2;0.||||Target_Child|||\n"
        " *CHI:\thello .\n@End\n"
    )

    # --- Error 15: Illegal role. ---
    files[15] = (
        "\ufeff@UTF8\n@Begin\n@Languages:\teng\n"
        "@Participants:\tCHI AlienRole\n"
        "@ID:\teng|test|CHI|2;0.||||AlienRole|||\n"
        "*CHI:\thello .\n@End\n"
    )

    # --- Error 18: Speaker not in participants list. ---
    files[18] = (
        "\ufeff@UTF8\n@Begin\n@Languages:\teng\n"
        "@Participants:\tCHI Target_Child\n"
        "@ID:\teng|test|CHI|2;0.||||Target_Child|||\n"
        "*DAD:\thello .\n@End\n"
    )

    # --- Error 21: Utterance delimiter expected. ---
    files[21] = (
        "\ufeff@UTF8\n@Begin\n@Languages:\teng\n"
        "@Participants:\tCHI Target_Child\n"
        "@ID:\teng|test|CHI|2;0.||||Target_Child|||\n"
        "*CHI:\thello\n@End\n"
    )

    # --- Error 22: Unmatched [ found on the tier. ---
    files[22] = (
        VALID + "*CHI:\t[text .\n@End\n"
    )

    # --- Error 23: Unmatched ] found on the tier. ---
    files[23] = (
        VALID + "*CHI:\ttext] .\n@End\n"
    )

    # --- Error 24: Unmatched < found on the tier. ---
    files[24] = (
        VALID + "*CHI:\t<text .\n@End\n"
    )

    # --- Error 25: Unmatched > found on the tier. ---
    files[25] = (
        VALID + "*CHI:\ttext> .\n@End\n"
    )

    # --- Error 26: Unmatched { found on the tier. ---
    files[26] = (
        VALID + "*CHI:\t{text .\n@End\n"
    )

    # --- Error 27: Unmatched } found on the tier. ---
    files[27] = (
        VALID + "*CHI:\ttext} .\n@End\n"
    )

    # --- Error 34: Illegal date representation. ---
    files[34] = (
        "\ufeff@UTF8\n@Begin\n@Languages:\teng\n"
        "@Participants:\tCHI Target_Child\n"
        "@ID:\teng|test|CHI|2;0.||||Target_Child|||\n"
        "@Date:\t99-99-9999\n"
        "*CHI:\thello .\n@End\n"
    )

    # --- Error 36: Utterance delimiter must be at end. ---
    files[36] = (
        VALID + "*CHI:\thello . world\n@End\n"
    )

    # --- Error 38: Numbers should be written out in words. ---
    files[38] = (
        VALID + "*CHI:\tI have 3 cats .\n@End\n"
    )

    # --- Error 40: Duplicate code tiers. ---
    files[40] = (
        VALID
        + "*CHI:\thello .\n"
        + "%mor:\tn|hello .\n"
        + "%mor:\tn|hello .\n"
        + "@End\n"
    )

    # --- Error 43: File must start with @Begin. ---
    # @UTF8 is OK before @Begin in modern CHAT, but other content isn't
    files[43] = (
        "\ufeff@UTF8\n@Comment:\tThis is before begin\n@Begin\n@Languages:\teng\n"
        "@Participants:\tCHI Target_Child\n"
        "@ID:\teng|test|CHI|2;0.||||Target_Child|||\n"
        "*CHI:\thello .\n@End\n"
    )

    # --- Error 44: File must end with @End. ---
    files[44] = (
        VALID + "*CHI:\thello .\n@End\n@Comment:\tAfter end\n"
    )

    # --- Error 45: More @Bg than @Eg. ---
    files[45] = (
        VALID
        + "@Bg:\tstory\n"
        + "*CHI:\thello .\n@End\n"
    )

    # --- Error 46: @Eg without matching @Bg. ---
    files[46] = (
        VALID
        + "*CHI:\thello .\n"
        + "@Eg:\tstory\n@End\n"
    )

    # --- Error 47: Numbers not allowed inside words. ---
    files[47] = (
        VALID + "*CHI:\thel3lo .\n@End\n"
    )

    # --- Error 48: Illegal character(s) found. ---
    files[48] = (
        VALID + "*CHI:\thello\x01 .\n@End\n"
    )

    # --- Error 50: Redundant utterance delimiter. ---
    files[50] = (
        VALID + "*CHI:\thello . .\n@End\n"
    )

    # --- Error 53: Only one @Begin. ---
    files[53] = (
        "\ufeff@UTF8\n@Begin\n@Begin\n@Languages:\teng\n"
        "@Participants:\tCHI Target_Child\n"
        "@ID:\teng|test|CHI|2;0.||||Target_Child|||\n"
        "*CHI:\thello .\n@End\n"
    )

    # --- Error 60: @ID tier is missing (needs +g4). ---
    files[60] = (
        "\ufeff@UTF8\n@Begin\n@Languages:\teng\n"
        "@Participants:\tCHI Target_Child\n"
        "*CHI:\thello .\n@End\n"
    )

    # --- Error 64: Wrong gender information. ---
    files[64] = (
        "\ufeff@UTF8\n@Begin\n@Languages:\teng\n"
        "@Participants:\tCHI Target_Child\n"
        "@ID:\teng|test|CHI|2;0.|alien|||Target_Child|||\n"
        "*CHI:\thello .\n@End\n"
    )

    # --- Error 68: Missing CHI Target_Child (needs +g2). ---
    files[68] = (
        "\ufeff@UTF8\n@Begin\n@Languages:\teng\n"
        "@Participants:\tMOT Mother\n"
        "@ID:\teng|test|MOT|||||Mother|||\n"
        "*MOT:\thello .\n@End\n"
    )

    # --- Error 69: UTF8 header missing. ---
    files[69] = (
        "@Begin\n@Languages:\teng\n"
        "@Participants:\tCHI Target_Child\n"
        "@ID:\teng|test|CHI|2;0.||||Target_Child|||\n"
        "*CHI:\thello .\n@End\n"
    )

    # --- Error 70: Expected either text or "0" on this tier. ---
    files[70] = (
        VALID + "*CHI:\t.\n@End\n"
    )

    # --- Error 82: BEG mark must be smaller than END mark. ---
    files[82] = (
        VALID
        + "@Media:\ttest, audio\n"
        + "*CHI:\thello . \x15200_100\x15\n@End\n"
    )

    # --- Error 83: Current BEG time smaller than previous BEG. ---
    files[83] = (
        TWO_SPEAKERS
        + "@Media:\ttest, audio\n"
        + "*CHI:\thello . \x152000_3000\x15\n"
        + "*MOT:\thi . \x151000_1500\x15\n@End\n"
    )

    # --- Error 89: Wrong characters in bullet. ---
    files[89] = (
        VALID
        + "@Media:\ttest, audio\n"
        + "*CHI:\thello . \x15abc_def\x15\n@End\n"
    )

    # --- Error 91: Blank lines not allowed. ---
    files[91] = (
        VALID + "*CHI:\thello .\n\n*CHI:\tworld .\n@End\n"
    )

    # --- Error 94: Mismatch of speaker and %mor: utterance delimiters. ---
    files[94] = (
        VALID
        + "*CHI:\thello ?\n"
        + "%mor:\tn|hello .\n@End\n"
    )

    # --- Error 107: Only single commas allowed. ---
    files[107] = (
        VALID + "*CHI:\thello,, world .\n@End\n"
    )

    # --- Error 117: Character must be used in pairs (CA delimiter). ---
    files[117] = (
        VALID + "*CHI:\t\u00b0hello .\n@End\n"
    )

    # --- Error 121: Language code not found in ISO-639. ---
    files[121] = (
        "\ufeff@UTF8\n@Begin\n@Languages:\txyz\n"
        "@Participants:\tCHI Target_Child\n"
        "@ID:\txyz|test|CHI|2;0.||||Target_Child|||\n"
        "*CHI:\thello .\n@End\n"
    )

    # --- Error 122: Language on @ID not defined on @Languages. ---
    files[122] = (
        "\ufeff@UTF8\n@Begin\n@Languages:\teng\n"
        "@Participants:\tCHI Target_Child\n"
        "@ID:\tfra|test|CHI|2;0.||||Target_Child|||\n"
        "*CHI:\thello .\n@End\n"
    )

    # --- Error 128: Unmatched \u2039 (single left-pointing angle quotation mark). ---
    files[128] = (
        VALID + "*CHI:\t\u2039hello .\n@End\n"
    )

    # --- Error 129: Unmatched \u203a. ---
    files[129] = (
        VALID + "*CHI:\thello\u203a .\n@End\n"
    )

    # --- Error 133: Speaker self-overlap (BEG < same speaker prev END). ---
    files[133] = (
        VALID
        + "@Media:\ttest, audio\n"
        + "*CHI:\thello . \x151000_3000\x15\n"
        + "*CHI:\tworld . \x152000_4000\x15\n@End\n"
    )

    # --- Error 136: Unmatched \u201c (left double quotation mark). ---
    files[136] = (
        VALID + "*CHI:\t\u201chello .\n@End\n"
    )

    # --- Error 137: Unmatched \u201d (right double quotation mark). ---
    files[137] = (
        VALID + "*CHI:\thello\u201d .\n@End\n"
    )

    # --- Error 140: %MOR does not link in size to speaker tier. ---
    files[140] = (
        VALID
        + "*CHI:\thello world .\n"
        + "%mor:\tn|hello .\n@End\n"
    )

    # --- Error 142: Role mismatch @ID vs @Participants. ---
    files[142] = (
        "\ufeff@UTF8\n@Begin\n@Languages:\teng\n"
        "@Participants:\tCHI Child\n"
        "@ID:\teng|test|CHI|2;0.||||Target_Child|||\n"
        "*CHI:\thello .\n@End\n"
    )

    # --- Error 143: @ID needs 10 fields. ---
    files[143] = (
        "\ufeff@UTF8\n@Begin\n@Languages:\teng\n"
        "@Participants:\tCHI Target_Child\n"
        "@ID:\teng|test|CHI\n"
        "*CHI:\thello .\n@End\n"
    )

    # --- Error 153: Age month or day missing initial zero. ---
    files[153] = (
        "\ufeff@UTF8\n@Begin\n@Languages:\teng\n"
        "@Participants:\tCHI Target_Child\n"
        "@ID:\teng|test|CHI|2;5.3||||Target_Child|||\n"
        "*CHI:\thello .\n@End\n"
    )

    # --- Error 157: Media filename must match data filename. ---
    files[157] = (
        "\ufeff@UTF8\n@Begin\n@Languages:\teng\n"
        "@Participants:\tCHI Target_Child\n"
        "@ID:\teng|test|CHI|2;0.||||Target_Child|||\n"
        "@Media:\twrongname, audio\n"
        "*CHI:\thello .\n@End\n"
    )

    # --- Error 160: Space not allowed after '<' or before '>'. ---
    files[160] = (
        VALID + "*CHI:\t< hello > [/] hello .\n@End\n"
    )

    # --- Error 161: Space required before '[' code item. ---
    files[161] = (
        VALID + "*CHI:\thello[/] hello .\n@End\n"
    )

    # Write all files
    for num, content in sorted(files.items()):
        filename = f"check_{num:03d}.cha"
        filepath = CORPUS_DIR / filename
        with open(filepath, "w", encoding="utf-8") as f:
            f.write(content)

    print(f"Generated {len(files)} CHECK error corpus files in {CORPUS_DIR}")
    print(f"Error numbers covered: {sorted(files.keys())}")

    # Report which of the 87 documented errors are missing
    documented = {
        1, 2, 3, 4, 5, 6, 7, 8, 12, 13, 14, 15, 18, 21, 22, 23, 24, 25,
        26, 27, 34, 36, 38, 40, 43, 44, 45, 46, 47, 48, 50, 51, 52, 53,
        55, 56, 57, 58, 60, 64, 68, 69, 70, 81, 82, 83, 84, 85, 89, 90,
        91, 92, 93, 94, 102, 107, 110, 117, 118, 120, 121, 122, 128, 129,
        130, 131, 136, 137, 140, 141, 142, 143, 151, 153, 155, 156, 159,
        160, 161,
    }
    covered = set(files.keys())
    missing = documented - covered
    print(f"\nDocumented errors not yet in corpus ({len(missing)}):")
    for n in sorted(missing):
        print(f"  {n}")


if __name__ == "__main__":
    generate()
