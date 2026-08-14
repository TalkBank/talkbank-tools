"""Italian multi-word-token policy.

The defect
----------
Stanza's Italian MWT processor treats multi-word tokens as an OPEN class and
over-splits. Measured 2026-07-28 against the CHILDES Italian corpora, stanza
1.13.0.

Context-free, on the single-word utterances that dominate child speech::

    cavallo (horse)  -> cava  + lo     bello  (nice)   -> ib   + lo
    gallina (hen)    -> galli + na     giallo (yellow) -> gia  + lo
    attenzione       -> attenzi + ne   uccelli (birds) -> u + ce + li

and in full sentences, which is the same defect and not a milder one::

    la stazione e molto grande .          -> la  = il + i
    secondo la mia opinione hai ragione . -> hai = ha + i
    questa e la mozzarella .              -> mozzarella = mozzar + la

The damage is not confined to the relation label: ``%mor`` records
``verb|attenzare`` for the noun *attenzione*, a verb that does not exist.
(Corpus-scale counts are deliberately absent here: an earlier quantification
was retracted for lacking per-word language resolution; the mechanisms above
are pinned by the ml_golden tests, which run the real pipeline.)

Why this is hard, and why an allowlist is wrong
-----------------------------------------------
Genuine single-word multi-word tokens are common and must keep splitting:
``dammelo`` (give it to me), ``diglielo`` (tell it to him), ``giralo`` (turn
it). Those are productive verb+enclitic imperatives, an OPEN class, so no
surface pattern separates them from ``cavallo``: both are a verb-shaped base
plus a real clitic. A closed-class allowlist was briefly committed and
destroyed every such imperative; it also made Stanza invent a lemma for them
(``diglielo`` came back as ``verb|diglielare``), so it did not even fail safe.

A part-of-speech probe is not enough either
-------------------------------------------
The obvious next idea is to analyze the UNSPLIT form with MWT disabled and
allow the split only when it tags VERB. Measured over the affected inventory
that rule is wrong both ways: it suppresses ``giralo`` (NOUN), ``aprila``
(NOUN) and ``eccolo`` (ADJ), and it allows ``dondolo``, ``viola``, ``scivola``,
``disegna`` and ``cancello``, which are ordinary words. On a 39-case
discrimination set it scored 29/39.

The rule this module implements
-------------------------------
Validate the split Stanza proposes against facts about Italian. None of those
facts mention context, so the same rule answers wherever the word appears.

Italian has exactly FOUR legitimate multi-word patterns, three of them closed.
A split is genuine only if it instantiates one:

1. **Preposition + definite article** (``alla`` = *a* + *la*). Requires BOTH
   that the surface is in the contracted paradigm and that the analysis is
   preposition + article. The analysis test alone admits English ``well`` ->
   *In* + *l*; the surface test alone cannot reject ``la`` -> *il* + *i*.
2. **Clitic cluster** (``glielo`` = *glie* + *lo*). No host at all, just stacked
   clitics, so the verb tests below cannot see these.
3. **``ecco`` + enclitic** (``eccolo``). The presentative is the only non-verb
   host of enclisis in the language.
4. **Verb + enclitic** (``giralo`` = *gira* + *lo*). The one open class, and the
   only one needing real validation:

   a. the split accounts for every character of the word, allowing the one
      regular departure (gemination after an apocopated monosyllabic
      imperative). Rejects ``pentolone`` -> *pento* + *lo*, which drops ``ne``.
   b. every non-initial piece is an Italian enclitic pronoun, a closed class.
      Rejects ``gallina`` -> *galli* + *na* and ``disegna`` -> *di*+*se*+*gna*.
   c. the base is an attested verb, from Stanza's own shipped lexicon rather
      than a list we maintain. Rejects ``bello`` -> *ib* + *lo*.
   d. the whole form is not itself a dictionary word. Rejects ``cavallo`` ->
      *cava* + *lo*, where the base *cava* genuinely IS a verb and (a) to (c)
      all pass.

Every one of (a) to (d) is load-bearing; each catches cases the others miss.
Scored 38/39 on a discrimination set, the single miss being ``svegliati``, a
real imperative that Stanza's lexicon lists as a word in its own right.

Where the rule must guess it prefers to under-split: losing a split leaves a
real word coarsely analyzed, while a false split invents a verb that does not
exist in Italian, which is the defect this module exists to remove.
"""

