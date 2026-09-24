#!/bin/sh
set -eu

cargo_target_dir="${TMPDIR:-/tmp}/google-jobs-scraper-target"
export CARGO_TARGET_DIR="$cargo_target_dir"

if command -v cargo >/dev/null 2>&1; then
    cargo test --locked
    exit 0
fi

if command -v docker >/dev/null 2>&1; then
    if docker info >/dev/null 2>&1; then
        docker run --rm -e CARGO_TARGET_DIR="$cargo_target_dir" -v "$PWD:/workspace" -w /workspace rust:1.90-slim-bookworm cargo test --locked
        exit 0
    fi
fi

if command -v rustup >/dev/null 2>&1; then
    rustup toolchain install 1.90.0 --profile minimal
    rustup run 1.90.0 cargo test --locked
    exit 0
fi

if ! command -v curl >/dev/null 2>&1; then
    echo "Rust tests require Cargo 1.90+, an accessible Docker daemon, or curl to install Rust." >&2
    exit 127
fi

rustup_root="${TMPDIR:-/tmp}/google-jobs-scraper-rustup"
cargo_root="${TMPDIR:-/tmp}/google-jobs-scraper-cargo"
export RUSTUP_HOME="$rustup_root"
export CARGO_HOME="$cargo_root"
mkdir -p "$rustup_root" "$cargo_root"

if [ -x "$cargo_root/bin/cargo" ]; then
    "$cargo_root/bin/cargo" test --locked
    exit 0
fi

rustup_installer=$(mktemp "${TMPDIR:-/tmp}/google-jobs-rustup-init.XXXXXX")
trap 'rm -f "$rustup_installer"' EXIT
curl --fail --location --silent --show-error https://sh.rustup.rs --output "$rustup_installer"
sh "$rustup_installer" -y --profile minimal --default-toolchain 1.90.0 --no-modify-path
"$cargo_root/bin/cargo" test --locked
