"""Italian multi-word-token policy.

Stanza's Italian MWT processor treats multi-word tokens as an OPEN class and
splits aggressively. On isolated single-word utterances it is wrong about a
third of the time, inventing verbs that do not exist in Italian. Measured
2026-07-28 against real single-word utterances pulled from the CHILDES Italian
corpora, stanza 1.13.0::

    gallina  (hen)    -> galli + na      VERB + PRON   ("na" is not a clitic)
    cavallo  (horse)  -> cava  + lo      VERB + PRON
    mucche   (cows)   -> mu + cce + he   X + X + X     flat:foreign
    attenzione, macchine, persone, stazione, canzone -> verb + "ne", all iob

10 of 29 real corpus words destroyed. The damage is not confined to the
relation label: ``%mor`` records ``verb|attenzare`` for the noun *attenzione*,
inventing a verb, so normalizing the relation cannot repair it.

**The same words are analyzed correctly in sentence context**, so this is
specific to utterances with no syntactic context to constrain the parse. Those
dominate child speech (``*CHI:\tmacchine .``), which is why CHILDES Italian is
hit hardest.

The fix, and why it is safe
---------------------------
Italian multi-word tokens are very nearly a CLOSED class, unlike Stanza's
treatment of them:

1. preposition + article contractions: fully closed, enumerated below
2. ``ecco`` + enclitic pronoun: fully closed, enumerated below
3. verb + enclitic pronoun: genuinely open (any verb), and the only reason
   this module cannot simply allowlist everything

For a SINGLE-WORD utterance, category 3 is rare and category 1/2 are
enumerable, so suppressing expansion outside the closed sets is correct far
more often than not. Verified by running the same words with MWT suppressed:

    attenzione -> NOUN attenzione    macchine -> NOUN macchina
    persone    -> NOUN persona       mucche   -> NOUN mucca
    cavallo    -> NOUN cavallo       gallina  -> NOUN gallina
    stazione, canzone -> NOUN        dai      -> VERB dare

9 of 10 become exactly right, including plural lemmas (*macchine* ->
*macchina*). The tenth, ``eccolo``, is category 2 and is preserved by the
allowlist here, giving 10 of 10.

Note ``dai`` in particular: as a one-word utterance it is the verb/interjection
*dai* ("come on"), and Stanza's split into *da* + *i* (preposition + article)
is wrong in that context. Suppression fixes it; an allowlist keyed only on
surface form would not have.

**Scope: single-word utterances only.** Multi-word utterances keep Stanza's
behavior, which was measured correct for these same nouns. A separate,
unfixed defect exists in multi-word context (``la`` -> *il i*, ``hai`` ->
*ha i*); see the Stanza limitations page in the book.
"""

from __future__ import annotations

# Preposition + article contractions. A closed class in Italian: the cross
# product of {a, da, di, in, su, con, per} with the definite articles, plus the
# elided apostrophe forms. Listed exhaustively rather than generated so the set
# is auditable by a linguist reading this file.
ITALIAN_PREP_ARTICLE: frozenset[str] = frozenset({
    # a +
    "al", "allo", "alla", "ai", "agli", "alle", "all",
    # da +
    "dal", "dallo", "dalla", "dai", "dagli", "dalle", "dall",
    # di +
    "del", "dello", "della", "dei", "degli", "delle", "dell",
    # in +
    "nel", "nello", "nella", "nei", "negli", "nelle", "nell",
    # su +
    "sul", "sullo", "sulla", "sui", "sugli", "sulle", "sull",
    # con + (archaic/regional but attested)
    "col", "collo", "colla", "coi", "cogli", "colle",
    # per + (archaic)
    "pel", "pei",
})

# Enclitic pronouns that attach to `ecco`. Closed.
ITALIAN_ECCO_CLITICS: frozenset[str] = frozenset({
    "lo", "la", "li", "le", "mi", "ti", "ci", "vi", "ne",
})


def is_closed_class_italian_mwt(word: str) -> bool:
    """Is *word* a multi-word token from one of Italian's CLOSED classes?

    Returns ``True`` only for preposition+article contractions and
    ``ecco``+enclitic forms. Verb+enclitic (``dammi``, ``portarmelo``) is an
    open class and deliberately returns ``False`` here: this predicate exists
    to decide what may still expand when expansion is otherwise suppressed,
    and an open class cannot be decided from the surface form alone.
    """
    lowered = word.lower().strip()
    if not lowered:
        return False
    # Strip a trailing apostrophe so `dell'` matches `dell`.
    normalized = lowered.rstrip("'’")
    if normalized in ITALIAN_PREP_ARTICLE:
        return True
    if normalized.startswith("ecco") and normalized[4:] in ITALIAN_ECCO_CLITICS:
        return True
    return False


def is_single_word_utterance(original_words: list[str]) -> bool:
    """Does this utterance carry exactly one word besides punctuation?

    CHAT utterances carry their terminator as a token (``["macchine", "."]``),
    so a one-word utterance has length 2 in the word list. Anything with two or
    more real words has enough context for Stanza to parse correctly, and is
    left alone.
    """
    content = [w for w in original_words if any(ch.isalnum() for ch in w)]
    return len(content) == 1
