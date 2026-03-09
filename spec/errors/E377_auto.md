# E377: RetraceParseError

## Description

A retrace annotation (`[/]`, `[//]`, `[///]`) could not be parsed. This
code was declared for malformed retrace content but the emission site has
not been wired up in the parser.

## Metadata

- **Error Code**: E377
- **Category**: parser\_recovery
- **Level**: utterance
- **Layer**: parser
- **Status**: not_implemented

## Notes

- Defined but not yet wired up. Intended for when retrace annotations
  have malformed content.
- No code path currently emits this error.
