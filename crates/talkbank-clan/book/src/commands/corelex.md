# CORELEX — Core Vocabulary

## Purpose

Identifies "core" vocabulary items that appear above a frequency threshold. Core vocabulary analysis is used in clinical assessment to evaluate whether a child's lexicon includes expected high-frequency words.

## Usage

```bash
chatter clan corelex file.cha
chatter clan corelex --speaker CHI file.cha
chatter clan corelex --min-frequency 5 file.cha
```

## Options

| Option | Description |
|--------|-------------|
| `--speaker <CODE>` | Include speaker |
| `--min-frequency <N>` | Minimum frequency for core classification (default: 3) |
| `--format <FMT>` | Output format: text, json, csv |

## Output

- Core word list (frequency >= threshold) sorted by frequency descending
- Non-core word list
- Core/total ratio and percentage
- Per-word speaker count (how many speakers used each word)

## Differences from CLAN

- Word identification uses AST-based `is_countable_word()`.
- Output supports text, JSON, and CSV formats.
- Core/non-core classification uses shared `NormalizedWord` for consistency.
