# E540: Invalid @Time Duration Format

## Description

An `@Time Duration` header contains a value that does not match the expected time format. The file parses successfully but the invalid value is stored as `Unsupported(String)` and flagged during validation.

## Metadata

- **Error Code**: E540
- **Category**: header_validation
- **Level**: header
- **Layer**: validation

## Example

```chat
@UTF8
@Begin
@Languages:	eng
@Participants:	CHI Target_Child
@ID:	eng|corpus|CHI|||||Target_Child|||
@Time Duration:	not-a-time
*CHI:	hello world .
@End
```

## Expected Behavior

- **Parser**: Should succeed — syntax is valid
- **Validator**: Should report E540 — invalid @Time Duration format

## CHAT Rule

The `@Time Duration` header accepts time formats like `HH:MM:SS`, `HH:MM:SS.mmm`, or time ranges like `HH:MM-HH:MM`. Free-text values are flagged as unsupported.

Reference: <https://talkbank.org/0info/manuals/CHAT.html#Time_Duration_Header>

## Notes

This is a warning-level diagnostic. Invalid time duration values are preserved in the model for roundtrip fidelity.
