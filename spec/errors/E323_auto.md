# E323: Auto-generated from corpus

## Description

Auto-generated from corpus

## Metadata

- **Error Code**: E323
- **Category**: validation
- **Level**: utterance
- **Layer**: parser
- **Status**: not_implemented

## Example 1

**Source**: `error_corpus/parse_errors/E323_missing_colon.cha`
**Trigger**: See example below
**Expected Error Codes**: E323

```chat
@Begin
@Languages:	eng
@Participants:	CHI Child
@ID:	eng|corpus|CHI|||||Child|||
@Comment:	ERROR: Speaker must be followed by colon
@Comment:	Invalid: '*CHI hello' - Missing colon
*CHI hello .
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