from __future__ import annotations

from collections.abc import Callable, Mapping, Sequence
from dataclasses import dataclass
from enum import Enum

# Italian enclitic pronouns: the complete closed class. Direct and indirect
# object, reflexive, partitive and locative, plus the `e`-form variants a clitic
# takes when another clitic follows it (me, te, se, ce, ve, glie). Listed
# exhaustively rather than generated so a linguist can audit it by reading.
ITALIAN_ENCLITICS: frozenset[str] = frozenset(
    {
        "mi",
        "ti",
        "si",
        "ci",
        "vi",
        "lo",
        "la",
        "li",
        "le",
        "ne",
        "gli",
        "me",
        "te",
        "se",
        "ce",
        "ve",
        "glie",
    }
)

# The `e`-form variants exist ONLY before another clitic: `mi` becomes `me` in
# ``dammelo`` (*me* + *lo*), `ci` becomes `ce` in ``metticelo``. Italian has no
# word ending in a bare enclitic `me`/`ce`/`ve`, so a split whose LAST piece is
# one of these is not a possible Italian word at all.
#
# This is the constraint that keeps force-splitting from fabricating structure
# on surfaces that merely end in clitic-shaped letters (verified against raw
# Stanza: without it, English ``face`` would force-split to *fa* + *ce*).
# `ne` is deliberately NOT here: it is a real final clitic (``dammene``,
# ``scegline``).
CLITICS_REQUIRING_A_FOLLOWER: frozenset[str] = frozenset(
    {
        "me",
        "te",
        "se",
        "ce",
        "ve",
        "glie",
    }
)

# The only prepositions that fuse with a following definite article, and the
# complete set of definite articles they fuse with. Both are closed function-word
# classes of Italian and have been for centuries, so enumerating them is a
# statement of the language, not a table that can drift the way the retired
# per-language override tables did.
#
# Validating the ANALYSIS (is this ADP + DET?) rather than the SURFACE (is this
# one of about forty contracted spellings?) is what lets this reject Stanza's
# `la` -> *il* + *i* and `hai` -> *ha* + *i*: both look like contractions but
# neither has a preposition in the base.
ITALIAN_ARTICLE_FUSING_PREPOSITIONS: frozenset[str] = frozenset(
    {
        "a",
        "da",
        "di",
        "in",
        "su",
        "con",
        "per",
    }
)
ITALIAN_DEFINITE_ARTICLES: frozenset[str] = frozenset(
    {
        "il",
        "lo",
        "la",
        "i",
        "gli",
        "le",
        "l'",
        "l",
    }
)

# The complete contracted paradigm: seven prepositions crossed with the definite
# articles. Finite, closed, and unchanged for centuries, so writing it out states
# the language rather than starting a table that grows one production incident at
# a time (which is exactly what the retired override tables did).
#
# The structural ADP+DET test alone is NOT enough: it accepts ANY pair the
# tagger labels preposition+article, including mangles of surfaces outside the
# paradigm (verified against raw Stanza: English `well` -> *In* + *l* passes
# the structural test alone). The paradigm is closed and centuries-stable, so
# requiring the surface to belong to it costs nothing and closes that hole.
#
# Bare elided forms (`all`, `dell`) are deliberately absent: written Italian
# keeps the elision apostrophe (`all'`, `dell'`), so a bare `all` is not a
# spelling of the contraction and must not be treated as one.
ITALIAN_PREPOSITION_ARTICLE_CONTRACTIONS: frozenset[str] = frozenset(
    {
        "al",
        "allo",
        "alla",
        "ai",
        "agli",
        "alle",
        "all'",
        "dal",
        "dallo",
        "dalla",
        "dai",
        "dagli",
        "dalle",
        "dall'",
        "del",
        "dello",
        "della",
        "dei",
        "degli",
        "delle",
        "dell'",
        "nel",
        "nello",
        "nella",
        "nei",
        "negli",
        "nelle",
        "nell'",
        "sul",
        "sullo",
        "sulla",
        "sui",
        "sugli",
        "sulle",
        "sull'",
        # con + and per + are archaic but attested in older transcripts.
        "col",
        "collo",
        "colla",
        "coi",
        "cogli",
        "colle",
        "pel",
        "pei",
    }
)

