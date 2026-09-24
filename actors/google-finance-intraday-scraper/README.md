# Google Finance Intraday Scraper

Apify actor for Scrappa's Google Finance intraday API. It accepts a batch of ticker symbols and writes one dataset item per returned graph point. The run summary is stored in the default key-value store record `OUTPUT`.

## Input

```json
{
  "symbols": [
    { "symbol": "AAPL", "exchange": "NASDAQ" },
    { "symbol": "MSFT", "exchange": "NASDAQ" }
  ],
  "hl": "en",
  "gl": "us"
}
```

`exchange` is optional and recommended when a symbol could match multiple markets. `hl` and `gl` are optional language and country codes. The actor schema retains the Apify prefill and defaults.

## Output

The dataset contains one row per intraday graph point, including its original fields, normalized numeric values, an ISO timestamp, and request metadata. The `OUTPUT` record summarizes requested symbols, successful symbols, symbols without data, and graph points.

In pay-per-event mode, each saved point is charged as `intraday-price-point`. A run stops when its spending limit prevents saving all points for a symbol.

## Development

Run the focused Rust tests with:

```bash
cargo test --locked
```

Build the production image from this directory with:

```bash
docker build -f .actor/Dockerfile -t google-finance-intraday-scraper .
```

The actor uses `SCRAPPA_API_KEY` for Scrappa authentication and the standard Apify run environment variables for input and storage access.
