# Google Finance Historical Prices Scraper

Apify Actor wrapper for Scrappa's `/api/google-finance/historical` endpoint. The Rust actor sends one authenticated request, keeps every returned price point in response order, and writes one dataset item per point.

## Development

Run the focused actor tests and build the production image from this directory:

```bash
cargo test --locked
docker build -f .actor/Dockerfile -t google-finance-historical-prices-scraper .
```

The actor reads its input and writes `OUTPUT` through the run's default Apify key-value store, appends results to the default dataset, and charges the configured `price-point` event for saved rows when the run uses `PAY_PER_EVENT` pricing.

## Input and output

The input schema and prefill remain unchanged. Use a preset `range` for the most stable historical data. Custom `start_date` and `end_date` values must be provided together without `range`; a Scrappa `NOT_FOUND` response for that custom range produces an empty dataset and a not-found object in `OUTPUT`.

Each dataset item contains one normalized point with its timestamp and ISO date, close/change/volume fields, instrument metadata, and the request parameters. The full Scrappa response is written to the key-value store record `OUTPUT` after all points fit the run's charge budget.