# The presentative `ecco` ("here is") takes enclitics exactly as a verb does
# (`eccolo`, `eccomi`) but is tagged ADV/INTJ, never VERB, so the verb test
# below cannot see it. It is the only non-verb host of enclisis in Italian,
# which is why this is one constant and not the beginning of a word list.
PRESENTATIVE_HOST: str = "ecco"

# An Italian infinitive drops its final `e` before an enclitic: *caricare* +
# *lo* -> ``caricarlo``, so the base surfaces as ``caricar``. That apocopated
# form is not itself a word and is often absent from the lexicon even when the
# full infinitive is present, so the verb test restores the `e` before giving
# up. Regular morphology, not a word list: these are the three conjugations.
INFINITIVE_APOCOPE_ENDINGS: tuple[str, ...] = ("ar", "er", "ir")

# The full infinitives, and their reflexive forms, DERIVED from the same three
# conjugations rather than retyped. A word whose lemma ends in one of these is a
# form of that verb, which is how verb attestation is reconstructed when Stanza
# ships a lemmatizer that no longer records it (see `extract_stanza_lexicon`).
# Derived because a third hand-written copy of this fact had already appeared
# and had already diverged on whether reflexives count.
INFINITIVE_ENDINGS: tuple[str, ...] = tuple(
    stem + suffix for stem in INFINITIVE_APOCOPE_ENDINGS for suffix in ("e", "si")
)

# The apocopated monosyllabic imperatives, and ONLY these, double the initial
# consonant of the clitic that follows: *da'* + *mi* -> ``dammi``, *fa'* + *lo*
# -> ``fallo``, *va'* + *ci* -> ``vacci``. Every other host concatenates plainly.
# This is what lets the reconstruction check below accept ``dammelo`` while
# rejecting ``hallo``, whose ``ha`` is not a geminating host.
GEMINATING_HOSTS: frozenset[str] = frozenset({"da", "di", "fa", "sta", "va"})

# `gli` is the documented exception: it never doubles (*da'* + *gli* -> ``dagli``,
# never ``daggli``).
NON_GEMINATING_CLITIC: str = "gli"


class MwtExpansion(Enum):
    """Whether a word may expand into a multi-word token.

    An enum rather than a bool because the two outcomes are not symmetric
    defaults: ALLOW preserves Stanza's own analysis, SUPPRESS overrides it, and
    call sites read better naming which is which.
    """

    ALLOW = "allow"
    SUPPRESS = "suppress"


@dataclass(frozen=True, slots=True)
class ItalianLexicon:
    """Stanza's shipped Italian lexicon, as the two questions we ask of it.

    Sourced from the loaded lemmatizer's dictionaries (roughly 50k surface forms
    and 13k verb surface forms on stanza 1.13.0), so it tracks whatever Stanza
    is installed instead of being a table we maintain and let drift. The April
    2026 audit retired five such hand-maintained per-language tables precisely
    because they had drifted.
    """

    surface_forms: frozenset[str]
    verb_forms: frozenset[str]

    def is_known_word(self, form: str) -> bool:
        return form.lower() in self.surface_forms

    def is_attested_verb(self, form: str) -> bool:
        return form.lower() in self.verb_forms


@dataclass(frozen=True, slots=True)
class ItalianMwtPolicy:
    """The two Stanza-derived inputs the decision needs, resolved lazily.

    Injected rather than read out of worker state, for two reasons. The
    tokenizer postprocessor closure is built BEFORE the pipeline it belongs to
    exists, and the lexicon can only be read off a LOADED pipeline, so eager
    wiring is impossible. And keeping this module free of Stanza imports is what
    lets the rule be tested in milliseconds with plain data.

    ``propose_splits`` takes the tokens of each utterance in the batch and
    returns, per utterance, a mapping from token text to the split Stanza's MWT
    processor would produce for it. ``None`` means the probe is unavailable.

    It takes whole UTTERANCES, not bare words, because Stanza's MWT is
    context-sensitive and probing a word in isolation answers a different
    question. Measured 2026-07-28: ``hai`` alone is left whole, but in
    ``secondo la mia opinione hai ragione .`` it becomes *ha* + *i*. An earlier
    revision probed isolated words and therefore saw no split to judge, so every
    in-context over-split sailed through while the single-word cases were caught.

    Batching the whole set of utterances into one call keeps the extra work to a
    single pipeline pass per batch.

    ``force`` names the tokens the probe should be told to expand. Stanza's MWT
    is not merely permissive about that hint, it obeys it: ``aprilo`` is left
    whole by default and yields *apri* + *lo* with the real lemma *aprire* when
    hinted. That is what lets one probe pass answer in BOTH directions, since
    forcing is a no-op on tokens Stanza was already going to expand.
    """

    propose_splits: Callable[
        [Sequence[Sequence[str]], frozenset[str]],
        Sequence[Mapping[str, tuple[str, ...]]] | None,
    ]
    lexicon: Callable[[], ItalianLexicon | None]


