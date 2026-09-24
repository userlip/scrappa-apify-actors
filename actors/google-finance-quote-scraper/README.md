# Google Finance Quote Scraper

Fetch a Google Finance quote through Scrappa's `/api/google-finance/quote` endpoint. The actor returns one structured dataset item and stores the full Scrappa response as key-value record `OUTPUT`.

## What you get

- Quote summary, including price, currency, change, and market status
- Key statistics and company profile data
- Financial statements, news, and related tickers
- The complete upstream response in the default key-value store's `OUTPUT` record

## Input

```json
{
  "symbol": "AAPL",
  "exchange": "NASDAQ",
  "period_type": "quarterly",
  "hl": "en",
  "gl": "us"
}
```

`symbol` is required. `exchange` is optional and recommended when a symbol is ambiguous. `period_type` accepts `quarterly` or `annual`; `hl` accepts a language code such as `en` or `zh-cn`; `gl` accepts a two-letter country code.

The actor keeps the input schema's `AAPL` and `NASDAQ` prefill values and its `quarterly`, `en`, and `us` defaults.

## Output

The default dataset receives one item for a usable quote. It contains the flattened quote summary, nested statistics and profile, financials, news, related tickers, pagination metadata when supplied, request fields, and result counts. The full upstream response, including any pagination fields, is stored unchanged in `OUTPUT`.

The `quote-result` PAY_PER_EVENT event is charged only after a usable quote is available. The actor checks the run's remaining charge budget for both the custom result event and the default dataset item before saving. Empty responses, upstream 5xx errors after retries, timeouts, and a depleted charge budget do not write `OUTPUT`.

Scrappa requests use `X-API-Key`, time out after 25 seconds per attempt, and retry timeouts and HTTP 408, 429, 500, 502, 503, or 504 up to three attempts with exponential backoff and jitter. If a quote request with `period_type` still returns a 5xx response, the actor retries without `period_type`.

## Development

Run the focused Rust tests with Docker:

```bash
docker run --rm -v "$PWD":/app -w /app rust:1.90-slim-bookworm cargo test --locked
```

Build and smoke-test the production image against local mock Apify and Scrappa endpoints:

```bash
docker build -f .actor/Dockerfile -t google-finance-quote-scraper:local .
node test/image-smoke.mjs google-finance-quote-scraper:local
```
