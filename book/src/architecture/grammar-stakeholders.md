# Grammar Stakeholder Analysis: Balancing Granularity for Different Consumers

**Status:** Analysis
**Date:** 2026-02-18
**Related:** [grammar-redesign.md](grammar-redesign.md)

## Context

The `tree-sitter-talkbank` grammar will be published as a standalone package across six ecosystems (npm, crates.io, PyPI, Swift Package Manager, Go modules, C/CMake). Once published, the CST node types become a **public API** consumed by people and tools we don't control. This document analyzes the different stakeholders, what each needs from the grammar's granularity, and how to balance those needs against our internal optimization goals.

## Stakeholder Inventory

### 1. TalkBank Internal Pipeline (Us)

**Who:** The `talkbank-parser`, `talkbank-lsp`, and `talkbank-cli` crates.

**How they use the grammar:**
- CST walk to build the Rust data model (`ChatFile`, `Utterance`, `Word`, `MorWord`, etc.)
- `highlights.scm` queries for semantic token generation
- `node.kind()` checks in the LSP for navigation, hover, completion
- Incremental reparsing for real-time editor feedback

**What they need:**
- Reliable tier dispatch (know which tier type you're in)
- Speaker identification
- Structural boundaries (utterance, header, dependent tier)
- Fast, conflict-free parsing

**What they DON'T need from tree-sitter:**
- Fine-grained word internals (the direct parser handles this better)
- Fine-grained bullet internals (simple regex extraction suffices)
- Whitespace sequencing rules (Rust validation handles this)

**Granularity preference:** Coarser is better. We have the direct parser as a subparser layer.

---

### 2. Editor Users WITHOUT an LSP (Neovim, Helix, Zed, Emacs)

**Who:** Anyone editing `.cha` files in an editor that supports tree-sitter but does not (or cannot) run our LSP. This is the **majority of potential users** once the grammar is published — most people will install the grammar through their editor's plugin system, not our full toolchain.

**How they use the grammar:**
- `highlights.scm` — syntax coloring. This is the **primary** and often **only** feature they get.
- `folds.scm` — code folding (collapse utterances, headers, dependent tier blocks)
- `indents.scm` — auto-indentation (continuation lines)
- `textobjects.scm` — structural text selection (select a word, select an utterance, select a tier)
- `tags.scm` — symbol extraction (list speakers, headers in a file outline)
- `locals.scm` — scope-aware highlighting (speaker references)

**What they need:**
- Named node types that queries can target for highlighting at meaningful semantic boundaries
- Enough granularity to color different constructs differently: words vs events vs pauses vs annotations vs CA markers
- Structural nodes for folding (utterances, header blocks)
- Semantic nodes for text objects (select inner word, select annotation, select tier content)

**What they DON'T need:**
- Internal structure invisible at the highlighting level (whitespace permutation variants, bullet sub-components)
- Dozens of identical tier rules that all look the same visually

**Granularity preference:** Medium. They need semantically meaningful nodes (words, events, annotations, CA markers, morphological categories), but not implementation-detail nodes (word_content, word_content_nontext, initial_word_segment vs word_segment).

**Critical insight:** These users have **no subparser fallback**. If a construct is a single opaque token, they cannot color its sub-parts. Every named node in the grammar is a potential `highlights.scm` target. Collapsing too aggressively means losing the ability to highlight CA markers, prosodic features, or morphological structure.

---

### 3. Researchers Using tree-sitter Programmatically (Python, Node, Go)

**Who:** Computational linguists, corpus linguists, NLP researchers, and students who use `tree-sitter-talkbank` from Python/Node/Go to parse CHAT files and extract structured data. This is a large potential audience — TalkBank is used by thousands of researchers worldwide.

**How they use the grammar:**
- Parse `.cha` files, walk the CST to extract data
- Tree-sitter queries to find specific constructs: "all words with language markers", "all %mor POS tags", "all timed pauses", "speaker CHI's utterances"
- Programmatic CST traversal: `node.children`, `node.child_by_field_name('speaker')`, `node.type`
- Data pipelines: extract morphological annotations, count word frequencies, compute MLU (Mean Length of Utterance)

**What they need:**
- Named nodes for every construct they'd want to query or extract:
  - Words, events, pauses, annotations (by type)
  - Speaker codes, language codes
  - %mor: POS categories, stems, suffixes, prefixes (critical for morphological research)
  - %gra: dependency relations with index/head/relation fields
  - Media timestamps (start_ms, end_ms)
  - Terminators (by type — researchers distinguish declarative vs interrogative vs interrupted)
- Field names for direct access: `node.child_by_field_name('speaker')`, `node.child_by_field_name('start_ms')`
- Stable node type names they can rely on across grammar versions

**What they DON'T need:**
- Internal parsing artifacts (whitespace nodes, structural punctuation nodes like `pipe`, `colon`, `left_bracket`)
- Multiple rules for the same semantic concept (header_sep vs tier_sep)
- Word-internal sub-character decomposition (the difference between `initial_word_segment` and `word_segment` is meaningless to a researcher)

**Granularity preference:** Selectively fine. They want **semantic** granularity (POS tag vs stem vs suffix) but not **structural** granularity (five nodes for a media bullet). The ideal for a researcher:

```python
# This should be easy:
for mor_word in tree.root_node.descendant_for_type("mor_word"):
    pos = mor_word.child_by_field_name("pos")
    stem = mor_word.child_by_field_name("stem")

# This should also be easy:
for utterance in tree.root_node.descendant_for_type("utterance"):
    speaker = utterance.child_by_field_name("speaker")
    bullet = utterance.descendant_for_type("media_url")
    start_ms = bullet.child_by_field_name("start_ms")  # <-- needs named field

# But nobody needs this:
for ws in tree.root_node.descendant_for_type("whitespaces"):  # useless
    ...
```

**Critical insight:** These users have **no Rust model layer**. The CST IS their data model. If we collapse `media_url` into a single opaque token, a Python researcher must write their own regex to extract timestamps. If we collapse %mor into a single token, they lose direct access to POS tags and stems — the most-queried data in TalkBank.

---

### 4. NLP and Automated Processing Pipelines

**Who:** Tools like CLAN (the legacy TalkBank tool), batchalign, and third-party NLP pipelines that process CHAT files at scale.

**How they use the grammar:**
- Fast batch parsing of thousands of CHAT files
- Extract specific tiers or features for downstream ML/NLP
- Often use the Python or Node bindings
- May use tree-sitter as a faster alternative to the legacy Java parser

**What they need:**
- Correct parsing (no dropped content, no misclassified tiers)
- Fast parsing (tree-sitter's C parser is already very fast)
- Reliable error recovery (partial parses when files have errors)
- Stable API (node types don't change between versions)

**Granularity preference:** They overlap with researchers (group 3). Speed favors coarser tokens (fewer nodes to allocate and traverse). Correctness favors enough structure to dispatch on tier type and extract content.

---

### 5. Editor Plugin / Tooling Developers

**Who:** People building Neovim plugins, Helix language configs, Zed extensions, or other editor integrations for CHAT files.

**How they use the grammar:**
- Write custom query files (highlights, text objects, injections)
- Build editor commands that operate on CST nodes ("select this utterance", "jump to next speaker")
- Potentially build lightweight linters or formatters using tree-sitter

**What they need:**
- Well-documented node types with clear semantics
- Supertypes for abstract categories (the grammar already has these: `terminator`, `linker`, `ca_element`, `ca_delimiter`, `base_annotation`, `dependent_tier`, `header`)
- Stable names that follow tree-sitter conventions (snake_case, descriptive)
- `node-types.json` that accurately reflects the grammar structure

**Granularity preference:** They want the **supertypes and semantic categories** to be well-defined. Individual leaf nodes matter less — they can use supertypes in queries: `(terminator) @punctuation.special` matches all terminator types without listing each one.

---

### 6. The Tree-Sitter Ecosystem / Grammar Registry

**Who:** The tree-sitter project maintainers and grammar registry (tree-sitter.github.io/tree-sitter/).

**Expectations:**
- Standard query files: at minimum `highlights.scm`, ideally also `folds.scm`, `indents.scm`, `tags.scm`, `locals.scm`
- Reasonable parser size (46K lines of parser.c is on the larger side but not extreme)
- Few or zero declared conflicts (11 is high — most published grammars have 0-3)
- Good error recovery
- Following naming conventions (snake_case node types, standard capture names)

**Granularity preference:** No strong opinion, but fewer conflicts and smaller parser size are considered quality signals.

---

## The Central Tension

```
Coarser tokens                              Finer tokens
(fewer nodes, fewer conflicts)              (more nodes, more queryable)
    |                                           |
    |  Internal pipeline (us)                   |  Researchers (Python/Node)
    |  NLP pipelines (speed)                    |  Editor users (highlighting)
    |  Parser quality (conflicts)               |  Plugin developers (queries)
    |                                           |
    v                                           v
We have the direct parser as fallback.    They DON'T have any fallback.
Coarser is purely beneficial for us.      Coarser means lost functionality.
```

**This is the key insight: we can afford coarser tokens because we have the direct parser. External consumers cannot.**

## Resolution: Tiered Granularity Strategy

The solution is not "coarser everywhere" or "finer everywhere" but **strategically choosing granularity per construct based on who queries it**:

### Tier 1: Keep Fine-Grained (Externally Queried)

These constructs are directly queried by external consumers (researchers, editors, plugins). They should remain as named, structured CST nodes.

| Construct | Why Fine | Primary Consumer |
|-----------|---------|-----------------|
| **%mor structure** (POS, stem, suffix, prefix, clitic, compound) | Most-queried data in TalkBank. Researchers compute MLU, POS distributions, morphological complexity directly from these nodes. | Researchers, highlighting |
| **%gra relations** (index, head, relation fields) | Dependency parsing data. Researchers extract syntactic structure. Fields should remain named. | Researchers |
| **Media timestamps** (start_ms, end_ms in media_url) | Researchers extract timing data for acoustic analysis, alignment studies. Should keep field names. | Researchers, NLP pipelines |
| **Terminator types** (period, question, trailing_off, etc.) | Researchers distinguish utterance types. Editors color them differently. The `terminator` supertype lets queries match all. | Researchers, highlighting |
| **Speaker codes** | Go-to-definition, completion, and the single most common research query ("find all CHI utterances"). | Everyone |
| **Annotation types** (replacement, explanation, error_marker, etc.) | Researchers query specific annotation types. Editors color them differently. | Researchers, highlighting |
| **Tier type dispatch** (each dependent tier as a named node) | Essential for both highlighting (different colors per tier type) and research queries ("find all %mor tiers"). | Everyone |
| **Pause types** (short, medium, long, timed) | Researchers study pause patterns. Timed pauses have duration data. | Researchers |
| **Linker types** | CA researchers study turn-taking patterns. Each linker type has specific meaning. | Researchers |

### Tier 2: Can Collapse (Only Internally Consumed)

These constructs are only consumed by our Rust CST walker. External consumers either don't query them or would get better results from a higher-level API.

| Construct | Current Rules | Proposed | Savings |
|-----------|--------------|----------|---------|
| **Word body internals** (word_content 12-way choice, word_content_nontext, initial_word_segment vs word_segment) | ~15 rules | Single `word_body` token; keep `standalone_word` as structural node with prefix/suffix children | ~50 states, 2 conflicts |
| **`contents` sequencing** (18-way whitespace/separator/overlap permutation) | 18 alternatives in repeat | Flat `repeat1(choice(ws, content, separator, overlap))` | ~80 states, 3-4 conflicts |
| **Inline bullet internals** (5 nodes: bullet_end, number, underscore, number, bullet_end) | 5 child nodes | Single `inline_bullet` token; Rust extracts timestamps | ~20 states |
| **Bracket annotation internals** (left_bracket, prefix, space, content, right_bracket for each type) | 5-6 nodes x 5 types | Keep named annotation types but collapse internals. Each annotation type remains a named node (for highlighting/queries) but its content is a single opaque child. | ~20 states, 1 conflict |
| **Identical text-tier boilerplate** (20 tiers with identical structure) | 20 rules + 20 prefix rules + 20 code rules | Merge structural rules, keep individual tier names via alias | ~40 states, ~40 rules |
| **Dead tier_code rules** (28 rules never referenced) | 28 rules | Delete | ~10 states, 28 rules |
| **Whitespace / structural punctuation nodes** (pipe, comma, left_bracket, etc. as named rules) | ~20 rules | Keep as-is (they're leaf tokens, cost is minimal, and they're occasionally useful for queries) | Minimal |

### Tier 3: Should ADD Granularity (Currently Missing)

These constructs are currently **under-specified** in the grammar or missing from queries entirely, which hurts external consumers.

| Construct | Current State | Proposed | Consumer |
|-----------|--------------|----------|----------|
| **CA elements** (↑, ↓, ∙, etc.) in highlighting | Not in highlights.scm | Add captures: `(ca_element) @markup.italic` or similar | Editor users, CA researchers |
| **CA delimiters** (°, ∆, ∇, etc.) in highlighting | Not in highlights.scm | Add captures: `(ca_delimiter) @markup.bold` or similar | Editor users, CA researchers |
| **Overlap markers** (⌈⌉⌊⌋) in highlighting | Not in highlights.scm | Add captures: `(overlap_point) @punctuation.bracket` | Editor users |
| **Linkers** (++, +<, etc.) in highlighting | Not in highlights.scm | Add captures: `(linker) @keyword.control` | Editor users |
| **Folding queries** | No `folds.scm` exists | Create folds.scm targeting `utterance`, header blocks, dependent tier groups | Editor users |
| **Indentation queries** | No `indents.scm` exists | Create indents.scm for continuation lines (`\n\t`) | Editor users |
| **Text object queries** | No `textobjects.scm` exists | Create textobjects.scm: word, utterance, tier, annotation, group | Plugin developers |
| **Tags/symbols queries** | No `tags.scm` exists | Create tags.scm: speakers from @Participants, headers | Plugin developers |
| **Event sub-types** | Single `event` node for all `&=action` | Could be useful to distinguish event types, but low priority | Researchers |

## Query File Roadmap

Once the grammar is published, editors expect standard query files. Here is what should exist:

### highlights.scm (Exists — Needs Expansion)

Currently 134 lines, 48 node types. Should add:
- CA element and delimiter captures
- Overlap point captures
- Linker captures
- Word sub-structure captures (if nodes are preserved): shortenings, stress markers
- Inline bullet captures (replaces VS Code regex decoration)

### folds.scm (New)

```scheme
;; Fold utterances (main tier + dependent tiers)
(utterance) @fold

;; Fold header groups
(participants_header) @fold
(id_header) @fold
```

### indents.scm (New)

```scheme
;; Continuation lines are indented with a tab
(continuation) @indent
```

### tags.scm (New)

```scheme
;; Speakers as symbols (for outline/symbol list)
(main_tier
  (speaker) @name) @definition.function

;; Participants as definitions
(participant
  code: (speaker) @name) @definition.variable

;; Headers as section markers
(header) @definition.module
```

### textobjects.scm (New — for Neovim nvim-treesitter-textobjects)

```scheme
;; @function.outer = utterance (main tier + dependent tiers)
(utterance) @function.outer

;; @function.inner = tier_body content
(tier_body) @function.inner

;; @class.outer = header
(header) @class.outer

;; @parameter.outer = word with annotations
(word_with_optional_annotations) @parameter.outer

;; @comment.outer = comment dependent tier
(com_dependent_tier) @comment.outer
```

## Stability Contract: What Is Public API?

Once published, node type names become a public API that external consumers depend on. We should define what's stable:

### Stable (Do Not Rename/Remove Without Major Version Bump)

- All supertype names: `terminator`, `linker`, `ca_element`, `ca_delimiter`, `base_annotation`, `dependent_tier`, `header`, `pre_begin_header`
- Document structure: `document`, `utterance`, `main_tier`, `tier_body`
- Speaker and participant: `speaker`, `participant`, `participants_header`
- All dependent tier names: `mor_dependent_tier`, `gra_dependent_tier`, etc.
- %mor structure: `mor_content`, `mor_word`, `mpos`, `mor_category`, `mor_subcategory`, `stem`, `mor_prefix`, `mor_suffix`, `mor_fusional_suffix`, `mor_colon_suffix`, `mor_translation`, `mor_pre_clitic`, `mor_post_clitic`, `mor_compound_word`
- %gra structure: `gra_relation`, `gra_index`, `gra_head`, `gra_relation_name`
- Content items: `word_with_optional_annotations`, `nonword_with_optional_annotations`, `event`, `pause_token`, `media_url`, `group_with_annotations`, `quotation`
- Annotations: `replacement`, `explanation_annotation`, `para_annotation`, `alt_annotation`, `error_marker_annotation`, `percent_annotation`, `duration_annotation`, `postcode`, `freecode`, `retrace_marker`, `scoped_symbol`, `overlap`
- All terminator types, all linker types
- All field names used in `field()` declarations

### Internal (May Change Without Notice)

- `word_body`, `word_content`, `word_content_nontext`, `initial_word_segment`, `word_segment`
- `contents`, `content_item`, `core_content`, `base_content_item`
- `whitespace`, `whitespaces`
- Structural punctuation aliases: `pipe`, `comma`, `left_bracket`, etc.
- `inline_bullet` internal structure (number of child nodes)
- `tier_sep`, `header_sep`, `header_gap`
- Any `_hidden` rules (prefixed with underscore)

### Documentation Required

The grammar's README should document:
1. The stable node types and what they represent
2. How to write queries against the grammar
3. Example Python/Node code for common research tasks (extract speakers, count words, get %mor data)
4. That sub-word structure may be opaque (word bodies may be single tokens)
5. The `node-types.json` file as the authoritative node type reference

## Recommendation Summary

| Principle | Rationale |
|-----------|-----------|
| **Keep %mor fine-grained** | Most-queried data in TalkBank. External consumers depend on POS/stem/suffix nodes. |
| **Keep %gra fine-grained** | Dependency relations with named fields are essential for syntactic research. |
| **Keep media_url fields** | Timestamps with named `start_ms`/`end_ms` fields serve researchers and NLP tools. |
| **Keep terminator/linker/annotation types** | Semantic distinctions that researchers and editors both query. Supertypes enable broad matching. |
| **Collapse word body internals** | No external consumer queries `initial_word_segment` vs `word_segment`. Keep `standalone_word` as the queryable node. |
| **Collapse `contents` sequencing** | Whitespace permutation rules are pure implementation detail. Flatten the repeat. |
| **Collapse inline_bullet internals** | Keep `inline_bullet` as a named node (for queries) but make it a single token. |
| **Keep annotation type nodes, collapse their internals** | Researchers query `explanation_annotation` by type. They don't query the internal `left_bracket`/`equals`/`space` structure. |
| **Merge identical text-tier structures** | Keep individual tier names (for highlighting), merge the boilerplate. |
| **Add missing query files** | `folds.scm`, `indents.scm`, `tags.scm`, `textobjects.scm` are expected by the editor ecosystem. |
| **Add CA/overlap/linker highlights** | Currently invisible to editors. CA researchers are a key TalkBank audience. |
| **Document stable vs internal nodes** | Prevent breaking external consumers when we refactor internals. |
| **Publish with comprehensive examples** | Python/Node code samples for common research tasks lower the adoption barrier. |

## The Direct Parser's Role for External Consumers

An interesting future possibility: publish the direct parser as a **companion library** (Python bindings via PyO3, or a WASM module). This would give external consumers a path to fine-grained parsing when tree-sitter's granularity isn't enough:

```python
# Tree-sitter for fast structural queries
import tree_sitter_talkbank

# Direct parser for fine-grained word analysis (hypothetical)
from talkbank_parser import parse_word, parse_mor_tier

# Use tree-sitter for structure
tree = parser.parse(source)
for utterance in find_utterances(tree):
    word_nodes = find_words(utterance)
    for word_node in word_nodes:
        # Use direct parser for fine-grained analysis
        word_model = parse_word(word_node.text)
        for content in word_model.body:
            if content.is_ca_delimiter():
                print(f"CA marker: {content.type} in '{word_node.text}'")
```

This gives external consumers the same architecture we use internally: tree-sitter for structure, direct parser for content. But this is a future initiative — for now, the grammar itself should provide enough granularity for the common research use cases without requiring a companion library.

## See Also

- [grammar-redesign.md](grammar-redesign.md) — the technical implementation plan
- `grammar/queries/highlights.scm` — current syntax highlighting queries
- `grammar/bindings/` — language binding implementations
- `grammar/tree-sitter.json` — grammar metadata and configuration
