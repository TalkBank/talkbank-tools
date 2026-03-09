# Tag: Auto-generated from corpus

## Description

Auto-generated from corpus

## Metadata

- **Error Code**: Tag
- **Category**: Alignment count mismatch
- **Level**: file
- **Layer**: parser
- **Status**: not_implemented

## Example

```chat
@Begin
@Languages:	eng
@Participants:	CHI Target_Child
@ID:	eng|corpus|CHI|2;6|male|||Target_Child|||
@Comment:	Note: Tag markers are alignable content
*CHI:	I want ± cookie .
%mor:	pro|I v|want n|cookie .
@Comment:	ERROR: Tag marker ± should have a mor item
@Comment:	Main tier alignable: I, want, ±, cookie = 4 words
@Comment:	Mor tier: Should have 4 items (missing item for ±)
@End
```

## Expected Behavior

The parser should reject this CHAT input and report a parse error at the location of the invalid syntax.

**Trigger**: Tag markers (±) should have corresponding mor items

## CHAT Rule

See the CHAT manual for format specifications: https://talkbank.org/0info/manuals/CHAT.pdf

## Notes

- Auto-generated from error corpus file: `error_corpus/E4xx_alignment_errors/tag_marker_alignment.cha`
- Review and enhance this specification as needed
