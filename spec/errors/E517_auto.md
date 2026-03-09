# E517: Age should be in format years;months.days

## Description

Age should be in format years;months.days

## Metadata

- **Error Code**: E517
- **Category**: Header validation - Age format
- **Level**: header
- **Layer**: validation

## Example 1

**Source**: `E5xx_header_errors/E517_invalid_age_format.cha`
**Trigger**: Age "2.6" in @ID without semicolon separator
**Expected Error Codes**: E517

```chat
@UTF8
@Begin
@Languages:	eng
@Participants:	CHI Child
@ID:	eng|corpus|CHI|2.6||||Child|||
@Comment:	Should be: "2;6.0" or "2;6" (years;months.days format)
*CHI:	hello .
@End
```

## Expected Behavior

The parser should successfully parse these CHAT files (unless marked as parser layer), and the appropriate error should be reported.

## CHAT Rule

[Add link to relevant CHAT manual section]

## Notes

- Auto-generated from error corpus
- Review and enhance this specification as needed