def is_preposition_article_contraction(
    word: str, proposed_split: Sequence[str]
) -> bool:
    """Is this a genuine Italian preposition + definite article fusion?

    ``alla`` -> *a* + *la*, ``della`` -> *di* + *la*, ``nel`` -> *in* + *il*.
    These are true fusions rather than concatenations, so
    ``split_reconstructs_word`` deliberately does NOT apply: *di* + *la* cannot
    spell ``della`` by any regular rule.

    BOTH the surface and the analysis must check out. The analysis test rejects
    Stanza's ``la`` -> *il* + *i* and ``hai`` -> *ha* + *i*, which have the shape
    of a contraction but no preposition in the base. The surface test rejects
    foreign words the Italian model mangles into a passing shape, like English
    ``well`` -> *In* + *l*. Each catches what the other misses.
    """
    if len(proposed_split) != 2:
        return False
    if word.lower() not in ITALIAN_PREPOSITION_ARTICLE_CONTRACTIONS:
        return False
    preposition, article = proposed_split[0].lower(), proposed_split[1].lower()
    return (
        preposition in ITALIAN_ARTICLE_FUSING_PREPOSITIONS
        and article in ITALIAN_DEFINITE_ARTICLES
    )


def could_be_enclisis(word: str, lexicon: ItalianLexicon) -> bool:
    """Might *word* be a verb carrying enclitics, even though Stanza left it whole?

    A cheap, purely lexical pre-filter with no Stanza call: peel a maximal
    sequence of enclitics off the end and ask whether what remains can host
    them. It exists to find the tokens worth force-probing, so it errs toward
    admitting; the four pattern tests then decide, on Stanza's actual proposed
    split, whether the expansion is real.

    This is the entry point for the defect where Stanza declines to split a
    genuine imperative and invents a verb for the whole surface: ``aprilo``
    comes back as ``verb|aprilare`` and ``leggila`` as ``verb|leggilare``,
    neither of which exists in Italian.

    Known dictionary words are excluded up front. Without that, ``cavallo``
    peels to ``cava`` (a real verb) and would be force-probed on every batch,
    only to be rejected later by the lexicality test.
    """
    lowered = word.lower()
    if len(lowered) < 3 or lexicon.is_known_word(lowered):
        return False

    remainder = lowered
    peeled = 0
    # Longest-first so `glie` is preferred over `gli`, and at most three
    # clitics, which is the maximum Italian stacks (`raccontaglielo`).
    while peeled < 3:
        for clitic in sorted(ITALIAN_ENCLITICS, key=len, reverse=True):
            if remainder.endswith(clitic) and len(remainder) > len(clitic):
                remainder = remainder[: -len(clitic)]
                peeled += 1
                break
        else:
            break
        if peeled and can_host_enclisis(remainder, lexicon):
            return True
    return False


def is_valid_clitic_sequence(pieces: Sequence[str]) -> bool:
    """Is *pieces* a sequence of enclitics that Italian can actually spell?

    Every piece must be an enclitic, and no piece may be an `e`-form unless
    another clitic follows it. The second condition is what rejects English
    ``face`` analyzed as *fa* + *ce*: `ce` in final position is not Italian.
    """
    if not pieces:
        return False
    lowered = [p.lower() for p in pieces]
    if not all(p in ITALIAN_ENCLITICS for p in lowered):
        return False
    return lowered[-1] not in CLITICS_REQUIRING_A_FOLLOWER


