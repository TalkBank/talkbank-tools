# E202: Missing form type after @

## Description

A word contains `@` at a position where a form type marker is expected, but
no valid form type follows. Tree-sitter produces an ERROR node at the `@`.

Valid form types: `@b`, `@c`, `@d`, `@f`, `@fp`, `@g`, `@i`, `@k`, `@l`,
`@ls`, `@n`, `@o`, `@p`, `@q`, `@sas`, `@si`, `@sl`, `@t`, `@wp`, `@x`,
`@z`, `@s` (language marker).

## Metadata
- **Status**: not_implemented

- **Error Code**: E202
- **Category**: Word validation
- **Level**: word
- **Layer**: parser

## Example 1

**Trigger**: Word ending with bare `@` — no form type follows
**Expected Error Codes**: E202

```chat
@UTF8
@Begin
@Participants:	CHI Target_Child
*CHI:	hello@ .
@End
```

## Example 2

**Trigger**: Word with `@` followed by invalid form letter
**Expected Error Codes**: E202

```chat
@UTF8
@Begin
@Participants:	CHI Target_Child
*CHI:	dog@j .
@End
```

## Expected Behavior

The parser should report E202 and recover by treating the word as malformed.
The raw text is preserved for downstream tools that may handle it differently.
