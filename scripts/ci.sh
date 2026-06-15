#!/usr/bin/env bash
# Wind Tunnel CI script.
# Runs the full test suite (Rust + Python validation).
# Exit code 0 = all pass. Non-zero = failure.

set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

echo "=== cargo fmt (check) ==="
cargo fmt --all -- --check

echo "=== cargo clippy ==="
cargo clippy --workspace --all-targets -- -D warnings

echo "=== cargo test ==="
cargo test --workspace

echo "=== python validation ==="
python src/validation.py

echo ""
echo "All checks passed."
