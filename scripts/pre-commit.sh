#!/bin/bash
set -e

echo "Running pre-commit hooks..."

echo "Checking formatting..."
cargo fmt --all -- --check

echo "Running clippy..."
cargo clippy --all-targets --all-features -- -D warnings

echo "All checks passed!"
