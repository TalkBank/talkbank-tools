# Alignment: Auto-generated from corpus

## Description

Auto-generated from corpus

## Metadata

- **Error Code**: Alignment
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
@Comment:	Note: Compounds like "ice+cream" may have multiple %mor items (n|ice+n|cream)
*CHI:	I want ice+cream .
%mor:	pro|I v|want n|ice .
@Comment:	ERROR: Compound "ice+cream" should have 2 mor items (n|ice+n|cream) not 1
@End
```

## Expected Behavior

The parser should reject this CHAT input and report a parse error at the location of the invalid syntax.

**Trigger**: Compound word in main tier (ice+cream) should have 2 mor items, but only has 1

## CHAT Rule

See the CHAT manual for format specifications: https://talkbank.org/0info/manuals/CHAT.pdf

## Notes

- Auto-generated from error corpus file: `error_corpus/E4xx_alignment_errors/compound_alignment_mismatch.cha`
- Review and enhance this specification as needed
