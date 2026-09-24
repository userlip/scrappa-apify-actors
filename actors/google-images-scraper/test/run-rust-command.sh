#!/bin/sh
set -eu

if command -v cargo >/dev/null 2>&1; then
    exec cargo "$@"
fi

if command -v docker >/dev/null 2>&1; then
    exec docker run --rm -v "$PWD:/work" -w /work rust:1.90-slim-bookworm cargo "$@"
fi

printf '%s\n' 'Rust tests require Cargo or Docker with the Rust 1.90 image.' >&2
exit 127
