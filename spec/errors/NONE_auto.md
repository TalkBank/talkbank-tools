# NONE: Media filename mismatch

## Description

Media filename mismatch

## Metadata

- **Error Code**: NONE
- **Category**: validation
- **Level**: file
- **Layer**: parser
- **Status**: not_implemented

## Example

```chat
@Begin
@Languages:	eng
@Participants:	CHI Target_Child
@ID:	eng|corpus|CHI|2;6|male|||Target_Child|||
@Media:	actual-recording, audio
@Comment:	The @Media header says the media file is "actual-recording"
@Comment:	ERROR: But this file is "media-filename-mismatch.cha"
@Comment:	The media name should match: "media-filename-mismatch"
*CHI:	hello . 15_20
@End
```

## Expected Behavior

The parser should reject this CHAT input and report a parse error at the location of the invalid syntax.

**Trigger**: See example below

## CHAT Rule

See the CHAT manual for format specifications: https://talkbank.org/0info/manuals/CHAT.pdf

## Notes

- Auto-generated from error corpus file: `error_corpus/validation_gaps/media-filename-mismatch.cha`
- Review and enhance this specification as needed
