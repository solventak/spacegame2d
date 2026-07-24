#!/usr/bin/env bash
#
# Headless QA for spacegame2d — exercises the full simulation game loop,
# flight control, autopilot, and fleet behavior without a GPU or display.
#
# This is the primary QA path for CI and autonomous agents. It runs the
# complete test gate (format check + lint + unit/integration tests) and a
# server smoke test, then reports a pass/fail summary.
#
# Usage:
#   ./scripts/qa-headless.sh
#
# Exit codes:
#   0  all checks passed
#   1  one or more checks failed

set -euo pipefail

# Color output when stdout is a terminal.
if [ -t 1 ]; then
    GREEN='\033[0;32m'
    RED='\033[0;31m'
    YELLOW='\033[0;33m'
    NC='\033[0m'
else
    GREEN=''
    RED=''
    YELLOW=''
    NC=''
fi

pass() {
    echo -e "${GREEN}[PASS]${NC} $1"
}

fail() {
    echo -e "${RED}[FAIL]${NC} $1"
}

info() {
    echo -e "${YELLOW}[..]${NC} $1"
}

FAILED=0

run_check() {
    local label="$1"
    shift
    info "$label"
    if "$@"; then
        pass "$label"
    else
        fail "$label"
        FAILED=1
    fi
}

echo "============================================"
echo "  spacegame2d — Headless QA"
echo "============================================"
echo ""

# 1. Format check
run_check "cargo fmt --check" cargo fmt --check

# 2. Lint (warnings denied)
run_check "cargo clippy -- -D warnings" cargo clippy -- -D warnings

# 3. Test suite — exercises the full simulation game loop headlessly:
#    ship movement, autopilot navigation, reset, world boundary, drone fleet.
run_check "cargo test" cargo test

# 4. Server smoke test — the server stub should print its banner and exit 0.
info "server smoke test (spacegame2d-server)"
SERVER_OUTPUT=$(cargo run -p spacegame2d-server 2>/dev/null || true)
if echo "$SERVER_OUTPUT" | grep -q "spacegame2d-server: placeholder startup banner"; then
    pass "server smoke test"
else
    fail "server smoke test (expected banner, got: $SERVER_OUTPUT)"
    FAILED=1
fi

echo ""
echo "============================================"
if [ "$FAILED" -eq 0 ]; then
    echo -e "  ${GREEN}All QA checks passed.${NC}"
else
    echo -e "  ${RED}Some QA checks failed.${NC}"
fi
echo "============================================"

exit "$FAILED"
