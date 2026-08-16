#!/usr/bin/env python3
"""Fail if the pre-push hook does not run everything CI runs.

The hook and the workflow are two lists of what must pass, and two lists drift.
This one drifted far enough that the hook printed "All pre-push checks passed"
for three pushes CI then rejected on 2026-08-14: it ran `fmt`, an affected-only
compile check, and clippy solely behind an environment variable that defaulted
to off, while CI ran `make batchalign-ci-rust` (clippy, audits, check, tests,
doctests, integration, PyO3).

The cure is not a longer hook, it is the same command. This asserts that every
`make` target a push-triggered workflow invokes is also reached by the hook, so
a step added to CI cannot silently stop being checked before a push.

"Reached by", not "written in": coverage is computed through the Makefile, so a
hook line reading `make batchalign-ci-rust` covers everything that target's
recipe invokes, transitively. Comparing the two texts literally would demand
that the hook restate the recipe, which is the mirroring this file exists to
prevent, one level down.

Modelled on chatter's `check_ci_gate_sync.py`, which exists because that
repository had FOUR independent lists of what must pass and all four had
drifted.

Usage: python3 scripts/check_push_gate_sync.py
"""

from __future__ import annotations

import re
import sys
from pathlib import Path
from typing import NamedTuple

REPO = Path(__file__).resolve().parents[1]
HOOK = REPO / "scripts" / "pre-push.sh"
MAKEFILE = REPO / "Makefile"
WORKFLOWS = REPO / ".github" / "workflows"

#: Targets a workflow may run that the hook is not expected to.
#:
#: Each needs a REASON, because an unexplained exemption is how the drift
#: starts again. Keep this list short; if it grows, the hook is the thing that
#: should change.
#:
#: This list is also the honest statement of what a green hook does NOT prove.
#: Everything here is covered only once CI runs, which is why anything touching
#: these areas goes to a branch first (docs/contributing/pushing.md).
EXEMPT: dict[str, str] = {
    # Builds the dashboard's JS bundle. Needs `npm ci` against the network and
    # is not a correctness gate on committed Rust.
    "batchalign-dashboard-build": "network npm install, not a correctness gate",
    # Runs in its own workflow job with its own Linux toolchain setup; the hook
    # covers the Rust side that can fail from committed content.
    "batchalign-build-pyo3": "separate job with its own toolchain setup",
    # Everything below needs `batchalign-python-prepare`, i.e. a maturin release
    # wheel built and installed into the dev environment. That is minutes, so
    # putting it in the hook would recreate the pressure that produced the
    # hand-written subset the hook used to be. The SOURCE-only half of the
    # Python gate (`batchalign-lint-python-source`) is deliberately split out
    # and IS in the hook.
    "batchalign-ci-python": "needs a built wheel; minutes, not seconds",
    "batchalign-lint-python": "needs a built wheel; use -source in the hook",
    "batchalign-typecheck-python": "needs a built wheel; mypy against the install",
    "batchalign-test-python": "needs a built wheel",
    "batchalign-python-prepare": "builds the wheel these depend on",
    "batchalign-build-wheel": "maturin release build",
    # NOT here any more: `batchalign-ipc-schema-check`. It was exempt for
    # "needs the built binary", which is true in isolation and irrelevant in
    # the hook, where `batchalign-ci-rust` has already built it and the check
    # costs 0 s. It is in the hook now, after a docstring edit regenerated six
    # IPC schemas and CI caught what every local gate had passed.
    #
    # Drift checks against generated artifacts that need the built binary or the
    # npm-installed frontend, and run in their own jobs with that setup.
    "batchalign-dashboard-api-check": "needs the npm-installed frontend",
    "batchalign-runtime-check": "runs in the wheel-installed job",
}


def make_targets(text: str) -> set[str]:
    """Every `make <target>` invoked in a shell snippet."""
    return {
        match.group(1) for match in re.finditer(r"\bmake\s+([a-z0-9][a-z0-9-]*)", text)
    }


class Uncovered(NamedTuple):
    """A gate CI runs on a push that the hook does not reach.

    Named rather than a bare pair because both fields are strings and the
    reporting loop had them in scope beside a `Path` of the same name.
    """

    workflow: str
    target: str


