FROM rust:1.90-slim-bookworm AS builder
WORKDIR /app
COPY Cargo.toml Cargo.lock ./
RUN mkdir src && printf 'fn main() {}\n' > src/main.rs && cargo build --release --locked && rm src/main.rs
COPY src ./src
RUN cargo clean --release -p tiktok-comments-scraper && cargo build --release --locked

FROM builder AS tests
RUN rustup component add rustfmt && cargo fmt --check && cargo test --locked

FROM debian:bookworm-slim AS runtime
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/tiktok-comments-scraper /usr/local/bin/tiktok-comments-scraper
USER 1000:1000
ENTRYPOINT ["/usr/local/bin/tiktok-comments-scraper"]
