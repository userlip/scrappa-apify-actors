FROM rust:1.90-slim-bookworm AS builder
WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY src ./src
RUN cargo build --release --locked

FROM debian:bookworm-slim AS runtime
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/youtube-api-batch-videos /usr/local/bin/youtube-api-batch-videos
USER 1000:1000
ENTRYPOINT ["/usr/local/bin/youtube-api-batch-videos"]
