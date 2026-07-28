"""Italian MWT policy: what may expand when expansion is suppressed."""

from __future__ import annotations

from batchalign.inference._italian_mwt import (
    is_closed_class_italian_mwt,
    is_single_word_utterance,
)


def test_preposition_article_contractions_are_closed_class() -> None:
    """These must keep expanding: they are genuine Italian MWTs."""
    for word in ("della", "nel", "sulla", "dagli", "al", "dai", "coi", "dell'"):
        assert is_closed_class_italian_mwt(word), f"{word!r} is a real contraction"


def test_ecco_clitic_forms_are_closed_class() -> None:
    """`eccolo` is the one single-word form the corpus needs preserved.

    Suppressing MWT on isolated words fixes 9 of the 10 measured corpus
    failures; `eccolo` is the tenth, and it is only recoverable because
    `ecco`+clitic is a closed class.
    """
    for word in ("eccolo", "eccola", "eccomi", "eccone", "eccoci"):
        assert is_closed_class_italian_mwt(word), f"{word!r} is ecco + clitic"


def test_the_corpus_failures_are_not_closed_class() -> None:
    """Every word Stanza destroyed must fall outside the allowlist.

    These are the real single-word utterances measured on 2026-07-28 that
    stanza 1.13.0 split into nonexistent verbs plus clitics.
    """
    for word in ("attenzione", "macchine", "persone", "stazione", "canzone",
                 "gallina", "cavallo", "mucche"):
        assert not is_closed_class_italian_mwt(word), (
            f"{word!r} is an ordinary noun and must not be expanded"
        )


def test_verb_enclitic_is_deliberately_not_allowlisted() -> None:
    """Open class: undecidable from surface form, so it is not allowlisted.

    `dammi`/`portarmelo` are genuine MWTs, but admitting them by pattern would
    also admit `cavallo` -> cava+lo and `attenzione` -> attenzi+ne, whose bases
    are equally verb-shaped. Losing the split on a rare one-word imperative is
    the accepted cost of never inventing a verb.
    """
    for word in ("dammi", "portarmelo", "dirgli", "farlo"):
        assert not is_closed_class_italian_mwt(word)


def test_case_and_whitespace_are_normalized() -> None:
    assert is_closed_class_italian_mwt("Della")
    assert is_closed_class_italian_mwt("  nel  ")
    assert not is_closed_class_italian_mwt("")


def test_single_word_utterance_detection_ignores_punctuation() -> None:
    """CHAT carries the terminator as a token, so one word means length 2."""
    assert is_single_word_utterance(["macchine", "."])
    assert is_single_word_utterance(["attenzione", "!"])
    assert is_single_word_utterance(["sì"])
    assert not is_single_word_utterance(["apri", "via", "!"])
    assert not is_single_word_utterance(["la", "stazione", "."])
