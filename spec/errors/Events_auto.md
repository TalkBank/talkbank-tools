# Events: Auto-generated from corpus

## Description

Auto-generated from corpus

## Metadata

- **Error Code**: Events
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
@Comment:	Note: Events are non-alignable, like pauses
*CHI:	I &=laugh want cookie .
%mor:	pro|I v|laugh v|want n|cookie .
@Comment:	ERROR: Event &=laugh shouldn't have a mor item (v|laugh is wrong)
@Comment:	Main tier alignable: I, want, cookie = 3 words (events excluded)
@Comment:	Mor tier: Should be pro|I v|want n|cookie (3 items + terminator)
@End
```

## Expected Behavior

The parser should reject this CHAT input and report a parse error at the location of the invalid syntax.

**Trigger**: Events &=laughs don't get mor items

## CHAT Rule

See the CHAT manual for format specifications: https://talkbank.org/0info/manuals/CHAT.pdf

## Notes

- Auto-generated from error corpus file: `error_corpus/E4xx_alignment_errors/event_no_alignment.cha`
- Review and enhance this specification as needed
