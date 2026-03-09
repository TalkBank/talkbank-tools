# E714: Pho count mismatch - too few pho tokens

## Description

Pho count mismatch - too few pho tokens

## Metadata

- **Error Code**: E714
- **Category**: Alignment count mismatch
- **Level**: tier
- **Layer**: validation

## Example 1

**Source**: `E4xx_alignment_errors/E714_pho_count_too_few.cha`
**Trigger**: Main tier has 3 words, but %pho has only 2 tokens
**Expected Error Codes**: E714

```chat
@UTF8
@Begin
@Languages:	eng
@Participants:	CHI Target_Child
@ID:	eng|corpus|CHI|2;6|male|||Target_Child|||
*CHI:	I want cookie .
%pho:	aɪ wɑnt
@Comment:	ERROR: Main tier has 3 words but %pho only has 2 tokens (missing cookie)
@End
```

## Expected Behavior

The parser should successfully parse these CHAT files (unless marked as parser layer), and the appropriate error should be reported.

## CHAT Rule

[Add link to relevant CHAT manual section]

## Notes

- Auto-generated from error corpus
- Review and enhance this specification as needed
