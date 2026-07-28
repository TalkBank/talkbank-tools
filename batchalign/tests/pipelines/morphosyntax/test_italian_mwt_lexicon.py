"""Pin the Stanza lexicon the Italian single-word MWT policy depends on.

The policy reads Stanza's shipped Italian word list off the lemmatizer's
trainer, which is a PRIVATE Stanza attribute
(``processors["lemma"]._trainer.word_dict``). That is a deliberate trade: the
alternative is a hand-maintained Italian table of our own, and the April 2026
MWT audit retired five such per-language tables precisely because they drifted
away from what the models actually do.

The cost of reaching into a private attribute is that a Stanza upgrade can move
it. These tests are how that break surfaces in CI instead of silently disabling
the policy and letting invented verbs back into published corpora. They assert
the shape, the scale, and a handful of specific entries the rule leans on.

Companion to ``test_italian_mwt.py``, which tests the rule itself against plain
data and needs no models at all.

Stanza is imported lazily inside fixtures so pytest collection does not pay the
stanza+torch cascade when these tests are deselected.
"""

from __future__ import annotations

from typing import TYPE_CHECKING

import pytest

from batchalign.inference._italian_mwt import (
    ItalianLexicon,
    MwtExpansion,
    decide_expansion,
)
from batchalign.worker._stanza_loading import (
    StanzaLexiconUnavailableError,
    extract_stanza_lexicon,
)

if TYPE_CHECKING:
    from collections.abc import Iterator


@pytest.fixture(scope="module")
def italian_lexicon() -> Iterator[ItalianLexicon]:
    """The real Italian lexicon, extracted the way production extracts it."""
    stanza = pytest.importorskip("stanza", reason="stanza not installed")
    from stanza import DownloadMethod

    nlp = stanza.Pipeline(
        lang="it",
        processors="tokenize,pos,lemma",
        download_method=DownloadMethod.REUSE_RESOURCES,
        tokenize_no_ssplit=True,
        tokenize_pretokenized=True,
    )
    yield extract_stanza_lexicon(nlp)


def test_extraction_finds_a_real_lexicon(italian_lexicon: ItalianLexicon) -> None:
    """Shape and scale. Measured on stanza 1.13.0: 50,502 forms, 13,248 verbs.

    The floors are deliberately far below the observed values: this test exists
    to catch "the attribute moved and we got nothing", not to pin an exact
    count that a legitimate model update would change.
    """
    assert len(italian_lexicon.surface_forms) > 10_000
    assert len(italian_lexicon.verb_forms) > 3_000
    # The two dictionaries are independent (word_dict vs composite_dict), so
    # neither contains the other. They must still describe the same language:
    # if the overlap collapsed, one of them is being read wrongly.
    overlap = italian_lexicon.verb_forms & italian_lexicon.surface_forms
    assert len(overlap) > len(italian_lexicon.verb_forms) // 2, (
        "verb forms and surface forms barely overlap, so one dictionary is "
        f"being misread: {len(overlap)} of {len(italian_lexicon.verb_forms)}"
    )


def test_the_entries_the_rule_leans_on_are_present(
    italian_lexicon: ItalianLexicon,
) -> None:
    """Specific facts the decision depends on, not just a population count.

    Each of these is load-bearing for a measured corpus case: `cavallo` is what
    stops *cava* + *lo*, and `gira`/`prendi` are what let `giralo`/`prendilo`
    keep splitting.
    """
    for word in ("cavallo", "bello", "giallo", "quello", "palla", "attenzione"):
        assert italian_lexicon.is_known_word(word), f"{word} should be a known word"
    for verb in ("gira", "prendi", "guarda", "da", "di", "chiama"):
        assert italian_lexicon.is_attested_verb(verb), f"{verb} should be a verb form"
    # The assembled forms must NOT be known words, or the lexicality test would
    # suppress the very imperatives this whole change exists to protect.
    for assembled in ("dammelo", "diglielo", "giralo", "prendilo", "eccolo"):
        assert not italian_lexicon.is_known_word(assembled), (
            f"{assembled} is an assembled form and must not be in the lexicon"
        )


def test_ecco_is_not_a_verb_so_its_exemption_is_load_bearing(
    italian_lexicon: ItalianLexicon,
) -> None:
    """`ecco` hosts enclitics but is tagged ADV/INTJ, never VERB.

    If this ever becomes true, the PRESENTATIVE_HOST special case can go. Until
    then, deleting it silently loses `eccolo`, which the golden test requires.
    """
    assert not italian_lexicon.is_attested_verb("ecco")


def test_extraction_fails_loudly_rather_than_returning_empty() -> None:
    """A missing lexicon must raise, never quietly answer "no" to everything.

    An empty lexicon would report every word as unknown and every base as a
    non-verb, which suppresses every multi-word token while looking healthy.
    """

    class NoLemmatizer:
        processors: dict[str, object] = {}

    with pytest.raises(StanzaLexiconUnavailableError):
        extract_stanza_lexicon(NoLemmatizer())

    class EmptyTrainer:
        word_dict: dict[str, str] = {}
        composite_dict: dict[tuple[str, str], str] = {}

    class EmptyLemmatizer:
        def __init__(self) -> None:
            self.processors = {"lemma": type("P", (), {"_trainer": EmptyTrainer()})()}

    with pytest.raises(StanzaLexiconUnavailableError):
        extract_stanza_lexicon(EmptyLemmatizer())


@pytest.mark.parametrize(
    ("word", "split", "expected"),
    [
        # Verb+enclitic imperatives that absolutely must keep splitting: they
        # are frequent in child-directed Italian and an OPEN class.
        ("dammelo", ("da", "me", "lo"), MwtExpansion.ALLOW),
        ("diglielo", ("di", "glie", "lo"), MwtExpansion.ALLOW),
        ("giralo", ("gira", "lo"), MwtExpansion.ALLOW),
        ("dimmelo", ("di", "me", "lo"), MwtExpansion.ALLOW),
        ("chiamalo", ("chiama", "lo"), MwtExpansion.ALLOW),
        ("eccolo", ("ecco", "lo"), MwtExpansion.ALLOW),
        # The highest-count corpus damage.
        ("bello", ("ib", "lo"), MwtExpansion.SUPPRESS),
        ("giallo", ("gia", "lo"), MwtExpansion.SUPPRESS),
        ("quello", ("qu", "lo"), MwtExpansion.SUPPRESS),
        ("palla", ("pa", "la"), MwtExpansion.SUPPRESS),
        ("cavallo", ("cava", "lo"), MwtExpansion.SUPPRESS),
        ("attenzione", ("attenzi", "ne"), MwtExpansion.SUPPRESS),
    ],
)
def test_the_rule_against_the_real_lexicon(
    italian_lexicon: ItalianLexicon,
    word: str,
    split: tuple[str, ...],
    expected: MwtExpansion,
) -> None:
    """The decision, run against the REAL lexicon rather than a fixture.

    `test_italian_mwt.py` proves the rule is internally consistent; this proves
    the lexicon it will actually be given supports those verdicts. A rule that
    passes on a hand-built fixture and fails on the shipped lexicon is the gap
    this closes. Splits are what stanza 1.13.0 really produced, measured
    2026-07-28.
    """
    assert decide_expansion(word, split, italian_lexicon) is expected
