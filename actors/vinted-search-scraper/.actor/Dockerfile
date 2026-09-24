FROM rust:1.90-slim-bookworm AS builder

WORKDIR /build
COPY Cargo.toml Cargo.lock ./
RUN mkdir src && printf 'fn main() {}\n' > src/main.rs \
    && cargo build --release --locked \
    && rm src/main.rs
COPY src ./src
COPY .actor/input_schema.json ./.actor/input_schema.json
RUN cargo clean --release -p vinted-search-scraper \
    && cargo build --release --locked

FROM debian:bookworm-slim
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/* \
    && useradd --create-home --uid 1000 myuser

WORKDIR /home/myuser
COPY --from=builder --chown=myuser:myuser /build/target/release/vinted-search-scraper /usr/local/bin/actor
USER myuser
CMD ["/usr/local/bin/actor"]
