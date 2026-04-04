# E312: Auto-generated from corpus

## Description

Auto-generated from corpus

## Metadata
- **Status**: not_implemented
- **Last updated**: 2026-04-04 08:15 EDT

- **Error Code**: E312
- **Category**: validation
- **Level**: utterance
- **Layer**: parser

## Example 1

**Source**: `E3xx_main_tier_errors/E312_unclosed_bracket.cha`
**Trigger**: See example below
**Expected Error Codes**: E304, E375

Note: The unclosed bracket `[= comment .` causes the parser to misparse the
line structure. The parser produces E304 (missing speaker code, because the
continuation after the bracket is misinterpreted as a new line) and E375
(ContentAnnotationParseError) rather than E312 (UnclosedBracket) or E316
(UnparsableContent).

```chat
@UTF8
@Begin
@Languages:	eng
@Participants:	CHI Child
@ID:	eng|corpus|CHI|||||Child|||
*CHI:	word [= comment .
@End
```

## Expected Behavior

The parser should successfully parse these CHAT files (unless marked as parser layer), and the appropriate error should be reported.

## CHAT Rule

[Add link to relevant CHAT manual section]

## Notes

- Auto-generated from error corpus
- Review and enhance this specification as needed
