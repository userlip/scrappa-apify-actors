FROM rust:1.90-slim-bookworm AS builder
WORKDIR /app
COPY Cargo.toml Cargo.lock ./
RUN mkdir src && printf 'fn main() {}\n' > src/main.rs && cargo build --release --locked && rm src/main.rs
COPY src ./src
RUN cargo clean --release -p instagram-user-posts-cheapest-0-20-1000-results \
    && cargo build --release --locked

FROM debian:bookworm-slim AS runtime
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/instagram-user-posts-cheapest-0-20-1000-results /usr/local/bin/instagram-user-posts-cheapest-0-20-1000-results
USER 1000:1000
ENTRYPOINT ["/usr/local/bin/instagram-user-posts-cheapest-0-20-1000-results"]
