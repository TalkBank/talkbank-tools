# KIDEVAL -- Child Language Evaluation

## Purpose

Produces a comprehensive child language evaluation report by combining multiple analysis methods into a single per-speaker summary. KIDEVAL is designed for evaluating children's language development and aggregates results from several individual CLAN commands.

See the [CLAN manual](https://talkbank.org/0info/manuals/CLAN.html#_Toc220409281) for the original KIDEVAL command specification.

## Usage

```bash
chatter clan kideval file.cha
chatter clan kideval --speaker CHI file.cha
chatter clan kideval --format json file.cha
chatter clan kideval --dss-rules-path english.scr file.cha
```

## Options

| Option | Description |
|--------|-------------|
| `--speaker <CODE>` | Include speaker |
| `--exclude-speaker <CODE>` | Exclude speaker |
| `--dss-rules-path <PATH>` | Custom DSS rules file (.scr) |
| `--ipsyn-rules-path <PATH>` | Custom IPSYN rules file |
| `--dss-max-utterances <N>` | Maximum utterances for DSS (default: 50) |
| `--ipsyn-max-utterances <N>` | Maximum utterances for IPSYN (default: 100) |
| `--format <FMT>` | Output format: text, json, csv, clan |

## Combined Metrics

KIDEVAL produces a single report combining:

| Metric | Source | Details |
|--------|--------|---------|
| MLU (words and morphemes) | Main tier + `%mor` | See [MLU](mlu.md) |
| NDW / TTR | Main tier word types/tokens | See [FREQ](freq.md) |
| DSS score | `%mor` tier | See [DSS](dss.md) |
| VOCD (D statistic) | Main tier words | See [VOCD](vocd.md) |
| IPSyn score | `%mor` tier | See [IPSYN](ipsyn.md) |
| POS category counts | `%mor` tier | Nouns, verbs, auxiliaries, etc. |
| Error counts | `[*]` markers | Word-level errors |

This is the primary tool for clinical assessment of child language samples, providing a comprehensive profile in a single command invocation.

## Differences from CLAN

### VOCD simplification

KIDEVAL uses a simplified TTR-based D estimate rather than the full bootstrap sampling approach used by the standalone [VOCD](vocd.md) command. This trades precision for speed when computing the combined report.

### IPSYN rules

Uses the built-in simplified rule subset unless a custom rules file is provided via `--ipsyn-rules-path`. For full 56-rule coverage, supply the official IPSYN rules file.

### DSS rules

Uses the built-in simplified rule subset unless a custom rules file is provided via `--dss-rules-path`. For full clinical scoring, supply a complete `.scr` rules file.

### AST-based analysis

All component analyses share the same AST-based infrastructure, ensuring consistent word identification and morpheme counting across all metrics. In CLAN, each component command has its own independent word-filtering logic, which can lead to subtle inconsistencies.

### Golden test parity

Verified against CLAN C binary output.
