# NONE: NONE: Media filename mismatch

**Severity**: error

## Description

Media filename mismatch

## Examples

### Example 1

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

**Error**: 

## How to Fix



