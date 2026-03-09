# SUGAR -- Sampling Utterances and Grammatical Analysis Revised

## Purpose

Computes language sample analysis metrics from `%mor` and `%gra` tiers, providing a quick clinical assessment of grammatical complexity. SUGAR is designed as a time-efficient alternative to more detailed scoring systems like DSS or IPSYN.

See the [CLAN manual](https://talkbank.org/0info/manuals/CLAN.html#_Toc220409287) for the original SUGAR command specification.

## Usage

```bash
chatter clan sugar file.cha
chatter clan sugar --speaker CHI file.cha
chatter clan sugar --min-utterances 25 file.cha
chatter clan sugar --format json file.cha
```

## Options

| Option | Description |
|--------|-------------|
| `--speaker <CODE>` | Include speaker |
| `--exclude-speaker <CODE>` | Exclude speaker |
| `--min-utterances <N>` | Minimum number of utterances required (default: 50) |
| `--format <FMT>` | Output format: text, json, csv, clan |

## Metrics

| Metric | Description | Source |
|--------|-------------|--------|
| **MLU-S** | Mean Length of Utterance in morphemes | `%mor` tier |
| **TNW** | Total Number of Words (tokens with POS tags) | `%mor` tier |
| **WPS** | Words Per Sentence | Utterances containing verbs |
| **CPS** | Clauses Per Sentence | `%gra` subordination relations |

## Algorithm

1. For each utterance, count morphemes and words from `%mor`
2. Detect **verb-containing utterances** using POS tags: `v`, `cop`, `aux`, `mod`, `part`
3. For verb utterances with `%gra`, count **subordinate clauses** via grammatical relations (`COMP`, `CSUBJ`, `CMOD`, etc.)
4. Compute per-speaker ratios at finalization:
   - WPS = total words / number of verb utterances
   - CPS = total clauses / number of verb utterances

### Minimum utterance threshold

If a speaker has fewer than `min_utterances` (default: 50), the sample is flagged as insufficient. This ensures statistical reliability of the computed ratios.

## Differences from CLAN

### Verb detection

Uses mapped POS tags from the parsed `%mor` tier structure. CLAN may use a slightly different POS tag set for verb identification. Both implementations identify the same core verb categories.

Post-clitic `%mor` chunks are included in verb detection, so clitic-bearing items still contribute when the verb-like chunk appears only after `~`.

### Clause counting

Uses `%gra` subordination relations only (dependency structure). CLAN's clause detection may use additional heuristics beyond grammatical relations.

### Morpheme counting

Morpheme counts are computed from typed `%mor` structure, including post-clitics and their features.

### Minimum utterance threshold

Configurable via `--min-utterances` (default: 50). CLAN uses a fixed value.

### Output formats

Supports text, JSON, and CSV. CLAN produces text only.

### Golden test parity

Verified against CLAN C binary output.
