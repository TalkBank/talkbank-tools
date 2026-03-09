#!/bin/bash
# Pre-commit checks for talkbank-tools
#
# This script runs all critical checks before committing code changes.
# Run this before EVERY commit to avoid regressions.
#
# Usage: ./scripts/pre-commit-check.sh

set -e  # Exit on first error

echo ""
echo "═══════════════════════════════════════════════════════════"
echo "  Pre-Commit Checks"
echo "═══════════════════════════════════════════════════════════"
echo ""

# Color codes
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Track overall success
CHECKS_PASSED=0
CHECKS_FAILED=0

# Helper function to run a check
run_check() {
    local name="$1"
    local command="$2"

    echo -n "▶ $name... "

    if eval "$command" > /tmp/check_output.log 2>&1; then
        echo -e "${GREEN}✓${NC}"
        CHECKS_PASSED=$((CHECKS_PASSED + 1))
        return 0
    else
        echo -e "${RED}✗${NC}"
        echo ""
        echo -e "${RED}Failed: $name${NC}"
        echo "Output:"
        cat /tmp/check_output.log
        echo ""
        CHECKS_FAILED=$((CHECKS_FAILED + 1))
        return 1
    fi
}

# 1. Format check
run_check "Format check" "cargo fmt --check"

# 2. Clippy
run_check "Clippy lints" "cargo clippy --all-targets -- -D warnings"

# 3. Build
run_check "Build check" "cargo build --all-targets"

# 4. Unit tests
run_check "Unit tests" "cargo test --workspace --lib --quiet"

# 5. Reference corpus (CRITICAL)
echo ""
echo "▶ Reference corpus (CRITICAL - may take a moment)..."
REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
if cargo nextest run -p talkbank-parser-tests --test roundtrip_reference_corpus --quiet > /tmp/roundtrip_output.log 2>&1; then
    echo -e "${GREEN}✓ Reference corpus${NC}"
    CHECKS_PASSED=$((CHECKS_PASSED + 1))
else
    echo -e "${RED}✗ Reference corpus${NC}"
    echo ""
    echo -e "${RED}CRITICAL: Reference corpus MUST pass at 100%!${NC}"
    echo "Output:"
    cat /tmp/roundtrip_output.log
    echo ""
    CHECKS_FAILED=$((CHECKS_FAILED + 1))
fi

# Summary
echo ""
echo "═══════════════════════════════════════════════════════════"
echo "  Summary"
echo "═══════════════════════════════════════════════════════════"
echo ""

if [ $CHECKS_FAILED -eq 0 ]; then
    echo -e "${GREEN}✓ All $CHECKS_PASSED checks passed!${NC}"
    echo ""
    echo "Safe to commit."
    echo ""
    exit 0
else
    echo -e "${RED}✗ $CHECKS_FAILED check(s) failed!${NC}"
    echo -e "${GREEN}✓ $CHECKS_PASSED check(s) passed${NC}"
    echo ""
    echo -e "${YELLOW}Fix failing checks before committing.${NC}"
    echo ""
    exit 1
fi
