# Google Finance Intraday QA repair

Mimir task: `a7db9f64-fdfc-4266-8950-8497df85fdd5`
Actor: `thescrappa/google-finance-intraday-scraper` (`IeOWC1bvG8SUebyDa`).

## Failure and cause

Apify QA run [`efHjm0AGPchmgVJvn`](https://console.apify.com/view/runs/efHjm0AGPchmgVJvn) failed on September 4, 2026 after 27.365 seconds. Build `1.0.2` exhausted three retries on HTTP 503. Its input exactly matches the current prefill and defaults:

```json
{"symbols":[{"symbol":"AAPL","exchange":"NASDAQ"}],"hl":"en","gl":"us"}
```

Direct API checks also returned HTTP 503 for MSFT, GOOGL, and BTC-USD. A fresh AAPL page fetched through Scrappa's existing provider contained 391 minute-level points in `ds:11`; `ds:10` had become quote-summary data. Scrappa's parser read only `ds:10` and classified the page as unrecognized.

The upstream repair preserves the legacy store when it contains recognized points and otherwise tries `ds:11`: [Scrappa PR #5930](https://github.com/userlip/scrappa/pull/5930). The actor client now includes Scrappa's `error` response field in error messages rather than hiding the actual error behind `Service Unavailable`.

The prefill, batching, price-point billing, retry limits, 128 MB memory setting, and automatic testing remain unchanged. No synthetic results or success-on-503 behavior was introduced.

## Validation

- Upstream: 20 parser tests and 7 intraday controller tests passed; Pint passed.
- Full captured AAPL response: `valid_graph`, 391 points after the parser fix (zero before).
- Actor: 13 tests and TypeScript typecheck passed.
- Live build/run verification: pending upstream deployment.
