# Google Jobs Scraper

Search Google Jobs listings through the Scrappa Google Jobs API. The actor forwards the supplied search and pagination parameters, retries transient Scrappa failures, and uses the Scrappa Indeed endpoint as a fallback for a failed first-page Google Jobs request.

## Input

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `q` | string | Conditionally | Job search query. Required unless `next_page_token` is provided. Empty input uses the prefilled `nurse jobs in Austin` search. |
| `next_page_token` | string | Conditionally | Token from a previous Google Jobs response. |
| `gl` | string | No | Two-letter country code, for example `us`, `uk`, or `de`. |
| `hl` | string | No | Two-letter language code, for example `en`, `de`, or `es`. |
| `google_domain` | string | No | Google domain to query, for example `google.com` or `google.de`. |
| `uule` | string | No | Google-encoded location parameter for precise geolocation. |
| `lrad` | integer | No | Search radius in miles. Requires `uule`. |
| `uds` | string | No | Dynamic filter string returned in a previous response. |

## Output

Each job from the response is written as a separate dataset item. The full upstream response is saved under `OUTPUT` in the default key-value store, including filters and pagination tokens.

The actor preserves pay-per-event charging for default dataset items and checks the run’s spending limit before writing results. If only part of the response fits the remaining budget, it saves the affordable rows while retaining the full response under `OUTPUT`.

## Local development

The production actor is Rust. Run its focused tests with `cargo test --locked`. The actor test workflow currently calls `npm test` for this path, so the actor-local npm script delegates to the same Rust tests and uses the Rust 1.90 Docker image when Cargo is not installed.

Build and smoke the production image against local mock services:

```sh
docker build -t google-jobs-scraper:local -f .actor/Dockerfile .
python3 test/local_image_smoke.py google-jobs-scraper:local
```
