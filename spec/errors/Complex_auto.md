# Complex: Auto-generated from corpus

## Description

Auto-generated from corpus

## Metadata

- **Error Code**: Complex
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
@Comment:	Note: Replacements can contain groups, which align recursively
*CHI:	I <want the big> [: need a large] cookie .
%mor:	pro|I v|need n|cookie .
@Comment:	ERROR: Replacement "need a large" has 3 words (need, a, large)
@Comment:	Main tier alignable: I, need, a, large, cookie = 5 words
@Comment:	Mor tier: Should be pro|I v|need det|a adj|large n|cookie (5 items + terminator)
@End
```

## Expected Behavior

The parser should reject this CHAT input and report a parse error at the location of the invalid syntax.

**Trigger**: Nested \<groups \[: corrections\]\> with multiple alignable items

## CHAT Rule

See the CHAT manual for format specifications: https://talkbank.org/0info/manuals/CHAT.pdf

## Notes

- Auto-generated from error corpus file: `error_corpus/E4xx_alignment_errors/complex_nested_structure.cha`
- Review and enhance this specification as needed
