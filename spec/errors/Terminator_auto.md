# Terminator: Auto-generated from corpus

## Description

Auto-generated from corpus

## Metadata

- **Error Code**: Terminator
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
@Comment:	Note: Terminators (. ! ?) align for %mor and %gra
*CHI:	I want cookie .
%mor:	pro|I v|want n|cookie
@Comment:	ERROR: %mor missing terminator (should end with .)
@Comment:	Main tier: 3 words + terminator = 4 alignable
@Comment:	Mor tier: Only 3 items (missing terminator)
@End
```

## Expected Behavior

The parser should reject this CHAT input and report a parse error at the location of the invalid syntax.

**Trigger**: Main tier terminator should align with %mor terminator

## CHAT Rule

See the CHAT manual for format specifications: https://talkbank.org/0info/manuals/CHAT.pdf

## Notes

- Auto-generated from error corpus file: `error_corpus/E4xx_alignment_errors/terminator_alignment.cha`
- Review and enhance this specification as needed
