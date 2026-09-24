#!/bin/sh
set -eu

if command -v cargo >/dev/null 2>&1; then
    exec cargo "$@"
fi

rustup_cargo="$HOME/.cargo/bin/cargo"
if [ -x "$rustup_cargo" ]; then
    exec "$rustup_cargo" "$@"
fi

if command -v docker >/dev/null 2>&1 && docker info >/dev/null 2>&1; then
    exec docker run --rm -v "$PWD:/work" -w /work rust:1.90-slim-bookworm cargo "$@"
fi

if command -v curl >/dev/null 2>&1; then
    rustup_installer=$(mktemp)
    trap 'rm -f "$rustup_installer"' EXIT HUP INT TERM
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs -o "$rustup_installer"
    sh "$rustup_installer" -y --profile minimal --default-toolchain 1.90.0
    exec "$rustup_cargo" "$@"
fi

printf '%s\n' 'Rust commands require Cargo, an accessible Docker daemon, or curl to install Rust 1.90.' >&2
exit 127
