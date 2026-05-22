# POSTLIST -- List POST Database Contents (deliberately not implemented)

**Status:** Reference -- stub command
**Last updated:** 2026-05-21 15:30 EDT

## Purpose

In legacy CLAN, `postlist` enumerates the contents of the POST
disambiguation rule database for a given language. It is a developer
inspection tool for the model that [`post`](post.md) consults at
runtime.

This project does **not** implement `postlist`. The POST rule
database does not exist here -- the neural morphotag pipeline
([see `mor`](mor.md)) replaces it.

## What to use instead

There is no direct replacement; Stanza models do not expose an
equivalent inspectable rule database. For developer inspection of
batchalign's morphotag output, dump the raw Stanza JSON via the
worker's debug logging, or operate on the typed `Mor` values in the
`talkbank-model` crate.

## Behavior

Invoking `chatter clan postlist` prints an error and exits non-zero.

## See also

- [MOR](mor.md), [POST](post.md), [POSTMODRULES](postmodrules.md),
  [POSTTRAIN](posttrain.md) -- companion stubs
