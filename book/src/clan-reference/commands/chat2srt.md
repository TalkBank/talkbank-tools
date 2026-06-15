# CHAT2SRT -- CHAT to Subtitle Conversion

**Status:** Current
**Last updated:** 2026-05-27 10:28 EDT

## Purpose

Converts CHAT files to subtitle format for captioned video. The legacy manual describes `CHAT2SRT` as converting a CHAT file to SRT for video captioning and then walks through a GUI workflow using external subtitle software.

This book focuses on the CLI semantics only: `talkbank-clan` converts timed CHAT utterances to SRT or WebVTT subtitle entries.

## Usage

```bash
chatter clan chat2srt input.cha
```

## Output Formats

| Format | Function | Timestamp style |
|--------|----------|-----------------|
| SRT    | `chat_to_srt()` | `HH:MM:SS,mmm` |
| WebVTT | `chat_to_vtt()` | `HH:MM:SS.mmm` |

## Options

| Option | Default | Description |
|--------|---------|-------------|
| *(none)* | | This converter operates on a parsed `ChatFile` and has no additional configuration options. |

## CLAN `+`-flag coverage audit

CHAT2SRT is a **converter** — input CHAT, output SRT or WebVTT.
Sources: `OSX-CLAN/src/clan/Chat2Srt.cpp::usage`,
`crates/talkbank-clan/src/converters/chat2srt.rs` (paired with
`chat2vtt.rs`).

### CHAT2SRT-specific `+`-flags (from `Chat2Srt.cpp::usage`)

| CLAN flag | Meaning | Chatter | Status | Notes |
|---|---|---|---|---|
| `+d` | Clean output without codes/replacements (default: keep all) | (default in chatter) | Partial | chatter's converter strips most CHAT annotations by default. The exact CLAN "everything" vs "clean" distinction is not user-toggleable. |
| `+v` | Create WebVTT instead of SRT | `chat2vtt` subcommand (auto-switched) | Done | CLAN: `chat2srt.cpp:108` (`case 'v'`). Different shape: chatter splits SRT vs WebVTT into two subcommands, each with its own clap surface; CLAN unifies them with `+v`. The `resolve_subcommand_alias` pre-pass in `clan_args.rs` swaps the subcommand token (`chat2srt` → `chat2vtt`) and drops `+v` before the per-arg rewriter runs, so legacy `chatter clan chat2srt +v file.cha` invocations work transparently. Subprocess regression guard: `legacy_chat2srt_v_switches_to_chat2vtt`. |

### Audit summary

| Bucket | Count |
|---|---|
| Done | 1 (default + WebVTT split) |
| Partial | 1 |
| Missing | 0 |

CHAT2SRT's CLAN flag for VTT-vs-SRT is replaced by chatter's
two-subcommand split. `+d` clean-output toggle is a UI gap;
chatter's default is "clean" with no opt-out to "include
everything."

## Input Format

Standard CHAT (`.cha`) files with timing bullets on utterances. Utterances without timing bullets are excluded from the output.

## Output

Numbered subtitle blocks with timestamps derived from CHAT timing bullets and cleaned text content (no CHAT markers, annotations, or speaker codes).

Example SRT output:

```rust,ignore
1
00:00:01,000 --> 00:00:03,500
hello world

2
00:00:04,200 --> 00:00:06,800
how are you
```

## Differences from CLAN

- **GUI material intentionally omitted here**: The legacy manual's Subtitle Writer walkthrough is out of scope for this CLI command chapter.
- Uses typed AST for subtitle text extraction
- Produces valid, well-formed SRT/WebVTT output
- Additionally supports WebVTT output format
- **Manual feature not yet mirrored**: The legacy manual documents using `+t%glo` to caption from the gloss tier. This chapter should not imply that arbitrary dependent-tier caption sourcing is available unless explicitly implemented.

## Reference

See the [CLAN manual](https://talkbank.org/0info/manuals/CLAN.html#_Toc220409296) for the original CHAT2SRT command documentation.
