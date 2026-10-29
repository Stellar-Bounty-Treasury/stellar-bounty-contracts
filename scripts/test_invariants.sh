#!/bin/sh
set -e

echo "=== Running Soroban Smart Contract Formal Invariant Test Suite ==="
cargo test --lib test_invariant -- --nocapture
echo "All formal invariants verified successfully."
