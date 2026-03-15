#!/usr/bin/env bash

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
SUITE_DIR="$SCRIPT_DIR/stella_test_suite/stage1"

if [[ $# -ge 1 ]]; then
    BIN="$1"
else
    echo "Building..."
    cargo build --manifest-path "$REPO_ROOT/Cargo.toml" 2>&1
    BIN="$REPO_ROOT/target/debug/stella-typechecker"
fi

if [[ ! -x "$BIN" ]]; then
    echo "ERROR: binary not found or not executable: $BIN" >&2
    exit 1
fi

PASS=0
FAIL=0

run_test() {
    local file="$1"
    local expect="$2"   # "ok" or "err"
    local output exit_code

    output=$("$BIN" "$file" 2>&1) && exit_code=$? || exit_code=$?

    if [[ "$expect" == "ok" && "$exit_code" -eq 0 ]]; then
        PASS=$((PASS + 1))
    elif [[ "$expect" == "err" && "$exit_code" -ne 0 ]]; then
        PASS=$((PASS + 1))
    else
        FAIL=$((FAIL + 1))
        local rel="${file#$REPO_ROOT/}"
        if [[ "$expect" == "ok" ]]; then
            echo "FAIL [well-typed] $rel"
            echo "  expected: Type OK (exit 0)  got: exit $exit_code"
        else
            echo "FAIL [ill-typed] $rel"
            echo "  expected: type error (exit != 0)  got: exit $exit_code"
        fi
        if [[ -n "$output" ]]; then
            echo "  output: $output" | head -5
        fi
    fi
}

# Well-typed: expect exit 0
while IFS= read -r -d '' f; do
    run_test "$f" "ok"
done < <(find "$SUITE_DIR/well-typed" "$SUITE_DIR/extra" -name "*.stella" -path "*/well-typed/*" -print0 | sort -z)

# Ill-typed: expect non-zero exit
while IFS= read -r -d '' f; do
    run_test "$f" "err"
done < <(find "$SUITE_DIR/ill-typed" "$SUITE_DIR/extra" -name "*.stella" -path "*/ill-typed/*" -print0 | sort -z)

TOTAL=$((PASS + FAIL))
echo ""
echo "Results: $PASS/$TOTAL passed"

if [[ $FAIL -gt 0 ]]; then
    exit 1
fi
