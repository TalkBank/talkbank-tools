# E220: Auto-generated from corpus

## Description

Auto-generated from corpus

## Metadata
- **Status**: not_implemented
- **Layer**: validation

- **Error Code**: E220
- **Category**: Word validation
- **Level**: word
- **Layer**: validation

## Example 1

**Source**: `E2xx_word_errors/E220_unknown_shortening.cha`
**Trigger**: Shortening with unknown type marker
**Expected Error Codes**: E220

```chat
@UTF8
@Begin
@Languages:	eng
@Participants:	CHI Child
@ID:	eng|corpus|CHI|||||Child|||
*CHI:	hel(z)o [: hello] world .
@End
```

## Example 2

**Source**: `E2xx_word_errors/E209_replacement_missing_original.cha`
**Trigger**: Word with empty cleaned text and no untranscribed marker
**Expected Error Codes**: E220

```chat
@UTF8
@Begin
@Languages:	eng
@Participants:	CHI Child
@ID:	eng|corpus|CHI|||||Child|||
@Comment:	Note: May need programmatic test - hard to trigger from CHAT
*CHI:	0 world .
@End
```

## Example 3

**Source**: `E3xx_main_tier_errors/E340_unsupported_content_type.cha`
**Trigger**: Try to trigger internal parser bug with unsupported content
**Expected Error Codes**: E220

```chat
@UTF8
@Begin
@Languages:	eng
@Participants:	CHI Child
@ID:	eng|corpus|CHI|||||Child|||
@Comment:	Note: This may need adjustment after testing
*CHI:	hello \x00 world .
@End
```

## Expected Behavior

The parser should successfully parse these CHAT files (unless marked as parser layer), and the appropriate error should be reported.

## CHAT Rule

[Add link to relevant CHAT manual section]

## Notes

- Auto-generated from error corpus
- Review and enhance this specification as needed
