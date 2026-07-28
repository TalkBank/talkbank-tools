"""Italian single-word MWT policy: which proposed splits are genuine.

Model-free by construction. The decision takes Stanza's proposed split and a
lexicon as plain data, so these run in milliseconds and pin the RULE; the
end-to-end behavior against real Stanza is pinned by
`golden_morphotag_ita_single_word_utterances_are_not_split` in the ml_golden
suite, which is the test that actually gates the fix.

Every split below is what stanza 1.13.0 really produced for that word, measured
2026-07-28; none are invented for the test.
"""

from __future__ import annotations

import pytest

from batchalign.inference._italian_mwt import (
    ItalianLexicon,
    MwtExpansion,
    can_host_enclisis,
    could_be_enclisis,
    decide_expansion,
    is_clitic_cluster,
    is_preposition_article_contraction,
    is_single_word_utterance,
    is_valid_clitic_sequence,
    split_reconstructs_word,
)

# A stand-in for Stanza's shipped lexicon carrying only the entries these cases
# exercise. Real one is ~50k surface forms and ~13k verb forms.
LEXICON = ItalianLexicon(
    surface_forms=frozenset({
        # Ordinary words that must never be split.
        "cavallo", "bello", "giallo", "quello", "palla", "attenzione",
        "uccelli", "spaghetti", "dondolo", "cancello", "viola", "scivola",
        "disegna", "cielo", "pentola", "cavolo", "coltello", "macchine",
        "gallina", "persone",
        # A real imperative that Stanza's lexicon also lists as a word, which is
        # why the real rule under-splits it. Measured, not aspirational.
        "svegliati",
        # Verb forms are words too.
        "cava", "cavo", "pento", "gira", "apri", "prendi", "guarda", "da",
        "di", "fa", "sta", "va", "chiama", "tagliate", "ecco",
        "leggi", "butta",
        # UD Italian tokenizes `fammi` as *fam* + *mi*, so the GEMINATED bases
        # are themselves lexicon entries. That is why the pre-filter can see
        # through gemination without knowing the rule.
        "fam", "dim", "dam",
    }),
    verb_forms=frozenset({
        "cava", "cavo", "pento", "gira", "apri", "prendi", "guarda", "da",
        "di", "fa", "sta", "va", "chiama", "tagliate", "scivola", "disegna",
        "svegliati", "caricare", "leggi", "butta", "fam", "dim", "dam",
    }),
)


@pytest.mark.parametrize(
    ("word", "split"),
    [
        # Verb + enclitic imperatives: an OPEN class, the whole reason a
        # closed-class allowlist cannot work. These forms are frequent in
        # child-directed Italian and must be processed properly, no exception.
        ("dammelo", ("da", "me", "lo")),
        ("diglielo", ("di", "glie", "lo")),
        ("giralo", ("gira", "lo")),
        ("aprila", ("apri", "la")),
        ("prendilo", ("prendi", "lo")),
        ("guardalo", ("guarda", "lo")),
        ("chiamalo", ("chiama", "lo")),
        # `ecco` + enclitic: the one non-verb host of enclisis in Italian.
        ("eccolo", ("ecco", "lo")),
        ("eccomi", ("ecco", "mi")),
    ],
)
def test_genuine_multi_word_tokens_are_allowed(word: str, split: tuple[str, ...]) -> None:
    assert decide_expansion(word, split, LEXICON) is MwtExpansion.ALLOW


