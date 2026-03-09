# MORTABLE -- Morphological Category Cross-Tabulation

## Purpose

Produces a per-speaker frequency table of morphosyntactic categories by matching POS tags from the `%mor` tier against patterns defined in a language-specific script file.

Requires a language script file (e.g., `eng.cut`) that defines patterns and their labels for categorizing morphemes from the `%mor` tier. Each rule line contains a quoted label and `+`/`-` prefixed POS patterns. Rules can be grouped as OR (first match wins) or AND (all must match).

See the [CLAN manual](https://talkbank.org/0info/manuals/CLAN.html#_Toc220409286) for the original MORTABLE command specification.

## Usage

```bash
chatter clan mortable --script-path eng.cut file.cha
chatter clan mortable --script-path eng.cut --speaker CHI file.cha
```

## Options

| Option | Description |
|--------|-------------|
| `--speaker <CODE>` | Include speaker |
| `--script-path <PATH>` | Language script file (.cut) -- required |
| `--format <FMT>` | Output format: text, json, csv, clan |

## Differences from CLAN

- **POS matching**: Operates on parsed `%mor` tier data rather than raw text line scanning.
- **POS matching detail**: POS tags are read directly from typed `%mor` items instead of reparsing serialized `%mor` content.
- **Script file format**: Compatible with CLAN's `.cut` files.
- **Output formats**: Supports text, JSON, and CSV formats.
- **Deterministic ordering**: `BTreeMap` ordering ensures deterministic output across runs.
- **Golden test parity**: Verified against CLAN C binary output.
