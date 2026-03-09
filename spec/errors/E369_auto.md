# E369: Nonvocal label mismatch

## Description

Nonvocal label mismatch

## Metadata

- **Error Code**: E369
- **Category**: validation
- **Level**: utterance
- **Layer**: parser
- **Status**: not_implemented

## Example 1

**Source**: `error_corpus/parse_errors/E369_nonvocal_label_mismatch.cha`
**Trigger**: Nonvocal begin/end labels don't match
**Expected Error Codes**: E369

```chat
@UTF8
@Begin
@Languages:	eng
@Participants:	CHI Target_Child
@ID:	eng|corpus|CHI|||||Target_Child|||
*CHI:	hello &{laughing} world &{crying} .
@End
```

## Expected Behavior

The parser should reject this CHAT input and report a parse error at the location of the invalid syntax.

**Trigger**: See example above

## CHAT Rule

See CHAT manual sections on main tier syntax and utterance structure. Every utterance must end with a terminator (., ?, or \!). The CHAT manual is available at: https://talkbank.org/0info/manuals/CHAT.pdf

## Notes

- Auto-generated from error corpus
- Review and enhance this specification as needed
