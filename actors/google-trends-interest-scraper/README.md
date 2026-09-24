# Google Trends Interest Scraper

Apify actor for Scrappa's Google Trends interest-over-time API.

## Development

This is a standalone Rust 1.90 actor. Focused request, output, retry, and PPE tests run with:

```bash
cargo test --locked
```

Build the production image from this directory with:

```bash
docker build -f .actor/Dockerfile -t google-trends-interest-scraper .
```

To run the binary against an Apify run, provide `APIFY_TOKEN`, `ACTOR_RUN_ID`, `ACTOR_DEFAULT_KEY_VALUE_STORE_ID`, `ACTOR_DEFAULT_DATASET_ID`, and `SCRAPPA_API_KEY`, then run `cargo run --locked`.

The actor reads `INPUT` and writes dataset items and `OUTPUT` through the Apify API. Set `APIFY_API_PUBLIC_BASE_URL` and `SCRAPPA_API_BASE_URL` to local HTTP mocks when smoke-testing the image. The interest endpoint returns the full timeline in one response; the actor has no page or continuation-token input.
