FROM rust:1.90-slim-bookworm AS builder
WORKDIR /app
COPY Cargo.toml Cargo.lock ./
RUN mkdir src && printf 'fn main() {}\n' > src/main.rs && cargo build --release --locked
COPY src ./src
RUN cargo clean --release -p google-maps-search-scraper && cargo build --release --locked

FROM debian:bookworm-slim AS runtime
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/google-maps-search-scraper /usr/local/bin/google-maps-search-scraper
USER 1000:1000
ENTRYPOINT ["/usr/local/bin/google-maps-search-scraper"]
