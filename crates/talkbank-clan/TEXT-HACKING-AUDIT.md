# Text-Hacking Audit (Phase C)

Classification of files using `to_chat_string()`, `split_whitespace()`, or text reparsing patterns.

## Classification Key

- **(a) Display boundary**: Serialization for final output presentation — correct by design.
- **(b) Generic fallback**: `split_whitespace()` on tiers without typed AST representation — correct for untyped tiers.
- **(c) Still-wrong**: Should use typed AST but doesn't — needs fix.

## `to_chat_string()` for Semantic Inspection (5 files)

| File | Lines | Pattern | Classification | Action |
|------|-------|---------|---------------|--------|
| `vocd.rs` | 226, 336-338 | Word token to string; echo utterance+%mor for output | (a) display | Keep |
| `combo.rs` | 242 | Store utterance text for match context output | (a) display | Keep |
| `kwal.rs` | 225 | Store utterance text for keyword match output | (a) display | Keep |
| `chip.rs` | 336, 338 | Echo utterance+%mor lines for CLAN output | (a) display | Keep |
| `uniq.rs` | 155 | Render CHAT lines for dedup counting | (a) display (semantic: UNIQ's unit IS the rendered line) | Keep |

**Conclusion**: All 5 are legitimate display boundaries. No action needed.

## `split_whitespace()` on Serialized Tier (8 files)

| File | Lines | Pattern | Classification | Action |
|------|-------|---------|---------------|--------|
| `dss.rs` | 359 | %mor token → DSS rule pattern matching via `split_whitespace()` | **(b) intentional** | DSS rules define patterns in serialized format; text matching is correct here |
| `eval.rs` | 401 | %mor morpheme counting via typed `MorTier.items` | **(FIXED)** | Converted to `extract_mor_tier()` + `classify_mor_item()` + `count_morphemes_typed()` |
| `kideval.rs` | 405 | %mor POS classification via typed `MorTier.items` | **(FIXED)** | Converted to `extract_mor_tier()` + `classify_mor_item()` + `count_morphemes_typed()` |
| `chains.rs` | (via `cod.rs`) | %cod semantic tier tokens | (b) intentional — %cod has no typed AST, `CodSemanticElement` wrapping is correct | Keep |
| `rely.rs` | 162-166 | Generic tier tokens with %cod special case | (b) fallback for unknown tiers | Keep |
| `keymap.rs` | 225-230 | Generic tier tokens with %cod special case | (b) fallback for unknown tiers | Keep |
| `trnfix.rs` | 182-184 | Token comparison across tiers; %mor/%gra have typed paths | (b) fallback for other tiers | Keep |
| `mortable.rs` | (via framework) | Script category from %mor items | (b) uses typed `mor_item_pos_tags()` | Keep |

**Conclusion**: `eval.rs` and `kideval.rs` have been **converted** to typed `MorTier.items` iteration. `dss.rs` uses text-based pattern matching which is correct for DSS rule format (rules define patterns in serialized `POS|lemma-feature` syntax). The remaining 5 files use `split_whitespace()` only for tiers without typed AST representation (correct fallback).

## Transform Text Reparsing (4 files)

| File | Lines | Pattern | Classification | Action |
|------|-------|---------|---------------|--------|
| `fixbullets.rs` | — | Operates on typed `Bullet` / `MediaTiming` AST nodes | NOT text-hacking | Keep |
| `postmortem.rs` | 87-97 | Rejects %mor rewriting with explicit error message | **(c) still-wrong** | Fix: implement AST-based %mor rewriting |
| `lines.rs` | 49-79 | Line numbering (display concern, not structural) | (a) text transform by design | Keep |
| `dataclean.rs` | 50-154 | Sub-token formatting fixes (brackets, tabs, ellipsis) | (a) text transform by design | Keep |

## Framework Semantic Layers

| File | Lines | Pattern | Classification |
|------|-------|---------|---------------|
| `cod.rs` | 53-62 | `split_whitespace()` on `%cod` → `CodSemanticElement` wrapping | Intentional semantic layer (no AST type for %cod) |
| `chat_ast.rs` | 15-36 | `for_each_leaf()` walker, typed tier accessors | Typed AST operations (correct) |
| `transform.rs` | — | Generic transform pipeline | Framework (not relevant) |

## Action Items

### Completed (Phase C2)

**`eval.rs`**: Converted from `extract_mor_text()` + `split_whitespace()` to typed `extract_mor_tier()` + `classify_mor_item()` + `count_morphemes_typed()`. All golden tests pass.

**`kideval.rs`**: Same conversion as eval.rs. `mor_texts` still collected via `extract_mor_text()` for DSS scoring compatibility. All golden tests pass.

### New framework helpers added (`mor.rs`)

- `extract_mor_tier(utterance) -> Option<&MorTier>` — typed tier extraction
- `classify_mor_item(item: &Mor, counts)` — typed POS/inflection classification
- `count_morphemes_typed(item: &Mor) -> u64` — typed morpheme counting

### Remaining (Phase D)

**`postmortem.rs` (lines 87-97)**: Currently rejects %mor rewriting with an explicit error. The fix requires AST-based %mor item rewriting: match patterns against typed `MorWord` fields and replace in place. This is Phase D work.

**`dss.rs` (line 359)**: DSS rule patterns are defined in serialized `POS|lemma-feature` format. Text-based pattern matching is correct for this use case. No action needed unless DSS rules are formalized into a typed rule language (Phase D).

---
Generated: 2026-03-06 (Phase C1)
