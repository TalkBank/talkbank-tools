# E720: Mor-Gra count mismatch

## Description

Mor-Gra count mismatch

## Metadata

- **Error Code**: E720
- **Category**: Alignment count mismatch
- **Level**: tier
- **Layer**: parser
- **Status**: not_implemented

## Example

```chat
@Begin
@Languages:	eng
@Participants:	CHI Target_Child
@ID:	eng|corpus|CHI|2;6|male|||Target_Child|||
@Comment:	Note: %gra aligns to %mor chunks, not items!
*CHI:	I want cookie .
%mor:	pro|I v|want n|cookie .
%gra:	1|2|SUBJ 2|0|ROOT 3|2|OBJ 4|4|EXTRA 5|2|PUNCT
@Comment:	ERROR: %mor has 4 chunks (3 words + terminator) but %gra has 5 relations
@End
```

## Expected Behavior

The parser should reject this CHAT input and report a parse error at the location of the invalid syntax.

**Trigger**: %mor has 3 chunks but %gra has 4 relations

## CHAT Rule

See CHAT manual sections on dependent tier formats (%mor, %gra, %pho, etc.). Each tier type has specific syntax requirements. The CHAT manual is available at: https://talkbank.org/0info/manuals/CHAT.pdf

## Notes

- Auto-generated from error corpus file: `error_corpus/E4xx_alignment_errors/E720_mor_gra_count_mismatch.cha`
- Review and enhance this specification as needed
