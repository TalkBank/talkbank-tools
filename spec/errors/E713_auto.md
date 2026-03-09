# E713: Gra head index invalid

## Description

Gra head index invalid

## Metadata

- **Error Code**: E713
- **Category**: Alignment count mismatch
- **Level**: tier
- **Layer**: validation

## Example 1

**Source**: `E4xx_alignment_errors/E720_mor_gra_count_mismatch.cha`
**Trigger**: %mor has 3 chunks but %gra has 4 relations
**Expected Error Codes**: E713

```chat
@UTF8
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

The parser should successfully parse these CHAT files (unless marked as parser layer), and the appropriate error should be reported.

## CHAT Rule

[Add link to relevant CHAT manual section]

## Notes

- Auto-generated from error corpus
- Review and enhance this specification as needed
