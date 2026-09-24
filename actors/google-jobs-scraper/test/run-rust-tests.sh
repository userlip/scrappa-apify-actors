#!/bin/sh
set -eu

if command -v cargo >/dev/null 2>&1; then
    cargo test --locked
    exit 0
fi

if command -v docker >/dev/null 2>&1; then
    docker run --rm -v "$PWD:/workspace" -w /workspace rust:1.90-slim-bookworm cargo test --locked
    exit 0
fi

echo "Rust tests require Cargo 1.90+ or Docker." >&2
exit 127
