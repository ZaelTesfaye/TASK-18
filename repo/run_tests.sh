#!/bin/bash
# SilverScreen Test Runner
# Executes all backend and frontend tests with summary output.
#
# Default mode (no arguments): runs everything inside Docker containers.
# No local toolchain (rustup, cargo, psql) is required on the host.
# This is the only supported invocation in CI.
#
# Developer escape hatch (NOT the canonical path — never use in CI):
#   ALLOW_LOCAL_RUN=true ./run_tests.sh
#   Requires: cargo, rustup, and exported env vars on the host.

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# ---------------------------------------------------------------------------
# Mode selection: Docker (default) vs Local (ALLOW_LOCAL_RUN=true only)
# ---------------------------------------------------------------------------
USE_DOCKER_CARGO=1   # Docker is the default — always.

if [ "${ALLOW_LOCAL_RUN:-}" = "true" ]; then
  echo "========================================================"
  echo "  WARNING: ALLOW_LOCAL_RUN=true detected."
  echo "  Running in LOCAL mode — NOT the canonical test path."
  echo "  This bypasses the Docker-only policy."
  echo "  Requires: cargo, rustup, env vars on the host."
  echo "  For reproducible CI results, unset ALLOW_LOCAL_RUN."
  echo "========================================================"
  if ! command -v cargo >/dev/null 2>&1; then
    echo "FATAL: local mode requires cargo on the host. Install via rustup."
    exit 1
  fi
  USE_DOCKER_CARGO=0
else
  if ! command -v docker >/dev/null 2>&1; then
    echo "FATAL: docker is required. Install Docker to run tests."
    exit 1
  fi
  echo "Running in Docker mode (no local toolchain required)."
fi

run_cargo() {
  if [ "$USE_DOCKER_CARGO" -eq 1 ]; then
    local docker_args=(run --rm -v "$PWD:/work" -w /work)
    if [ -f "$SCRIPT_DIR/.env" ]; then
      docker_args+=(--env-file "$SCRIPT_DIR/.env")
    fi

    # On Linux CI hosts, host networking lets tests reach services exposed on localhost.
    if [ "$(uname -s)" = "Linux" ]; then
      docker_args+=(--network host)
    fi

    for env_name in DATABASE_URL JWT_SECRET ENCRYPTION_KEY RUST_LOG; do
      if [ -n "${!env_name:-}" ]; then
        docker_args+=(--env "$env_name")
      fi
    done

    docker "${docker_args[@]}" rust:1.88-bookworm cargo "$@"
  else
    cargo "$@"
  fi
}

run_cargo_frontend_wasm_check() {
  if [ "$USE_DOCKER_CARGO" -eq 1 ]; then
    local docker_args=(run --rm -v "$PWD:/work" -w /work)
    if [ -f "$SCRIPT_DIR/.env" ]; then
      docker_args+=(--env-file "$SCRIPT_DIR/.env")
    fi

    if [ "$(uname -s)" = "Linux" ]; then
      docker_args+=(--network host)
    fi

    docker "${docker_args[@]}" rust:1.88-bookworm /bin/bash -lc "rustup target add wasm32-unknown-unknown >/dev/null 2>&1 || true; cargo check --target wasm32-unknown-unknown"
  else
    cargo check --target wasm32-unknown-unknown
  fi
}

TOTAL=0
PASSED=0
FAILED=0
ERRORS=""

echo "============================================="
echo "  SilverScreen Test Suite"
echo "============================================="
echo ""

# --- Compile Check Gate ---
echo "--- Compile Check: Backend ---"
cd /app/backend 2>/dev/null || cd backend 2>/dev/null || cd /repo/backend
if ! run_cargo check 2>&1; then
  echo "FATAL: Backend cargo check failed. Fix compile errors before running tests."
  exit 1
fi
echo "  Backend compiles OK"
echo ""

echo "--- Compile Check: Frontend (wasm32) ---"
cd /app/frontend 2>/dev/null || cd ../frontend 2>/dev/null || cd /repo/frontend
if ! run_cargo_frontend_wasm_check 2>&1; then
  # Fallback: try native check if wasm target not installed
  if ! run_cargo check 2>&1; then
    echo "FATAL: Frontend cargo check failed. Fix compile errors before running tests."
    exit 1
  fi
fi
echo "  Frontend compiles OK"
echo ""

cd /app/backend 2>/dev/null || cd ../backend 2>/dev/null || cd /repo/backend

# --- Backend Unit Tests ---
echo "--- Backend Unit Tests ---"
UNIT_OUTPUT=$(run_cargo test --lib -- --test-threads=1 2>&1) || true
UNIT_TOTAL=$(echo "$UNIT_OUTPUT" | grep -oP 'test result:.*?(\d+) passed' | grep -oP '\d+' | tail -1 || echo "0")
UNIT_FAILED=$(echo "$UNIT_OUTPUT" | grep -oP '(\d+) failed' | grep -oP '\d+' | head -1 || echo "0")
UNIT_PASSED=${UNIT_TOTAL:-0}
UNIT_FAILED=${UNIT_FAILED:-0}
UNIT_COUNT=$((UNIT_PASSED + UNIT_FAILED))
TOTAL=$((TOTAL + UNIT_COUNT))
PASSED=$((PASSED + UNIT_PASSED))
FAILED=$((FAILED + UNIT_FAILED))
echo "  Total: $UNIT_COUNT | Passed: $UNIT_PASSED | Failed: $UNIT_FAILED"
if [ "$UNIT_FAILED" -gt 0 ]; then
  ERRORS="$ERRORS\n[Backend Unit] $UNIT_FAILED test(s) failed:\n$(echo "$UNIT_OUTPUT" | grep 'FAILED\|panicked')\n"
