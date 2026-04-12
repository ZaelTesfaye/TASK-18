#!/bin/bash
# CI pipeline step: Frontend compile gate
# Runs cargo check against the wasm32 target to catch type mismatches
# before they reach the test phase.
#
# Usage: ./ci/check_frontend.sh

set -e

echo "=========================================="
echo "  Frontend Compile Gate (wasm32)"
echo "=========================================="

cd frontend 2>/dev/null || cd /repo/frontend 2>/dev/null || cd /app/frontend

# Ensure wasm32 target is installed
rustup target add wasm32-unknown-unknown 2>/dev/null || true

echo "Running cargo check --target wasm32-unknown-unknown ..."
if cargo check --target wasm32-unknown-unknown 2>&1; then
    echo ""
    echo "PASS: Frontend compiles for wasm32-unknown-unknown"
    exit 0
else
    echo ""
    echo "FAIL: Frontend compile errors detected!"
    echo "Fix all type and signature mismatches before merging."
    exit 1
fi
