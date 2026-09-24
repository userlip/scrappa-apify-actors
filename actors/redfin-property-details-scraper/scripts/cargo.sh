#!/bin/sh
set -eu

rust_version=1.90.0
CARGO_HOME="${CARGO_HOME:-$HOME/.cargo}"
export CARGO_HOME
PATH="$CARGO_HOME/bin:$PATH"
export PATH

if [ "${GITHUB_ACTIONS:-}" != "true" ] \
    && command -v docker >/dev/null 2>&1 \
    && docker info >/dev/null 2>&1; then
    exec docker run --rm -v "$PWD:/build" -w /build rust:1.90-slim-bookworm cargo "$@"
fi

if command -v rustup >/dev/null 2>&1; then
    rustup toolchain install "$rust_version" --profile minimal
    exec cargo "+$rust_version" "$@"
fi

if command -v cargo >/dev/null 2>&1 \
    && command -v rustc >/dev/null 2>&1 \
    && rustc --version | grep -Fq " $rust_version "; then
    exec cargo "$@"
fi

rustup_installer="${TMPDIR:-/tmp}/redfin-rustup-init.sh"
curl --proto '=https' --tlsv1.2 --silent --show-error --fail \
    https://sh.rustup.rs \
    --output "$rustup_installer"
sh "$rustup_installer" -y --no-modify-path --default-toolchain none --profile minimal
rm -f "$rustup_installer"

rustup toolchain install "$rust_version" --profile minimal
exec cargo "+$rust_version" "$@"
