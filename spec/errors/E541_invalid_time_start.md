# E541: Invalid @Time Start Format

## Description

An `@Time Start` header contains a value that does not match the expected time format. The file parses successfully but the invalid value is stored as `Unsupported(String)` and flagged during validation.

## Metadata

- **Error Code**: E541
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
@Time Start:	not-a-time
*CHI:	hello world .
@End
```

## Expected Behavior

- **Parser**: Should succeed — syntax is valid
- **Validator**: Should report E541 — invalid @Time Start format

## CHAT Rule

The `@Time Start` header accepts time formats `MM:SS`, `HH:MM:SS`, or either with `.mmm` milliseconds. Free-text values are flagged as unsupported.

Reference: <https://talkbank.org/0info/manuals/CHAT.html#Time_Start_Header>

## Notes

This is a warning-level diagnostic. Invalid time start values are preserved in the model for roundtrip fidelity.