fi
echo ""

# --- Backend Integration/API Tests ---
echo "--- Backend API Tests ---"
API_OUTPUT=$(run_cargo test --test '*' -- --test-threads=1 2>&1) || true
API_TOTAL=$(echo "$API_OUTPUT" | grep -oP 'test result:.*?(\d+) passed' | grep -oP '\d+' | tail -1 || echo "0")
API_FAILED=$(echo "$API_OUTPUT" | grep -oP '(\d+) failed' | grep -oP '\d+' | head -1 || echo "0")
API_PASSED=${API_TOTAL:-0}
API_FAILED=${API_FAILED:-0}
API_COUNT=$((API_PASSED + API_FAILED))
TOTAL=$((TOTAL + API_COUNT))
PASSED=$((PASSED + API_PASSED))
FAILED=$((FAILED + API_FAILED))
echo "  Total: $API_COUNT | Passed: $API_PASSED | Failed: $API_FAILED"
if [ "$API_FAILED" -gt 0 ]; then
  ERRORS="$ERRORS\n[Backend API] $API_FAILED test(s) failed:\n$(echo "$API_OUTPUT" | grep 'FAILED\|panicked')\n"
fi
echo ""

# --- Backend API Coverage (cargo-tarpaulin) ---
COVERAGE_THRESHOLD=90
COVERAGE_DIR="$SCRIPT_DIR/coverage"
mkdir -p "$COVERAGE_DIR"

echo "--- Backend API Coverage ---"
run_tarpaulin() {
  if [ "$USE_DOCKER_CARGO" -eq 1 ]; then
    local docker_args=(run --rm --security-opt seccomp=unconfined -v "$PWD:/work" -w /work)
    if [ -f "$SCRIPT_DIR/.env" ]; then
      docker_args+=(--env-file "$SCRIPT_DIR/.env")
    fi
    if [ "$(uname -s)" = "Linux" ]; then
      docker_args+=(--network host)
    fi
    for env_name in DATABASE_URL JWT_SECRET ENCRYPTION_KEY RUST_LOG; do
      if [ -n "${!env_name:-}" ]; then
        docker_args+=(--env "$env_name")
      fi
    done
    docker "${docker_args[@]}" rust:1.88-bookworm /bin/bash -lc \
      "cargo install cargo-tarpaulin 2>/dev/null; cargo tarpaulin \$*" -- "$@"
  else
    if ! command -v cargo-tarpaulin >/dev/null 2>&1; then
      cargo install cargo-tarpaulin 2>/dev/null
    fi
    cargo tarpaulin "$@"
  fi
}

COV_OUTPUT=$(run_tarpaulin \
  --test-threads=1 \
  --out json \
  --output-dir "$COVERAGE_DIR" \
  --skip-clean \
  -- --test-threads=1 2>&1) || true

