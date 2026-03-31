# Cross-Tier Alignment

**Last updated:** 2026-03-30 13:40 EDT

CHAT transcripts are layered: a main tier line carries the spoken words, while dependent tiers (`%mor`, `%gra`, `%pho`, `%sin`) carry aligned annotations. The extension surfaces these alignment relationships through hover tooltips, document highlighting, and smart selection, so you never have to count positions manually.

## Hover: Cross-Tier Alignment

Hover over any word or tier item to see how it aligns with elements on other tiers. The tooltip is computed from the parsed `ChatFile` model via `talkbank-model`'s positional alignment, not from string-position heuristics.

### Hovering on a Main Tier Word

Place your cursor over a word on a main tier line (e.g., `*CHI: I want cookie .`). A tooltip appears showing every tier item aligned to that word:

```
Main Tier Word: "cookie"

- %mor tier
  POS: n (noun)
  Stem: cookie

- %gra tier
  3|2|OBJ -- OBJ -> want (word 2)

- %pho tier
  kuki

---
Alignment computed by talkbank-model
```

> **[SCREENSHOT: Hover popup on main tier word showing %mor, %gra, and %pho alignment]**
> *Capture this: hover the cursor over the word "cookie" on a `*CHI:` main tier line that has `%mor`, `%gra`, and `%pho` dependent tiers. The tooltip should show all three aligned items.*

### Hovering on a %mor Item

Hover over a morphological analysis item to see its full breakdown, including POS subcategory, stem, and any prefixes, suffixes, clitics, or translations:

```
Morphology Element: "pro:sub|I"

POS: pro:sub (subject pronoun)
Stem: I

<- Main tier: "I" (word 1)

- %gra tier
  1|2|SUBJ -- SUBJ -> want (word 2)
```

Compound morphology (`n|+n|pine+n|apple`), fusional items, and clitic chains are all displayed when present.

### Hovering on a %gra Item

Hover over a grammatical relation to see the dependency it encodes, with back-links to both the `%mor` tier and the main tier:

```
Grammar Relation: "3|2|OBJ"

Source: cookie (word 3)
Head: want (word 2)
Relation: OBJ

<- %mor tier: n|cookie
<- Main tier: "cookie"
```

### Other Dependent Tiers

Hovering also works on `%pho` (phonological transcription), `%mod` (model phonology), and `%sin` (gesture/sign language) tier items, showing their alignment back to the main tier word and, where applicable, the corresponding `%mor` item.

### Header Hover

Hovering over `@` headers (`@Languages`, `@Participants`, `@ID`, `@Media`, etc.) shows inline documentation describing the header's purpose, syntax, and valid values. The `@ID` hover is especially detailed, displaying a field-by-field table explaining each of the 10 pipe-delimited fields (language, corpus, code, age, sex, group, SES, role, education, custom).

### Bullet Hover

Hovering over a timing bullet (`*NNN_NNN*`) shows the formatted start time, end time, and duration of the segment in human-readable form (e.g., `01:23.456 - 01:25.789 (2.333s)`). This makes it easy to inspect timing without mentally converting millisecond values.

## Document Highlighting: Visual Alignment

Click on any word or tier item and all aligned elements across tiers are highlighted simultaneously. This provides an instant visual map of how a single word flows through the annotation layers.

For example, clicking on the word "cookie" on the main tier highlights:

- **"cookie"** on the main tier (primary highlight, write access)
- **"n|cookie"** on the `%mor` tier (secondary highlight, read access)
- **"3|2|OBJ"** on the `%gra` tier (secondary highlight, read access)
- The corresponding item on `%pho`, `%mod`, or `%sin` if present

This works bidirectionally: clicking a `%gra` item highlights both its source word and head word on `%mor`, plus the corresponding main tier words.

> **[SCREENSHOT: Cross-tier highlighting with a word selected on the main tier]**
> *Capture this: click on a word like "cookie" on a main tier line that has `%mor`, `%gra`, and `%pho` tiers. All aligned items across tiers should show colored highlights.*

## Smart Selection

Use VS Code's **Expand Selection** command to expand through CHAT structural levels:

| Keystroke | macOS | Windows/Linux |
|-----------|-------|---------------|
| Expand selection | `Cmd+Shift+Right Arrow` | `Ctrl+Shift+Right Arrow` |
| Shrink selection | `Cmd+Shift+Left Arrow` | `Ctrl+Shift+Left Arrow` |

The expansion follows CHAT document structure:

1. **Word** -- a single word or tier item
2. **Tier content** -- all content on the current tier line
3. **Full line** -- the entire tier line including the tier label
4. **Utterance block** -- the main tier line plus all its dependent tiers
5. **Entire file** -- the whole document

This mirrors VS Code's standard selection expansion but is tuned for the CHAT document structure, making it easy to select exactly the scope you need for copying, deleting, or searching.

## Related Chapters

- [Dependency Graphs](dependency-graphs.md) -- visualize `%gra` tier relations as interactive arc diagrams
- [Go to Definition](go-to-definition.md) -- jump from a dependent tier item to its aligned main tier word
- [Document Symbols](symbols.md) -- navigate the file by speakers and headers
- [Syntax Highlighting](../editing/syntax-highlighting.md) -- color coding for tiers and CHAT constructs
- [Validation](../editing/validation.md) -- alignment mismatch diagnostics
