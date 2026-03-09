# Omitted: 0word

## Description

0word

## Metadata

- **Error Code**: Omitted
- **Category**: Alignment count mismatch
- **Level**: file
- **Layer**: parser
- **Status**: not_implemented

## Example

```chat
@Begin
@Languages:	eng
@Participants:	CHI Target_Child
@ID:	eng|corpus|CHI|2;6|male|||Target_Child|||
@Comment:	Note: Omitted words (0word) are alignable despite being phonologically null
*CHI:	he 0is happy .
%mor:	pro|he adj|happy .
@Comment:	ERROR: Omitted word 0is should have a mor item (v|be&3S)
@Comment:	Main tier alignable: he, 0is, happy = 3 words
@Comment:	Mor tier: Should be pro|he v|be&3S adj|happy (3 items + terminator)
@End
```

## Expected Behavior

The parser should reject this CHAT input and report a parse error at the location of the invalid syntax.

**Trigger**: Omitted words like 0is should have mor items

## CHAT Rule

See the CHAT manual for format specifications: https://talkbank.org/0info/manuals/CHAT.pdf

## Notes

- Auto-generated from error corpus file: `error_corpus/E4xx_alignment_errors/omitted_word_alignment.cha`
- Review and enhance this specification as needed
