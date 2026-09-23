FROM rust:1.90-slim-bookworm AS builder
WORKDIR /app
COPY Cargo.toml Cargo.lock ./
RUN mkdir src && printf 'fn main() {}\n' > src/main.rs && cargo build --release --locked && rm src/main.rs
COPY src ./src
# Cleaning the warm-up package prevents Cargo from shipping its no-op binary instead of src.
RUN cargo clean --release -p youtube-api-channel-podcasts && cargo build --release --locked

FROM debian:bookworm-slim AS runtime
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/youtube-api-channel-podcasts /usr/local/bin/youtube-api-channel-podcasts
USER 1000:1000
ENTRYPOINT ["/usr/local/bin/youtube-api-channel-podcasts"]
