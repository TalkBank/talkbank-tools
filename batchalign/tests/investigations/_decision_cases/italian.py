"""Italian decision-probe cases: the sentence terminator as an input to Stanza.

Adjudication status (2026-07-31)
--------------------------------
These cases lock the finding that closes the `dammela` investigation: Stanza's
Italian model analyses a clitic-cluster imperative CORRECTLY when the utterance
carries its sentence-final terminator, and INCORRECTLY when it does not. The
terminator is not decoration; it is evidence the model was trained to use.

Observed, four conditions and one variable:

===============================  ======================================
input                            analysis of `dammela`
===============================  ======================================
`per favore dammela`             ADJ, lemma `dammelo`, amod; no MWT
`per favore dammela .`           VERB `dare` root + PRON me + PRON la
`per favore da me la`            `da` ADP, case (the prepositional read)
`per favore da me la .`          VERB `dare` root + PRON + PRON
===============================  ======================================

The second row is the whole answer to a defect that had survived three
falsified hypotheses: the worker was building Stanza's input by joining the
CHAT words and dropping the terminator the Rust side had supplied, so Stanza
saw a fragment. `MorphosyntaxBatchItem.terminator` existed, with a `"."`
default, and nothing read it, which is why no test could see the loss.

The pre-split rows matter independently. They show the symptom does NOT come
from the Italian MWT policy's force-split handing the tagger a homograph base:
the split form is analysed correctly too, once the terminator is present. That
retires the "splitting causes it" hypothesis on direct evidence rather than
inference.

Gold conventions
----------------
Gold is UD best-effort linguistic judgment, and its LENGTH must match the
number of UD words Stanza actually emits on that side (the harness comparator
requires this; Stanza owns the count, we own the labels). The pre side emits
one unexpanded word, so its gold is the single label that word should carry.
"""

from __future__ import annotations

from .._decision_probe_types import (
    CandidateClass,
    DecisionOutcome,
    DecisionProbeCase,
    Gold,
    TokenMapping,
)

CASES: tuple[DecisionProbeCase, ...] = (
    DecisionProbeCase(
        label="dammela_needs_its_terminator",
        utterance_prose="per favore dammela .",
        pre_words=("per", "favore", "dammela"),
        post_words=("per", "favore", "dammela", "."),
        affected_mappings=(
            TokenMapping(
                pre_token_indices=(2,),
                post_token_indices=(2,),
                gold=Gold(
                    # Pre emits ONE unexpanded UD word. Whatever else is true,
                    # `dammela` is a verb form, not an adjective.
                    pre_upos=("VERB",),
                    # Post expands to the clitic cluster, which is the point.
                    post_upos=("VERB", "PRON", "PRON"),
                ),
            ),
        ),
        expected_outcome=DecisionOutcome.POST_STRICTLY_BETTER,
        rationale=(
            "Without the terminator Stanza reads `dammela` as ADJ with lemma "
            "`dammelo` and does not MWT-expand it at all; with it, the model "
            "produces the correct `dare` + me + la. This is the mechanism "
            "behind the mis-tagged head reported for this surface, and the "
            "reason the worker must pass the terminator it is given."
        ),
        candidate_class=CandidateClass.SENTENCE_PERIOD,
    ),
    DecisionProbeCase(
        label="dammela_presplit_needs_its_terminator",
        utterance_prose="per favore da me la .",
        pre_words=("per", "favore", "da", "me", "la"),
        post_words=("per", "favore", "da", "me", "la", "."),
        affected_mappings=(
            TokenMapping(
                pre_token_indices=(2,),
                post_token_indices=(2,),
                gold=Gold(pre_upos=("VERB",), post_upos=("VERB",)),
            ),
        ),
        expected_outcome=DecisionOutcome.POST_STRICTLY_BETTER,
        rationale=(
            "The control for the force-split hypothesis. Presented already "
            "split, the host `da` is still ADP without the terminator and "
            "VERB `dare` with it, so the enclisis split is not what causes "
            "the prepositional reading."
        ),
        candidate_class=CandidateClass.SENTENCE_PERIOD,
    ),
)
