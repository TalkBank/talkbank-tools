# E309: Unexpected syntax

## Description

Unexpected syntax

## Metadata

- **Error Code**: E309
- **Category**: validation
- **Level**: utterance
- **Layer**: parser
- **Status**: implemented

## Example 1

**Source**: `error_corpus/parse_errors/E309_unexpected_syntax.cha`
**Trigger**: Unexpected characters in utterance context
**Expected Error Codes**: E309

```chat
@Begin
@Languages:	eng
@Participants:	CHI Child
@ID:	eng|corpus|CHI|||||Child|||
*CHI:	hello ## world .
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
