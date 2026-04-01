# E360: Deprecated Skip Bullet

## Description

The media bullet contains a deprecated skip flag (dash before closing NAK delimiter). The skip flag was deprecated as of 2026-03-31 (confirmed by Brian MacWhinney). Only 10 occurrences exist in 7 files across the entire 99,742-file corpus.

## Metadata

- **Status**: not_implemented
- **Error Code**: E360
- **Category**: Main tier validation
- **Level**: utterance
- **Layer**: validation

## Example 1

**Source**: `content/deprecated-skip-bullet.cha`
**Trigger**: Media bullet with dash before closing NAK
**Expected Error Codes**: E360

```chat
@UTF8
@Begin
@Languages:	eng
@Participants:	CHI Child
@ID:	eng|corpus|CHI|||||Child|||
*CHI:	hello . 357000_357477-
@End
```

## Expected Behavior

The parser should successfully parse the file. The dash is silently stripped from the bullet timestamp. The validator should report E360 warning that the skip flag is deprecated.

## CHAT Rule

Media bullets use NAK delimiters: `\u0015start_end\u0015`. The legacy skip variant `\u0015start_end-\u0015` (dash before closing NAK) was used to mark segments that should be skipped during continuous playback. This feature is no longer supported.

## Notes

The 7 affected files in the corpus are all in Brian's CHILDES recordings (Eng-NA/MacWhinney) plus one CA file (SCoSE/mary.cha) and one aphasia file (Menn/GW.cha). These files should have the dash removed from their bullet markers.
