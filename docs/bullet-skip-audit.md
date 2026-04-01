# Bullet "Skip" Flag Usage Audit

**Status:** Current
**Last updated:** 2026-03-31 10:01 EDT

## Summary

The CHAT media bullet format `\u0015start_end\u0015` has an optional "skip" variant: `\u0015start_end-\u0015` (dash before closing NAK). The dash signals "don't play this media segment." This audit measures how widely skip bullets are used across the entire TalkBank corpus.

**Finding: 10 skip bullets in 7 files out of 99,742 (0.007%).**

## Data

Searched all `.cha` files in `/Users/chen/talkbank/data/` using:
```bash
rg '\x15[0-9]+_[0-9]+-\x15' --glob '*.cha' --count-matches /Users/chen/talkbank/data/
```

### Files with skip bullets

| File | Count | Corpus |
|------|-------|--------|
| `childes-eng-na-data/Eng-NA/MacWhinney/020718c.cha` | 2 | CHILDES |
| `childes-eng-na-data/Eng-NA/MacWhinney/020718a.cha` | 2 | CHILDES |
| `childes-eng-na-data/Eng-NA/MacWhinney/010425b.cha` | 2 | CHILDES |
| `childes-eng-na-data/Eng-NA/MacWhinney/010411a.cha` | 1 | CHILDES |
| `childes-eng-na-data/Eng-NA/MacWhinney/020805a.cha` | 1 | CHILDES |
| `ca-data/SCoSE/mary.cha` | 1 | CA |
| `aphasia-data/English/NonProtocol/Menn/GW.cha` | 1 | Aphasia |

**Total:** 10 occurrences, 7 files, 3 corpora.

All CHILDES files are from `Eng-NA/MacWhinney` — Brian's own recordings. The CA and aphasia files are single instances.

### Example

```
*CHI:	&-uh . \u0015357000_357477-\u0015
```

The dash before the closing `\u0015` marks this segment as "skip" — the media player should not play the audio for this timing range.

## Architecture Impact

The skip flag is currently represented as:
- **Grammar:** `media_url: $ => token(/\u0015\d+_\d+-?\u0015/)` — the `-?` makes the dash optional
- **Model:** `Bullet { timing: MediaTiming, skip: bool, span: Span }`
- **Token:** `Token::MediaBullet { start_time, end_time, skip: bool }`
- **WriteChat:** Serializes `skip` as `-` before closing NAK

## Recommendation

Given 10 occurrences in 99,742 files (0.007%), the skip feature is a candidate for deprecation. Options:

1. **Keep as-is** — 10 files use it; it's harmless. The `bool` field costs nothing.
2. **Deprecate** — Remove skip from the grammar, model, and parsers. The 7 affected files would need the dash removed from their bullets. Ask Brian if the skip flag has any future use.
3. **Warn** — Add a validation warning (W-level) when skip bullets are found, flagging them as deprecated.

**Ask Brian:** Does the skip flag serve any current purpose in CLAN or media playback? If not, option 2 (deprecate) simplifies the bullet type from `{ start_ms, end_ms, skip }` to `{ start_ms, end_ms }`.
