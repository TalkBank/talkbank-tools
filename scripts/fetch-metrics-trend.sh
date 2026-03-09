#!/usr/bin/env bash
# scripts/fetch-metrics-trend.sh
#
# Fetches metrics.json artifacts from recent CI runs and renders a trend table.
#
# Prerequisites:
#   - gh CLI installed and authenticated to this repo
#   - jq installed
#
# Usage:
#   ./scripts/fetch-metrics-trend.sh           # last 10 completed runs
#   ./scripts/fetch-metrics-trend.sh 20        # last 20 completed runs
#   ./scripts/fetch-metrics-trend.sh 10 main   # last 10 runs on main branch

set -euo pipefail

N="${1:-10}"
BRANCH="${2:-}"

TMPDIR_ROOT=$(mktemp -d)
trap 'rm -rf "$TMPDIR_ROOT"' EXIT

# ── Fetch run list ─────────────────────────────────────────────────────────────
BRANCH_FLAG=""
if [[ -n "$BRANCH" ]]; then
  BRANCH_FLAG="--branch $BRANCH"
fi

echo "Fetching last $N completed CI runs from GitHub..." >&2

# shellcheck disable=SC2086
mapfile -t RUNS < <(
  gh run list \
    --workflow=ci.yml \
    --limit "$N" \
    --status completed \
    $BRANCH_FLAG \
    --json databaseId,headSha,createdAt,conclusion \
  | jq -r '.[] | "\(.databaseId) \(.headSha[0:7]) \(.createdAt[0:10]) \(.conclusion)"'
)

if [[ ${#RUNS[@]} -eq 0 ]]; then
  echo "No completed runs found." >&2
  exit 0
fi

# ── Collect metrics per run ────────────────────────────────────────────────────
declare -a ROWS=()
for run in "${RUNS[@]}"; do
  read -r run_id sha date conclusion <<< "$run"
  artifact_dir="$TMPDIR_ROOT/$run_id"
  mkdir -p "$artifact_dir"

  if gh run download "$run_id" --name metrics --dir "$artifact_dir" 2>/dev/null; then
    metrics_file="$artifact_dir/metrics.json"
    if [[ -f "$metrics_file" ]]; then
      read -r errors constructs corpus <<< "$(
        jq -r '[
          .spec.errors_total,
          .spec.constructs_total,
          .corpus.reference_files
        ] | @tsv' "$metrics_file"
      )"
      ROWS+=("$sha $date $conclusion $errors $constructs $corpus")
    else
      ROWS+=("$sha $date $conclusion — — —")
    fi
  else
    ROWS+=("$sha $date $conclusion (no artifact) — —")
  fi
done

# ── Render trend table ─────────────────────────────────────────────────────────
echo ""
echo "| Commit  | Date       | Result    | Error Specs | Construct Specs | Corpus Files |"
echo "|---------|------------|-----------|-------------|-----------------|--------------|"
for row in "${ROWS[@]}"; do
  read -r sha date conclusion errors constructs corpus <<< "$row"
  icon="✅"
  [[ "$conclusion" == "failure" ]] && icon="❌"
  [[ "$conclusion" == "cancelled" ]] && icon="⚠️"
  printf "| %-7s | %-10s | %s %-8s | %-11s | %-15s | %-12s |\n" \
    "$sha" "$date" "$icon" "$conclusion" "$errors" "$constructs" "$corpus"
done
echo ""
echo "Run './scripts/metrics-snapshot.sh' locally to capture current state."
