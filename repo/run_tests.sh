#!/bin/bash
# SilverScreen Test Runner
# Executes all backend and frontend tests with summary output
#
# Native host usage (no Docker):
#   1. Ensure PostgreSQL is running and DATABASE_URL is exported.
#   2. Apply migration:  psql -d silverscreen -f backend/migrations/001_initial.sql
#   3. Export env:        source .env   (or export DATABASE_URL, JWT_SECRET, ENCRYPTION_KEY)
#   4. Backend tests:    cd backend && cargo test --lib -- --test-threads=1
#                        cargo test --test '*' -- --test-threads=1
#   5. Frontend tests:   cd frontend && cargo test --all-targets && cargo test --test '*'

set -e

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
if ! cargo check 2>&1; then
  echo "FATAL: Backend cargo check failed. Fix compile errors before running tests."
  exit 1
fi
echo "  Backend compiles OK"
echo ""

echo "--- Compile Check: Frontend (wasm32) ---"
cd /app/frontend 2>/dev/null || cd ../frontend 2>/dev/null || cd /repo/frontend
if ! cargo check --target wasm32-unknown-unknown 2>&1; then
  # Fallback: try native check if wasm target not installed
  if ! cargo check 2>&1; then
    echo "FATAL: Frontend cargo check failed. Fix compile errors before running tests."
    exit 1
  fi
fi
echo "  Frontend compiles OK"
echo ""

cd /app/backend 2>/dev/null || cd ../backend 2>/dev/null || cd /repo/backend

# --- Backend Unit Tests ---
echo "--- Backend Unit Tests ---"
UNIT_OUTPUT=$(cargo test --lib -- --test-threads=1 2>&1) || true
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
API_OUTPUT=$(cargo test --test '*' -- --test-threads=1 2>&1) || true
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

# --- Frontend Unit Tests ---
echo "--- Frontend Unit Tests ---"
cd /app/frontend 2>/dev/null || cd ../frontend 2>/dev/null || cd /repo/frontend
FE_OUTPUT=$(cargo test --all-targets 2>&1) || true
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
E2E_OUTPUT=$(cargo test --test '*' 2>&1) || true
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

# --- Frontend WASM Browser Tests (optional — requires wasm-pack + headless Chrome) ---
echo "--- Frontend WASM Browser Tests ---"
if command -v wasm-pack &>/dev/null; then
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
    ERRORS="$ERRORS\n[Frontend WASM] $WASM_FAILED test(s) failed:\n$(echo "$WASM_OUTPUT" | grep 'FAILED\|panicked')\n"
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
