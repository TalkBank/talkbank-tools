# Dependent Tier Semantics Audit

Audit date: 2026-03-06

This note records which dependent tiers in `talkbank-clan` are currently forced
to have command-level semantics beyond plain preserved text, and which tiers are
still only transported, merged, reordered, or displayed.

## Why this exists

The AST/parser/serializer policy means we should not let commands quietly invent
 structure by reparsing flattened CHAT text. If a CLAN command depends on a tier
 having semantic units, we either:

- model those units in the shared TalkBank AST, or
- introduce an explicit `talkbank-clan` semantic layer derived from the parsed AST.

## Result

### Tiers that already have real structure in the shared model

- `%mor`, `%trn` alias: morphological items
- `%gra`, `%grt` alias: grammatical relations
- `%pho`, `%mod`: phonological items
- `%sin`: sign/gesture items
- `%wor`: word timing
- `%tim`: parsed time values

These tiers should be consumed through their typed ASTs directly.

### Tiers that now have an explicit clan-local semantic layer

- `%cod`

Reason:

- `CODES`, `CHAINS`, `KEYMAP`, and `RELY` all treat `%cod` as an ordered
  sequence of code-bearing items, not as generic free text.
- Real corpus data shows `%cod` items with optional selectors such as `<w4>`,
  `<w4-5>`, `<wl>`, and `<W2>`, plus opaque values like `$WR`, `$MC:NA`, `PL`,
  and `00:58`.

Bug fixed (2026-03-06):

- `%cod` command paths had been flattening parsed tier content back into raw
  whitespace tokens.
- That silently counted selector tokens like `<w4>` as codes.
- `talkbank-clan` now derives a local semantic `%cod` item stream from
  `CodTier.content`, preserving bullets/continuations/pictures and attaching an
  optional selector to the next code value.

## Remaining free-text dependent tiers

Bullet/text families:

- `%act`, `%add`, `%com`, `%exp`, `%gpx`, `%int`, `%sit`, `%spa`

Simple text families:

- `%alt`, `%coh`, `%def`, `%eng`, `%err`, `%fac`, `%flo`, `%gls`, `%ort`,
  `%par`

Current audit finding:

- No command-specific implementation in `talkbank-clan` currently requires a
  richer tier-specific semantic model for these families.
- They are mainly used for:
  - preservation/roundtrip
  - tier ordering
  - tier combination (`COMBTIER`)
  - generated output tiers like `%flo` and `%ort`
- Generic commands with `--tier` options (`CHAINS`, `KEYMAP`, `RELY`) can be
  pointed at non-`%cod` tiers, but that fallback tokenization is generic command
  behavior, not evidence that any particular tier has a settled CLAN semantic
  item model.

## Immediate policy consequence

- `%cod` should remain a `talkbank-clan` local semantic layer until we are ready
  to promote that model into the shared TalkBank grammar/data model.
- The other free-text tiers should stay as preserved text/bullet content unless
  we find a concrete CLAN command whose semantics require a minimal structured
  interpretation for that specific tier.

## Transform classification

- `LONGTIER`: pure text layout; should remain text-level
- `LINES`: pure display decoration; should remain text-level
- `INDENT`: serialized CHAT layout alignment; should remain text-level
- `DATACLEAN`: CHAT-aware text cleanup; acceptable as text-level cleanup
- `COMBTIER`: semantic tier operation; now preserves bullet/text tier variants
  rather than degrading them to user-defined tiers
- `POSTMORTEM`: semantic rewrite command; typed `%mor` rewrites are now
  explicitly unsupported instead of being flattened into text
