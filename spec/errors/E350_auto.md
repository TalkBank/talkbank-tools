# E350: Replacement text empty

## Description

Replacement text empty

## Metadata

- **Error Code**: E350
- **Category**: Word validation
- **Level**: utterance
- **Layer**: parser
- **Status**: not_implemented

## Example

```chat
@Begin
@Languages:	eng
@Participants:	CHI Child
@ID:	eng|corpus|CHI|||||Child|||
*CHI:	hello [:] world .
@End
```

## Expected Behavior

The parser should reject this CHAT input and report a parse error at the location of the invalid syntax.

**Trigger**: Replacement with no words in corrected form

## CHAT Rule

See CHAT manual sections on main tier syntax and utterance structure. Every utterance must end with a terminator (., ?, or \!). The CHAT manual is available at: https://talkbank.org/0info/manuals/CHAT.pdf

## Notes

- Auto-generated from error corpus file: `error_corpus/E2xx_word_errors/E208_invalid_form_type.cha`
- Review and enhance this specification as needed
