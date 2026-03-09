# Multiple: Auto-generated from corpus

## Description

Auto-generated from corpus

## Metadata

- **Error Code**: Multiple
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
@Comment:	Note: Each word in corrected form aligns separately
*CHI:	I <wanna> [: want to] <the> [: a] cookie .
%mor:	pro|I v|want n|cookie .
@Comment:	ERROR: Replacements have "want to" (2 words) and "a" (1 word)
@Comment:	Main tier alignable: I, want, to, a, cookie = 5 words
@Comment:	Mor tier: Should be pro|I v|want inf|to det|a n|cookie (5 items + terminator)
@End
```

## Expected Behavior

The parser should reject this CHAT input and report a parse error at the location of the invalid syntax.

**Trigger**: Multiple <original> \[: corrected\] forms

## CHAT Rule

See the CHAT manual for format specifications: https://talkbank.org/0info/manuals/CHAT.pdf

## Notes

- Auto-generated from error corpus file: `error_corpus/E4xx_alignment_errors/multiple_replacements.cha`
- Review and enhance this specification as needed
