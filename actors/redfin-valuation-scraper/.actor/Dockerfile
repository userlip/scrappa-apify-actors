FROM rust:1-bookworm AS builder

WORKDIR /build
COPY Cargo.toml Cargo.lock ./
COPY src ./src
RUN cargo build --release --locked

FROM debian:bookworm-slim

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/* \
    && useradd --create-home --uid 1000 myuser

WORKDIR /home/myuser/app
COPY --from=builder --chown=myuser:myuser /build/target/release/redfin-valuation-scraper /usr/local/bin/redfin-valuation-scraper
USER myuser

CMD ["/usr/local/bin/redfin-valuation-scraper"]
