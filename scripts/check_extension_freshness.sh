#!/usr/bin/env bash
#
# Refuse to run the Python suite against a compiled extension older than the
# Rust it is built from.
#
# THE QUESTION THIS ASKS is "which extension will Python actually import, and is
# it current?" It asks Python rather than guessing a path, because there are two
# places the extension can come from and which one wins is a property of
# sys.path, not of anything stated:
#
#   1. `batchalign_core/batchalign_core.abi3.so` in the working tree, written by
#      `uv run maturin develop` (debug profile, per [tool.maturin] in
#      pyproject.toml).
#   2. The copy inside the wheel that `make batchalign-python-prepare` installs
#      into .venv (release profile).
#
# `batchalign_core/__init__.py` in the repo root shadows the installed package
# whenever anything runs from the repo root, and it star-imports whichever
# extension its `__path__` finds first: the in-tree one if it exists, the
# installed one otherwise. So "build the wheel, install it, run pytest" can test
# a wheel it did not import.
#
# That is not hypothetical. On 2026-08-21 a session's worth of PyO3 boundary
# changes were reported as verified by a green Python suite that had spent the
# whole time importing an extension built the previous afternoon. Behaviour was
# preserved, so it passed either way, which is precisely why nothing caught it.
#
# An earlier version of this script mtime-checked the in-tree path directly.
# That had the same defect one level up: it inspected a proxy for the artifact
# instead of the artifact, so it was blind when the in-tree file was absent and
# the INSTALLED one was ancient. Resolving the import is the whole point.

set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

# Which extension does an import from the repo root actually resolve to? This is
# the same shadowing situation pytest runs under.
if ! resolved=$(uv run --no-sync python -c \
    'import batchalign_core.batchalign_core as ext; print(ext.__file__)' 2>/dev/null); then
    cat >&2 <<'EOF'
ERROR: no compiled batchalign_core extension could be imported.

Build one, then re-run:

    uv run maturin develop          # fast, debug profile, inner loop
    make batchalign-python-prepare  # release wheel, what actually ships

EOF
    exit 1
fi

if [ ! -e "$resolved" ]; then
    echo "ERROR: Python resolved the extension to '$resolved', which does not exist." >&2
    exit 1
fi

# Newest input the extension is built from. Tracked files only, so an editor
# swapfile or a target/ artifact cannot make the tree look newer than it is.
# Scope is this workspace's own Rust plus the manifests: a dependency bump
# changes the extension too.
newest_source=""
newest_epoch=0
while IFS= read -r source; do
    [ -e "$source" ] || continue
    epoch=$(stat -f %m "$source")
    if [ "$epoch" -gt "$newest_epoch" ]; then
        newest_epoch=$epoch
        newest_source=$source
    fi
done < <(git ls-files 'crates/*.rs' 'crates/*/Cargo.toml' Cargo.toml Cargo.lock)

resolved_epoch=$(stat -f %m "$resolved")

if [ "$resolved_epoch" -ge "$newest_epoch" ]; then
    exit 0
fi

cat >&2 <<EOF
ERROR: the extension Python will import is older than the Rust it is built from.

  will import: $resolved
    built $(date -r "$resolved_epoch" '+%Y-%m-%d %H:%M:%S')

  newest input: $newest_source
    changed $(date -r "$newest_epoch" '+%Y-%m-%d %H:%M:%S')

Running the suite now would report a pass that says nothing about your changes.
Rebuild whichever extension you meant to test, then re-run:

    uv run maturin develop          # fast, debug profile, inner loop
    make batchalign-python-prepare  # release wheel, what actually ships

EOF
exit 1