@pytest.mark.parametrize(
    ("word", "split", "why"),
    [
        # Rejected because the split does not account for the whole word.
        ("cavallo", ("cava", "lo"), "cavalo is not cavallo, and cava does not geminate"),
        ("cavalla", ("cava", "la"), "cavala is not cavalla; a mare, not an imperative"),
        ("hallo", ("ha", "lo"), "ha is not a geminating host, so halo is not hallo"),
        ("tagliatelle", ("tagliate", "le"), "tagliatele is not tagliatelle; pasta"),
        ("pentolone", ("pento", "lo"), "the split silently drops ne"),
        ("stampella", ("sta", "me", "la"), "stammela is not stampella; a crutch"),
        ("coltello", ("colte", "lo"), "coltelo is not coltello"),
        # Rejected by the enclitic test.
        ("gallina", ("galli", "na"), "na is not an Italian clitic"),
        ("mucche", ("mu", "cce", "he"), "neither cce nor he is a clitic"),
        ("disegna", ("di", "se", "gna"), "gna is not a clitic"),
        # Rejected by the verb-base test: the bases are not Italian verb forms.
        ("bello", ("ib", "lo"), "ib is not a word"),
        ("pello", ("ip", "lo"), "ip is not a word"),
        ("giallo", ("gia", "lo"), "gia is not a verb"),
        ("quello", ("qu", "lo"), "qu is not a verb"),
        ("palla", ("pa", "la"), "pa is not a verb"),
        ("attenzione", ("attenzi", "ne"), "attenzi is not a verb"),
        ("uccelli", ("u", "ce", "li"), "u is not a verb"),
        ("spaghetti", ("spaghet", "ti"), "spaghet is not a verb"),
        ("cielo", ("cie", "lo"), "cie is not a verb"),
        ("dondolo", ("dondo", "lo"), "dondo is not a verb; dondolo is a noun"),
        # Rejected by the lexicality test: everything else about these passes.
        ("pentola", ("pento", "la"), "pento is a real verb but pentola is a pot"),
        ("cavolo", ("cavo", "lo"), "cavo is a real verb but cavolo is a cabbage"),
    ],
)
def test_over_splits_are_suppressed(word: str, split: tuple[str, ...], why: str) -> None:
    assert decide_expansion(word, split, LEXICON) is MwtExpansion.SUPPRESS, why


def test_each_of_the_four_tests_is_load_bearing() -> None:
    """Removing any one test would let a measured real failure through.

    Guards against a later simplification that drops a check because the others
    look sufficient. Each case below is rejected by exactly ONE of the four, so
    deleting that check turns this test red.
    """
    # Reconstruction only: real verb base, real clitic, not a known word, but
    # `pento` + `lo` cannot rebuild `pentolone`.
    assert decide_expansion("pentolone", ("pento", "lo"), LEXICON) is (
        MwtExpansion.SUPPRESS
    ), "reconstruction: the split drops ne"
    # Enclitic only: `di` is an attested verb and `digna` is not a known word.
    assert decide_expansion("digna", ("di", "gna"), LEXICON) is (
        MwtExpansion.SUPPRESS
    ), "enclitic test: gna is not a clitic"
    # Verb-base only: `lo` is a clitic, `iblo` reconstructs and is not a known
    # word, so nothing else rejects it.
    assert decide_expansion("iblo", ("ib", "lo"), LEXICON) is (
        MwtExpansion.SUPPRESS
    ), "verb-base test: ib is not an attested verb"
    # Lexicality only: reconstructs exactly, real clitic, real verb base.
    assert decide_expansion("pentola", ("pento", "la"), LEXICON) is (
        MwtExpansion.SUPPRESS
    ), "lexicality test: pentola is itself a word"


@pytest.mark.parametrize(
    ("word", "split"),
    [
        ("dammelo", ("da", "me", "lo")),   # da' + mmelo
        ("dillo", ("di", "lo")),           # di' + llo
        ("fallo", ("fa", "lo")),           # fa' + llo
        ("vallo", ("va", "lo")),           # va' + llo
        ("dagli", ("da", "gli")),          # gli is the documented non-doubler
    ],
)
def test_gemination_after_apocopated_imperatives_reconstructs(
    word: str, split: tuple[str, ...]
) -> None:
    """Only da'/di'/fa'/sta'/va' double the following clitic, and gli never does."""
    assert split_reconstructs_word(word, split), f"{word} should reconstruct from {split}"


def test_gemination_is_not_granted_to_other_hosts() -> None:
    """`ha` is not an apocopated imperative, so `hallo` is not `ha` + `lo`."""
    assert not split_reconstructs_word("hallo", ("ha", "lo"))
    assert not split_reconstructs_word("tagliatelle", ("tagliate", "le"))


def test_apocopated_infinitives_can_host_enclitics() -> None:
    """An infinitive drops its final e before a clitic: caricare + lo -> caricarlo.

    The apocopated base is often absent from the lexicon even when the full
    infinitive is present, which is why the verb test restores the e.
    """
    assert can_host_enclisis("caricar", LEXICON), "caricar comes from caricare"
    assert can_host_enclisis("gira", LEXICON), "a plain imperative still works"
    assert can_host_enclisis("ecco", LEXICON), "the presentative hosts enclitics"
    assert not can_host_enclisis("pennar", LEXICON), "pennare is not a verb"


