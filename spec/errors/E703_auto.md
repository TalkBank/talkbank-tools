# E703: Unexpected morphology node

## Description

Unexpected morphology node

## Metadata

- **Error Code**: E703
- **Category**: validation
- **Level**: tier
- **Layer**: parser
- **Status**: not_implemented

## Example 1

**Source**: `error_corpus/validation_errors/E703_unexpected_mor_node.cha`
**Trigger**: Invalid morphology format
**Expected Error Codes**: E703

```chat
@UTF8
@Begin
@Languages:	eng
@Participants:	CHI Target_Child
@ID:	eng|corpus|CHI|||||Target_Child|||
*CHI:	hello .
%mor:	||||| .
@End
```

## Expected Behavior

The parser should reject this CHAT input and report a parse error at the location of the invalid syntax.

**Trigger**: See example above

## CHAT Rule

See CHAT manual sections on dependent tier formats (%mor, %gra, %pho, etc.). Each tier type has specific syntax requirements. The CHAT manual is available at: https://talkbank.org/0info/manuals/CHAT.pdf

## Notes

- Auto-generated from error corpus
- Review and enhance this specification as needed
