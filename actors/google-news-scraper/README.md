# Google News Scraper

Apify actor for Scrappa's Google News API.

Use `queries` to run multiple keyword searches in one Apify run. Legacy single-query `q` and token-based requests still work for topic, publication, section, story, and Knowledge Graph pages.

## Development

This is a standalone Rust 1.90 actor. The request-builder and storage tests use local HTTP mocks:

```bash
cargo test --locked
```

## Run locally

```bash
APIFY_TOKEN=... ACTOR_DEFAULT_KEY_VALUE_STORE_ID=... ACTOR_DEFAULT_DATASET_ID=... SCRAPPA_API_KEY=... cargo run --locked
```

The actor reads `INPUT` from the configured default key-value store. `APIFY_API_PUBLIC_BASE_URL` and `SCRAPPA_API_BASE_URL` can be overridden for local mocks.