def is_clitic_cluster(word: str, proposed_split: Sequence[str]) -> bool:
    """Is this a bare stack of enclitics, with no verb host?

    Italian writes an indirect+direct clitic sequence as one word: ``glielo``
    (to-him it) is *glie* + *lo*, ``gliela`` is *glie* + *la*. There is no verb
    in the token, so the verb+enclitic tests cannot see these, and they were
    being suppressed until the corpus scan surfaced them.

    Reconstruction is required and does the discriminating work: it admits
    ``glielo`` (which spells out exactly) while rejecting babble and names that
    happen to decompose into clitic-shaped pieces, such as ``tella`` -> *ti* +
    *la* (spells `tila`) and ``Lalla`` -> *La* + *la* (spells `lala`).
    """
    if len(proposed_split) < 2:
        return False
    if not split_reconstructs_word(word, proposed_split):
        return False
    return is_valid_clitic_sequence(proposed_split)


class Reconstruction(Enum):
    """HOW a split rebuilds its surface, which is not the same as whether it does.

    GEMINATED means the base was APOCOPATED (`da'` for `dare`). Collapsing that
    into "yes it reconstructs" discarded the one fact a caller needs to reason
    about the base the POS tagger will receive, which is why this is a variant
    and not the bool it used to be.
    """

    #: The parts concatenate to the surface. `di` + `glie` + `lo` = `diglielo`.
    PLAIN = "plain"
    #: Reconstructing needed the doubled consonant: `da` + `me` + `la` = `dammela`.
    GEMINATED = "geminated"


def reconstruct_split(
    word: str, proposed_split: Sequence[str]
) -> Reconstruction | None:
    """How does *proposed_split* account for every character of *word*, if at all?

    An enclisis analysis that cannot rebuild the surface is malformed by
    definition, and acting on one corrupts the data: Stanza proposes
    ``pentolone`` -> *pento* + *lo*, which silently drops ``ne``, so emitting it
    would give a two-item ``%mor`` describing a word that is not the word on the
    main tier. That case is ``None``.

    Plain concatenation is the normal case. The one licensed departure is
    gemination after an apocopated monosyllabic imperative, which is why
    ``dammelo`` (*da* + *me* + *lo*) is accepted while ``hallo`` (*ha* + *lo*)
    and ``tagliatelle`` (*tagliate* + *le*) are not.

    Note that a geminating HOST does not imply a geminated SPLIT: `gli` never
    doubles, so ``diglielo`` is `di` + `glie` + `lo` by plain concatenation even
    though `di` is in `GEMINATING_HOSTS`. Callers that care about the apocopated
    base must ask about the RECONSTRUCTION, not about the host.
    """
    target = word.lower()
    if "".join(proposed_split).lower() == target:
        return Reconstruction.PLAIN
    if len(proposed_split) < 2:
        return None
    base, first_clitic = proposed_split[0].lower(), proposed_split[1].lower()
    if base not in GEMINATING_HOSTS or first_clitic == NON_GEMINATING_CLITIC:
        return None
    if not first_clitic:
        return None
    geminated = base + first_clitic[0] + "".join(proposed_split[1:]).lower()
    return Reconstruction.GEMINATED if geminated == target else None


def split_reconstructs_word(word: str, proposed_split: Sequence[str]) -> bool:
    """Whether *proposed_split* rebuilds *word* at all, by either route."""
    return reconstruct_split(word, proposed_split) is not None


def can_host_enclisis(base: str, lexicon: ItalianLexicon) -> bool:
    """Can *base* carry an enclitic pronoun in Italian?

    True for an attested verb form, for an apocopated infinitive whose full form
    is attested (``caricar`` from *caricare*), and for the presentative ``ecco``,
    which hosts enclitics exactly as a verb does but is tagged ADV/INTJ and so is
    invisible to any verb test.
    """
    lowered = base.lower()
    if lowered == PRESENTATIVE_HOST:
        return True
    if lexicon.is_attested_verb(lowered):
        return True
    if lowered.endswith(INFINITIVE_APOCOPE_ENDINGS):
        return lexicon.is_attested_verb(lowered + "e")
    return False