def test_unavailable_inputs_preserve_stanza_behaviour() -> None:
    """A check that did not run must not downgrade the analysis.

    Suppressing on a failed probe would silently reintroduce exactly the damage
    the closed-class allowlist caused, and would do it invisibly.
    """
    assert decide_expansion("dammelo", None, LEXICON) is MwtExpansion.ALLOW
    assert decide_expansion("dammelo", ("da", "me", "lo"), None) is (
        MwtExpansion.ALLOW
    )
    assert decide_expansion("cavallo", None, None) is MwtExpansion.ALLOW


def test_a_word_stanza_does_not_split_is_left_alone() -> None:
    """No proposed split means there is nothing to suppress."""
    assert decide_expansion("casa", ("casa",), LEXICON) is MwtExpansion.ALLOW
    assert decide_expansion("", (), LEXICON) is MwtExpansion.ALLOW
    assert decide_expansion("   ", None, LEXICON) is MwtExpansion.ALLOW


def test_case_is_normalized() -> None:
    """Utterance-initial capitals must not change the verdict."""
    assert decide_expansion("Cavallo", ("Cava", "lo"), LEXICON) is (
        MwtExpansion.SUPPRESS
    )
    assert decide_expansion("Giralo", ("Gira", "lo"), LEXICON) is (
        MwtExpansion.ALLOW
    )


@pytest.mark.parametrize(
    ("word", "split"),
    [
        ("alla", ("a", "la")),
        ("della", ("di", "la")),
        ("nel", ("in", "il")),
        ("sul", ("su", "il")),
        ("dai", ("da", "i")),
        ("col", ("con", "il")),
        ("degli", ("di", "gli")),
        ("dell'", ("di", "l'")),
    ],
)
def test_preposition_article_contractions_are_recognized(
    word: str, split: tuple[str, ...]
) -> None:
    """Genuine fusions. Splits are what stanza 1.13.0 really produced."""
    assert is_preposition_article_contraction(word, split), f"{word} = {split}"


@pytest.mark.parametrize(
    ("word", "split", "why"),
    [
        ("la", ("il", "i"), "il is an article, not a preposition"),
        ("hai", ("ha", "i"), "ha is 2sg of avere, not a preposition"),
        ("stella", ("sti", "la"), "sti is not a preposition"),
        ("dammelo", ("da", "me", "lo"), "three pieces is not a contraction"),
        ("giralo", ("gira", "lo"), "gira is a verb, lo is not an article"),
    ],
)
def test_contraction_shaped_but_not_contractions(
    word: str, split: tuple[str, ...], why: str
) -> None:
    """The whole point of validating the analysis rather than the surface.

    `la` -> il + i and `hai` -> ha + i are limitation 3: they have the SHAPE of
    a contraction and no preposition in the base.
    """
    assert not is_preposition_article_contraction(word, split), why


@pytest.mark.parametrize(
    ("word", "split", "is_cluster", "why"),
    [
        ("glielo", ("glie", "lo"), True, "to-him-it, a real clitic stack"),
        ("gliela", ("glie", "la"), True, "to-him-it (fem)"),
        # Babble and names that decompose into clitic-shaped pieces. These are
        # why reconstruction is required: each spells out to something else.
        ("tella", ("ti", "la"), False, "ti + la spells tila, not tella"),
        ("telle", ("ti", "le"), False, "ti + le spells tile, not telle"),
        ("Lalla", ("La", "la"), False, "La + la spells lala, not Lalla"),
        ("lallo", ("la", "lo"), False, "la + lo spells lalo, not lallo"),
    ],
)
def test_clitic_clusters_need_no_verb_host(
    word: str, split: tuple[str, ...], is_cluster: bool, why: str
) -> None:
    """Italian writes an indirect+direct clitic sequence as one word.

    There is no verb in the token, so the verb tests cannot see these. Each
    split below is what Stanza 1.13.0 itself proposes for that surface.
    """
    assert is_clitic_cluster(word, split) is is_cluster, why


def test_clitic_clusters_are_allowed_end_to_end() -> None:
    """The pattern must reach the decision, not just the predicate."""
    assert decide_expansion("glielo", ("glie", "lo"), LEXICON) is MwtExpansion.ALLOW
    assert decide_expansion("tella", ("ti", "la"), LEXICON) is MwtExpansion.SUPPRESS


