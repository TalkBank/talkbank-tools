# E357: UnmatchedUnderlineEnd

## Description

An underline end marker was found without a preceding underline begin
marker in the same utterance. The end marker has no open underline to
close.

## Metadata

- **Error Code**: E357
- **Category**: underline\_balance
- **Level**: utterance
- **Layer**: validation
- **Status**: implemented

## Example 1

**Trigger**: Underline end marker without preceding begin marker

```chat
@UTF8
@Begin
@Languages:	eng
@Participants:	CHI Target_Child
@ID:	eng|test|CHI||female|||Target_Child|||
*CHI:	I really↘ want it .
@End
```

## Expected Behavior

Validation should report E357. The underline end control character
(`\x02\x02`) appears without a preceding begin character (`\x02\x01`)
on the stack.

## Notes

- Underline markers are control characters used in CA (Conversation
  Analysis) transcription. An orphaned end marker without a matching
  begin is a data error.
