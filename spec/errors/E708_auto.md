# E708: GRA relation missing index

## Description

GRA relation missing index

## Metadata

- **Error Code**: E708
- **Category**: Dependent tier parsing
- **Level**: tier
- **Layer**: validation
- **Status**: implemented

## Example 1

**Source**: `E7xx_tier_parsing/E709_gra_missing_index.cha`
**Trigger**: %gra relation with empty index field
**Expected Error Codes**: E708

```chat
@UTF8
@Begin
@Languages:	eng
@Participants:	CHI Child
@ID:	eng|corpus|CHI|||||Child|||
*CHI:	hello world .
%gra:	|2|SUBJ 2|0|ROOT
@End
```

## Expected Behavior

The parser should successfully parse these CHAT files (unless marked as parser layer), and the appropriate error should be reported.

## CHAT Rule

[Add link to relevant CHAT manual section]

## Notes

- Auto-generated from error corpus
- Review and enhance this specification as needed