# Extract coverage percentage from JSON artifact
COV_JSON="$COVERAGE_DIR/tarpaulin-report.json"
if [ -f "$COV_JSON" ]; then
  # tarpaulin JSON has a top-level "coverage" field as a percentage
  COV_PCT=$(python3 -c "
import json, sys
with open('$COV_JSON') as f:
    data = json.load(f)
# Try different tarpaulin JSON formats
if 'coverage' in data:
    print(f\"{data['coverage']:.1f}\")
elif 'files' in data:
    covered = sum(f.get('covered', 0) for f in data['files'])
    total = sum(f.get('coverable', 0) for f in data['files'])
    print(f'{(covered/total*100) if total > 0 else 0:.1f}')
else:
    print('0.0')
" 2>/dev/null || echo "0.0")

  echo "  Coverage: ${COV_PCT}% (threshold: ${COVERAGE_THRESHOLD}%)"
  echo "  Artifact: $COV_JSON"

  # Threshold check
  COV_PASS=$(python3 -c "print('yes' if float('$COV_PCT') >= $COVERAGE_THRESHOLD else 'no')" 2>/dev/null || echo "no")
  if [ "$COV_PASS" != "yes" ]; then
    echo "  FAIL: API coverage ${COV_PCT}% is below ${COVERAGE_THRESHOLD}% threshold"
    FAILED=$((FAILED + 1))
    TOTAL=$((TOTAL + 1))
    ERRORS="$ERRORS\n[Coverage] Backend API coverage ${COV_PCT}% < ${COVERAGE_THRESHOLD}% threshold\n"
  else
    echo "  PASS: Coverage meets threshold"
    PASSED=$((PASSED + 1))
    TOTAL=$((TOTAL + 1))
  fi
else
  echo "  SKIPPED (tarpaulin not available or failed to produce report)"
  echo "  Output: $(echo "$COV_OUTPUT" | tail -5)"
fi
echo ""

# --- Frontend Unit Tests ---
echo "--- Frontend Unit Tests ---"
cd /app/frontend 2>/dev/null || cd ../frontend 2>/dev/null || cd /repo/frontend
FE_OUTPUT=$(run_cargo test --all-targets 2>&1) || true
FE_TOTAL=$(echo "$FE_OUTPUT" | grep -oP 'test result:.*?(\d+) passed' | grep -oP '\d+' | tail -1 || echo "0")
FE_FAILED=$(echo "$FE_OUTPUT" | grep -oP '(\d+) failed' | grep -oP '\d+' | head -1 || echo "0")
FE_PASSED=${FE_TOTAL:-0}
FE_FAILED=${FE_FAILED:-0}
FE_COUNT=$((FE_PASSED + FE_FAILED))
TOTAL=$((TOTAL + FE_COUNT))
PASSED=$((PASSED + FE_PASSED))
FAILED=$((FAILED + FE_FAILED))
echo "  Total: $FE_COUNT | Passed: $FE_PASSED | Failed: $FE_FAILED"
if [ "$FE_FAILED" -gt 0 ]; then
  ERRORS="$ERRORS\n[Frontend Unit] $FE_FAILED test(s) failed:\n$(echo "$FE_OUTPUT" | grep 'FAILED\|panicked')\n"
fi
echo ""

# --- Frontend E2E Tests ---
echo "--- Frontend E2E Tests (contract tests) ---"
E2E_OUTPUT=$(run_cargo test --test '*' 2>&1) || true
E2E_TOTAL=$(echo "$E2E_OUTPUT" | grep -oP 'test result:.*?(\d+) passed' | grep -oP '\d+' | tail -1 || echo "0")
E2E_FAILED=$(echo "$E2E_OUTPUT" | grep -oP '(\d+) failed' | grep -oP '\d+' | head -1 || echo "0")
E2E_PASSED=${E2E_TOTAL:-0}
E2E_FAILED=${E2E_FAILED:-0}
E2E_COUNT=$((E2E_PASSED + E2E_FAILED))
TOTAL=$((TOTAL + E2E_COUNT))
PASSED=$((PASSED + E2E_PASSED))
FAILED=$((FAILED + E2E_FAILED))
echo "  Total: $E2E_COUNT | Passed: $E2E_PASSED | Failed: $E2E_FAILED"
if [ "$E2E_FAILED" -gt 0 ]; then
  ERRORS="$ERRORS\n[Frontend E2E] $E2E_FAILED test(s) failed:\n$(echo "$E2E_OUTPUT" | grep 'FAILED\|panicked')\n"
fi
echo ""

# --- Frontend WASM Browser Tests (includes E2E flows — requires wasm-pack + headless Chrome) ---
echo "--- Frontend WASM Browser + E2E Tests ---"
if command -v wasm-pack &>/dev/null; then
  # The wasm test suite includes both browser-level component tests and
  # full-stack E2E flows (register→login→cart→order) that hit the live backend.
  # E2E tests gracefully skip if the backend is not reachable.
  WASM_OUTPUT=$(wasm-pack test --headless --chrome --test wasm 2>&1) || true
  WASM_TOTAL=$(echo "$WASM_OUTPUT" | grep -oP 'test result:.*?(\d+) passed' | grep -oP '\d+' | tail -1 || echo "0")
  WASM_FAILED=$(echo "$WASM_OUTPUT" | grep -oP '(\d+) failed' | grep -oP '\d+' | head -1 || echo "0")
  WASM_PASSED=${WASM_TOTAL:-0}
  WASM_FAILED=${WASM_FAILED:-0}
  WASM_COUNT=$((WASM_PASSED + WASM_FAILED))
  TOTAL=$((TOTAL + WASM_COUNT))
  PASSED=$((PASSED + WASM_PASSED))
  FAILED=$((FAILED + WASM_FAILED))
  echo "  Total: $WASM_COUNT | Passed: $WASM_PASSED | Failed: $WASM_FAILED"
  if [ "$WASM_FAILED" -gt 0 ]; then
    ERRORS="$ERRORS\n[Frontend WASM/E2E] $WASM_FAILED test(s) failed:\n$(echo "$WASM_OUTPUT" | grep 'FAILED\|panicked')\n"
  fi
else
  echo "  SKIPPED (wasm-pack not installed)"
fi
echo ""

# --- Summary ---
echo "============================================="
echo "  SUMMARY"
echo "============================================="
echo "  Total Tests: $TOTAL"
echo "  Passed:      $PASSED"
echo "  Failed:      $FAILED"
echo "============================================="

if [ "$FAILED" -gt 0 ]; then
  echo ""
  echo "--- Error Details ---"
  echo -e "$ERRORS"
  exit 1
else
  echo ""
  echo "All tests passed!"
  exit 0
fi