@pytest.mark.parametrize(
    ("word", "split"),
    [
        # Surfaces outside the contracted paradigm that the tagger can
        # nevertheless label ADP+DET (splits are what raw Stanza proposed).
        ("well", ("In", "l")),      # English
        ("wel", ("In", "il")),
        ("dem", ("di", "i")),       # German
        ("au", ("a", "i")),         # French
        ("all", ("a", "l")),        # English
        ("dài", ("di", "i")),       # Italian verb "give!", not a contraction
    ],
)
def test_foreign_lookalikes_are_not_contractions(
    word: str, split: tuple[str, ...]
) -> None:
    """The structural ADP+DET test alone passes these; the surface test rejects.

    None of these surfaces is in Italian's closed contracted paradigm, so none
    may be treated as a preposition+article fusion, whatever the tagger says
    about the pieces.
    """
    assert not is_preposition_article_contraction(word, split)
    assert decide_expansion(word, split, LEXICON) is MwtExpansion.SUPPRESS


def test_contractions_are_fusions_so_reconstruction_does_not_apply() -> None:
    """`di` + `la` cannot spell `della` by any regular rule, and that is fine.

    Documents why `is_preposition_article_contraction` is checked BEFORE, and
    independently of, `split_reconstructs_word`. Wiring them the other way round
    would reject every genuine contraction in the language.
    """
    assert not split_reconstructs_word("della", ("di", "la"))
    assert not split_reconstructs_word("nel", ("in", "il"))
    assert is_preposition_article_contraction("della", ("di", "la"))


@pytest.mark.parametrize(
    ("pieces", "valid", "why"),
    [
        (("lo",), True, "a plain final clitic"),
        (("ne",), True, "ne IS a real final clitic: dammene, scegline"),
        (("mi",), True, "plain"),
        (("me", "lo"), True, "e-form followed by another clitic"),
        (("glie", "lo"), True, "glie always takes a follower"),
        (("ce", "lo"), True, "ce before another clitic"),
        (("ce",), False, "bare final ce is not Italian: this is English `face`"),
        (("me",), False, "bare final me is not Italian"),
        (("ve",), False, "bare final ve: this is the name `Dave`"),
        (("glie",), False, "glie never stands alone"),
        (("na",), False, "not a clitic at all"),
        ((), False, "empty"),
    ],
)
def test_e_form_clitics_require_a_follower(
    pieces: tuple[str, ...], valid: bool, why: str
) -> None:
    """`mi` becomes `me` only before another clitic, so a final `me` is impossible.

    This constraint is what keeps force-splitting from fabricating structure on
    surfaces that merely end in clitic-shaped letters: without it English `face`
    would split to *fa* + *ce* and `Dave` to *Da* + *ve* (verified against raw
    Stanza).
    """
    assert is_valid_clitic_sequence(pieces) is valid, why


@pytest.mark.parametrize(
    ("word", "plausible", "why"),
    [
        ("aprilo", True, "apri is an attested verb, lo a clitic"),
        ("leggila", True, "leggi + la"),
        ("buttalo", True, "butta + lo"),
        ("dimmi", True, "gemination: da'/di' style host"),
        ("cavallo", False, "a known dictionary word, excluded up front"),
        ("pentola", False, "a known dictionary word"),
        ("casa", False, "no clitic tail"),
        ("lo", False, "too short to be host plus clitic"),
    ],
)
def test_pre_filter_finds_forcing_candidates(
    word: str, plausible: bool, why: str
) -> None:
    """The cheap lexical gate that decides which unsplit tokens are worth probing.

    It errs toward admitting; the four pattern tests then judge the split Stanza
    actually proposes. Known dictionary words are excluded up front so `cavallo`
    is not force-probed on every batch only to be rejected later.
    """
    assert could_be_enclisis(word, LEXICON) is plausible, why


def test_single_word_utterance_detection_ignores_punctuation() -> None:
    """CHAT carries the terminator as a token, so one word means length 2."""
    assert is_single_word_utterance(["macchine", "."])
    assert is_single_word_utterance(["attenzione", "!"])
    assert is_single_word_utterance(["sì"])
    assert not is_single_word_utterance(["apri", "via", "!"])
    assert not is_single_word_utterance(["la", "stazione", "."])
