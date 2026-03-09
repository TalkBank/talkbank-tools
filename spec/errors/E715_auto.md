# E715: Pho count mismatch - too many pho tokens

## Description

Pho count mismatch - too many pho tokens

## Metadata

- **Error Code**: E715
- **Category**: Alignment count mismatch
- **Level**: tier
- **Layer**: validation

## Example 1

**Source**: `E4xx_alignment_errors/E715_pho_count_too_many.cha`
**Trigger**: Main tier has 2 words, but %pho has 3 tokens
**Expected Error Codes**: E715

```chat
@UTF8
@Begin
@Languages:	eng
@Participants:	CHI Target_Child
@ID:	eng|corpus|CHI|2;6|male|||Target_Child|||
*CHI:	want cookie .
%pho:	aɪ wɑnt kʊki
@Comment:	ERROR: Main tier has 2 words but %pho has 3 tokens (extra aɪ)
@End
```

## Expected Behavior

The parser should successfully parse these CHAT files (unless marked as parser layer), and the appropriate error should be reported.

## CHAT Rule

[Add link to relevant CHAT manual section]

## Notes

- Auto-generated from error corpus
- Review and enhance this specification as needed
