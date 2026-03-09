# Pauses: Auto-generated from corpus

## Description

Auto-generated from corpus

## Metadata

- **Error Code**: Pauses
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
@Comment:	Note: Pauses, events, actions don't align - only words
*CHI:	I (.) want cookie .
%mor:	pro|I v|pause v|want n|cookie .
@Comment:	ERROR: Pause (.) shouldn't have a mor item (v|pause is wrong)
@Comment:	Main tier alignable: I, want, cookie = 3 words (pauses excluded)
@Comment:	Mor tier: Should be pro|I v|want n|cookie (3 items + terminator)
@End
```

## Expected Behavior

The parser should reject this CHAT input and report a parse error at the location of the invalid syntax.

**Trigger**: Pauses (.) don't get mor items

## CHAT Rule

See the CHAT manual for format specifications: https://talkbank.org/0info/manuals/CHAT.pdf

## Notes

- Auto-generated from error corpus file: `error_corpus/E4xx_alignment_errors/pause_no_alignment.cha`
- Review and enhance this specification as needed
