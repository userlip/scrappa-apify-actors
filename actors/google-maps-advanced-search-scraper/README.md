# Google Maps Advanced Search Actor

This actor calls Scrappa's `GET /api/maps/advance-search` endpoint and stores each returned business in the default dataset. It keeps the raw API response in the default key-value store under `OUTPUT`.

The existing input schema is in `.actor/input_schema.json`. The endpoint's default page is `0`; the actor forwards `limit` when provided and leaves page selection to the API's existing default.

## Development

This is a standalone Rust 1.90 actor. Focused request, storage, pricing, and input-schema tests use local HTTP mocks:

```bash
cargo test --locked
```

Build the actor image from this directory:

```bash
docker build -f .actor/Dockerfile -t google-maps-advanced-search-scraper:local .
```
