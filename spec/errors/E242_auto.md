# E242: Auto-generated from corpus

## Description

Auto-generated from corpus

## Metadata

- **Error Code**: E242
- **Category**: validation
- **Level**: word
- **Layer**: validation
- **Status**: implemented

## Example 1

**Source**: `error_corpus/validation_errors/E242_unbalanced_quotation.cha`
**Trigger**: See example below
**Expected Error Codes**: E242

```chat
@UTF8
@Begin
@Languages:	eng
@Participants:	CHI Target_Child
@ID:	eng|corpus|CHI|||||Target_Child|||
@Comment:	ERROR: Quotation marks must be balanced
@Comment:	Invalid: '"hello' - Missing closing quote
*CHI:	"hello .
@End
```

## Expected Behavior

The parser should reject this CHAT input and report a parse error at the location of the invalid syntax.

**Trigger**: See example above

## CHAT Rule

See CHAT manual sections on word-level syntax and special markers. The CHAT manual is available at: https://talkbank.org/0info/manuals/CHAT.pdf

## Notes

- Auto-generated from error corpus
- Review and enhance this specification as needed
