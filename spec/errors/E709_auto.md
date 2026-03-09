# E709: Invalid grammar index

## Description

Invalid grammar index

## Metadata

- **Error Code**: E709
- **Category**: validation
- **Level**: tier
- **Layer**: validation
- **Status**: not_implemented

## Example 1

**Source**: `error_corpus/validation_errors/E709_invalid_grammar_index.cha`
**Trigger**: GRA relation has non-numeric index
**Expected Error Codes**: E709

```chat
@UTF8
@Begin
@Languages:	eng
@Participants:	CHI Target_Child
@ID:	eng|corpus|CHI|||||Target_Child|||
*CHI:	hello .
%mor:	co|hello .
%gra:	abc|0|ROOT .
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