def makefile_recipes() -> dict[str, str]:
    """Each target's recipe body, keyed by target name.

    A recipe is the run of tab-indented lines following a `target:` line.
    Enough of make's syntax for this file, which uses no pattern rules, no
    multi-target rules and no conditionals around recipes.
    """
    recipes: dict[str, str] = {}
    current: str | None = None
    for line in MAKEFILE.read_text(encoding="utf-8").splitlines():
        if line.startswith("\t"):
            if current is not None:
                recipes[current] += line + "\n"
            continue
        match = re.match(r"^([A-Za-z0-9_][A-Za-z0-9_.-]*)\s*:(?!=)", line)
        current = match.group(1) if match else None
        if current is not None:
            recipes.setdefault(current, "")
            # Prerequisites run before the recipe, so they are part of what the
            # target reaches. `target: dep1 dep2` puts them on the header line.
            _, _, prereqs = line.partition(":")
            recipes[current] += prereqs.replace("=", "") + "\n"
    return recipes


def reachable(roots: set[str], recipes: dict[str, str]) -> set[str]:
    """Every target reached from `roots`, following recipes and prerequisites.

    A prerequisite is named bare (`foo: bar`) while a recursive call is written
    `$(MAKE) bar`, and `make_targets` only sees the latter, so prerequisites are
    folded in from the header line by `makefile_recipes`.
    """
    seen: set[str] = set()
    pending = list(roots)
    while pending:
        target = pending.pop()
        if target in seen:
            continue
        seen.add(target)
        body = recipes.get(target)
        if body is None:
            continue
        pending.extend(make_targets(body) & recipes.keys())
        pending.extend(word for word in body.split() if word in recipes)
    return seen


def is_push_triggered(text: str) -> bool:
    """Does this workflow run on a push to main, or on a PR against it?

    Those are the runs a developer can turn red by pushing, so they are exactly
    the ones the hook is supposed to predict. Read as text rather than parsed:
    the alternative is a YAML dependency in a script whose whole job is to be
    runnable from a git hook on any checkout, and the two trigger keys are
    unambiguous at the top level of every workflow here.
    """
    header = text.split("\njobs:", 1)[0]
    return "\n  push:" in header or "\n  pull_request:" in header


def main() -> int:
    if not HOOK.is_file():
        print(f"missing {HOOK}", file=sys.stderr)
        return 2

    recipes = makefile_recipes()
    hook_roots = make_targets(HOOK.read_text(encoding="utf-8")) & recipes.keys()
    if not hook_roots:
        print(
            "the pre-push hook invokes no make target at all; either it was "
            "rewritten to call cargo directly (in which case this check needs "
            "updating) or it is not gating anything",
            file=sys.stderr,
        )
        return 1
    hook_targets = reachable(hook_roots, recipes)

    missing: list[Uncovered] = []
    for workflow in sorted(WORKFLOWS.glob("*.yml")):
        text = workflow.read_text(encoding="utf-8")
        # EVERY push-triggered workflow, not just the Rust one. Scoping this to
        # `batchalign-ci-rust` is what let a `ruff format --check` failure reach
        # main on 2026-08-14: the Python workflow was outside the scan, so its
        # gates were never compared against the hook at all. A workflow that
        # does not run on a push (release, the scheduled dependency audit) is
        # correctly out of scope, and that is now decided by its own triggers
        # rather than by which target it happens to name.
        if not is_push_triggered(text):
            continue
        # Intersected with the Makefile's own targets, so English prose in a
        # workflow comment ("make target", "make each ...") is not read as an
        # invocation. A workflow naming a target that does not exist fails in
        # CI on the first run and is not the silent drift this file is for.
        for target in sorted(make_targets(text) & recipes.keys()):
            if target in hook_targets or target in EXEMPT:
                continue
            missing.append(Uncovered(workflow=workflow.name, target=target))

    if missing:
        print("pre-push hook does not run what CI runs:", file=sys.stderr)
        for gap in missing:
            print(
                f"  {gap.workflow} runs 'make {gap.target}', the hook does not",
                file=sys.stderr,
            )
        print(
            "\nAdd it to scripts/pre-push.sh, or add it to EXEMPT in this file "
            "WITH a reason.",
            file=sys.stderr,
        )
        return 1

    covered = ", ".join(sorted(hook_targets))
    print(f"pre-push runs CI's targets ({covered})")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