class MwtPattern(Enum):
    """The multi-word token patterns Italian actually has.

    Exactly three, two of them closed. Anything Stanza proposes that matches
    none of these is an artifact, whether or not there is surrounding context.
    """

    PREPOSITION_ARTICLE = "preposition_article"  # alla = a + la      CLOSED
    ECCO_ENCLITIC = "ecco_enclitic"  # eccolo = ecco + lo CLOSED
    CLITIC_CLUSTER = "clitic_cluster"  # glielo = glie + lo CLOSED
    VERB_ENCLITIC = "verb_enclitic"  # giralo = gira + lo OPEN


def classify_split(
    word: str,
    proposed_split: Sequence[str],
    lexicon: ItalianLexicon,
) -> MwtPattern | None:
    """Which legitimate Italian pattern does this split instantiate, if any?

    ``None`` means the split matches no pattern the language has, which is the
    signature of a Stanza artifact. Context-independent by construction: these
    are facts about Italian, not about the utterance, which is why the same
    function answers for `cavallo` alone and for `mozzarella` mid-sentence.
    """
    if len(proposed_split) < 2:
        return None

    # Preposition + article is checked FIRST and separately, because these are
    # true fusions rather than concatenations: `di` + `la` cannot spell `della`
    # by any regular rule, so the reconstruction test below would reject every
    # genuine contraction in the language.
    if is_preposition_article_contraction(word, proposed_split):
        return MwtPattern.PREPOSITION_ARTICLE

    # A bare clitic stack has no host at all, so it is checked before the two
    # hosted patterns rather than falling through their base tests.
    if is_clitic_cluster(word, proposed_split):
        return MwtPattern.CLITIC_CLUSTER

    # Both hosted patterns must account for the whole word and attach only real
    # clitics. They differ solely in what is allowed to host them.
    if not split_reconstructs_word(word, proposed_split):
        return None
    if not is_valid_clitic_sequence(proposed_split[1:]):
        return None

    base = proposed_split[0].lower()
    if base == PRESENTATIVE_HOST:
        return MwtPattern.ECCO_ENCLITIC

    if not can_host_enclisis(base, lexicon):
        return None
    # A form that is itself a dictionary word is that word, not an assembly.
    # `cavallo` reaches here with a real verb base (`cava`) and a real clitic
    # (`lo`); only its own lexicality distinguishes it from `giralo`.
    if lexicon.is_known_word(word):
        return None
    return MwtPattern.VERB_ENCLITIC


def decide_expansion(
    word: str,
    proposed_split: Sequence[str] | None,
    lexicon: ItalianLexicon | None,
) -> MwtExpansion:
    """Should Stanza's proposed split of *word* be allowed to happen?

    ``proposed_split`` is what Stanza's MWT processor would produce for the word;
    ``None`` means the probe could not run. ``lexicon`` is Stanza's Italian
    lexicon, or ``None`` when it could not be extracted.

    When either input is unavailable the answer is ALLOW. A check that did not
    run must not silently downgrade the analysis: preserving Stanza's own
    behavior is the honest failure mode, and the alternative would reintroduce
    the very damage the closed-class allowlist caused on ``dammelo``.
    """
    if not word.strip():
        return MwtExpansion.ALLOW
    if proposed_split is None or lexicon is None:
        return MwtExpansion.ALLOW
    # Stanza is not proposing a split, so there is nothing to suppress.
    if len(proposed_split) < 2:
        return MwtExpansion.ALLOW
    if classify_split(word, proposed_split, lexicon) is None:
        return MwtExpansion.SUPPRESS
    # `is_geminated_split` is deliberately NOT consulted here. Whether these
    # should be suppressed is open, and it is recorded where this repo keeps
    # such things: Defect 8, "Open residue", in
    # `book/src/batchalign/reference/stanza-limitations.md`.
    return MwtExpansion.ALLOW


def is_geminated_split(word: str, proposed_split: Sequence[str]) -> bool:
    """Would splitting *word* leave an APOCOPATED base?

    Defined and deliberately NOT called: whether `decide_expansion` should
    suppress these is an open question recorded as Defect 8's residue in
    `book/src/batchalign/reference/stanza-limitations.md`. Kept as real,
    type-checked, tested code rather than a commented-out block, so that
    enabling it is one call site and not a transcription.

    Asks about the RECONSTRUCTION, not about the host, which is the trap the
    `Reconstruction` variant exists to close: `di` is a geminating host but
    `diglielo` reconstructs plainly, because `gli` never doubles.
    """
    return reconstruct_split(word, proposed_split) is Reconstruction.GEMINATED
