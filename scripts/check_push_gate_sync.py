#!/usr/bin/env python3
"""Fail if the pre-push hook does not run everything CI runs.

The hook and the workflow are two lists of what must pass, and two lists drift.
This one drifted far enough that the hook printed "All pre-push checks passed"
for three pushes CI then rejected on 2026-08-14: it ran `fmt`, an affected-only
compile check, and clippy solely behind an environment variable that defaulted
to off, while CI ran `make batchalign-ci-rust` (clippy, audits, check, tests,
doctests, integration, PyO3).

The cure is not a longer hook, it is the same command. This asserts that every
`make` target the Rust workflow invokes is also invoked by the hook, so a step
added to CI cannot silently stop being checked before a push.

Modelled on chatter's `check_ci_gate_sync.py`, which exists because that
repository had FOUR independent lists of what must pass and all four had
drifted.

Usage: python3 scripts/check_push_gate_sync.py
"""

from __future__ import annotations

import re
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parents[1]
HOOK = REPO / "scripts" / "pre-push.sh"
WORKFLOWS = REPO / ".github" / "workflows"

#: Targets a workflow may run that the hook is not expected to.
#:
#: Each needs a REASON, because an unexplained exemption is how the drift
#: starts again. Keep this list short; if it grows, the hook is the thing that
#: should change.
EXEMPT: dict[str, str] = {
    # Builds the dashboard's JS bundle. Needs `npm ci` against the network and
    # is not a correctness gate on committed Rust.
    "batchalign-dashboard-build": "network npm install, not a correctness gate",
    # Runs in its own workflow job with its own Linux toolchain setup; the hook
    # covers the Rust side that can fail from committed content.
    "batchalign-build-pyo3": "separate job with its own toolchain setup",
}


def make_targets(text: str) -> set[str]:
    """Every `make <target>` invoked in a shell snippet."""
    return {
        match.group(1)
        for match in re.finditer(r"\bmake\s+([a-z0-9][a-z0-9-]*)", text)
    }


def main() -> int:
    if not HOOK.is_file():
        print(f"missing {HOOK}", file=sys.stderr)
        return 2

    hook_targets = make_targets(HOOK.read_text(encoding="utf-8"))
    if not hook_targets:
        print(
            "the pre-push hook invokes no make target at all; either it was "
            "rewritten to call cargo directly (in which case this check needs "
            "updating) or it is not gating anything",
            file=sys.stderr,
        )
        return 1

    missing: list[tuple[str, str]] = []
    for workflow in sorted(WORKFLOWS.glob("*.yml")):
        text = workflow.read_text(encoding="utf-8")
        # Only the Rust gate; the book has its own hook line and other
        # workflows (release, audit) are not push-time correctness gates.
        if "batchalign-ci-rust" not in text:
            continue
        for target in sorted(make_targets(text)):
            if target in hook_targets or target in EXEMPT:
                continue
            missing.append((workflow.name, target))

    if missing:
        print("pre-push hook does not run what CI runs:", file=sys.stderr)
        for workflow, target in missing:
            print(f"  {workflow} runs 'make {target}', the hook does not", file=sys.stderr)
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
