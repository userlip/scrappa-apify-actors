#!/bin/sh
set -eu

if command -v cargo >/dev/null 2>&1; then
    cargo test --locked
    exit
fi

if command -v docker >/dev/null 2>&1 && docker info >/dev/null 2>&1; then
    exec docker run --rm \
        --user "$(id -u):$(id -g)" \
        -v "$PWD:/workspace" \
        -w /workspace \
        rust:1.90-slim-bookworm cargo test --locked
fi

case "$(uname -s)-$(uname -m)" in
    Linux-x86_64) rust_host=x86_64-unknown-linux-gnu ;;
    Linux-aarch64|Linux-arm64) rust_host=aarch64-unknown-linux-gnu ;;
    Darwin-arm64) rust_host=aarch64-apple-darwin ;;
    Darwin-x86_64) rust_host=x86_64-apple-darwin ;;
    *)
        echo "Cannot install the Rust test toolchain for $(uname -s)-$(uname -m)." >&2
        exit 1
        ;;
esac

rustup_dir="${RUNNER_TEMP:-/tmp}/instagram-post-info-rust-toolchain"
mkdir -p "$rustup_dir"
curl --fail --silent --show-error --location \
    "https://static.rust-lang.org/rustup/dist/$rust_host/rustup-init" \
    --output "$rustup_dir/rustup-init"
chmod +x "$rustup_dir/rustup-init"

export CARGO_HOME="$rustup_dir/cargo"
export RUSTUP_HOME="$rustup_dir/rustup"
export PATH="$CARGO_HOME/bin:$PATH"
"$rustup_dir/rustup-init" -y --default-toolchain 1.90.0 --profile minimal --no-modify-path
cargo test --locked
