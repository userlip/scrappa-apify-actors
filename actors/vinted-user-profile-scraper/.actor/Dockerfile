FROM rust:1.90-slim-bookworm AS builder
WORKDIR /build
COPY Cargo.toml Cargo.lock ./
COPY src ./src
RUN cargo build --release --locked

FROM debian:bookworm-slim
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*
COPY --from=builder /build/target/release/vinted-user-profile-scraper /usr/local/bin/actor
USER 1000:1000
ENTRYPOINT ["/usr/local/bin/actor"]
