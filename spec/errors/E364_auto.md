# E364: Auto-generated from corpus

## Description

Auto-generated from corpus

## Metadata
- **Status**: not_implemented
- **Last updated**: 2026-04-04 08:15 EDT

- **Error Code**: E364
- **Category**: validation
- **Level**: utterance
- **Layer**: parser

## Example 1

**Source**: `E2xx_word_errors/E364_malformed_word_content.cha`
**Trigger**: `@s:+` — language marker with `+` instead of language code
**Expected Error Codes**: E246, E249

Note: The parser successfully parses `hello@s:+` but the `+` after `@s:` is
not a valid language code. This triggers E246 (LengtheningMarkerPosition, since
`+` is misinterpreted) and E249 (MissingLanguageContext), rather than E316
(UnparsableContent) or E364 (MalformedWordContent).

```chat
@UTF8
@Begin
@Languages:	eng
@Participants:	CHI Child
@ID:	eng|corpus|CHI|||||Child|||
*CHI:	hello@s:+ .
@End
```

## Expected Behavior

The parser should successfully parse these CHAT files (unless marked as parser layer), and the appropriate error should be reported.

## CHAT Rule

[Add link to relevant CHAT manual section]

## Notes

- Auto-generated from error corpus
- Review and enhance this specification as needed
