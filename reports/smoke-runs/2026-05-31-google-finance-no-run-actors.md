# 2026-05-31 Google Finance No-Run Actor Smoke Sweep

Scope: smoke-run the four public paid Google Finance actors that were reported as `NO_RUNS` in the latest owner-filtered Apify health audit. Inputs were intentionally small and routed through Scrappa API endpoints.

Post-run validation: `APIFY_TOKEN=... node scripts/audit-apify-health.mjs --json` at `2026-05-31T07:42:55.060Z` reported zero target actors as `NO_RUNS`. `google-finance-markets-scraper`, `google-finance-quote-scraper`, and `google-finance-search-scraper` moved to `OK`; `google-finance-historical-prices-scraper` moved to `RECENT_FAILED_BUT_LATEST_OK` because the first custom-date smoke failed before the successful preset-range rerun.

| Actor | Actor ID | Pricing status | Secret check | Latest build | Smoke input | Run ID | Status | Default dataset items | Log evidence |
| --- | --- | --- | --- | --- | --- | --- | --- | ---: | --- |
| `google-finance-historical-prices-scraper` | `zqckCpYtvPvxb2oxl` | `PAY_PER_EVENT` | `SCRAPPA_API_KEY` present as secret on version `1.0` | `WSfvkon4YiGocXsuZ` (`SUCCEEDED`) | `symbol: AAPL`, `exchange: NASDAQ`, `range: 2`, `interval: daily`, `hl: en`, `gl: us` | `wTae9vz5f6OvJzQ5T` | `SUCCEEDED` | 70 | Log reported `Fetching Google Finance historical prices for AAPL:NASDAQ (range=2, interval=daily, hl=en, gl=us)`, `Google Finance historical prices scraping completed successfully`, and `prices: 70`. |
| `google-finance-markets-scraper` | `WvbWRqj67ve6fwwWZ` | `PAY_PER_EVENT` | `SCRAPPA_API_KEY` present as secret on version `1.0` | `bXQhzetk0mvtzNQCr` (`SUCCEEDED`) | `trend: currencies`, `hl: en`, `gl: us` | `hSjpB2r2uDT4rBvyu` | `SUCCEEDED` | 75 | Log reported `Fetching Google Finance markets data for trend=currencies (hl=en, gl=us)`, `Google Finance markets scraping completed successfully`, `market_rows: 69`, and `news_results: 6`. |
| `google-finance-quote-scraper` | `aE7VcbT6CIWBxob7U` | `PAY_PER_EVENT` | `SCRAPPA_API_KEY` present as secret on version `1.0` | `mmQdDWHOx0v9NwSlv` (`SUCCEEDED`) | `symbol: AAPL`, `exchange: NASDAQ`, `period_type: quarterly`, `hl: en`, `gl: us` | `t8FnjDfKf88dIucXz` | `SUCCEEDED` | 1 | Log reported `Fetching Google Finance quote for AAPL:NASDAQ (period_type=quarterly, hl=en, gl=us)` and `Google Finance quote scraping completed successfully`. |
| `google-finance-search-scraper` | `JiWVC7gsvVEfwCxKv` | `PAY_PER_EVENT` | `SCRAPPA_API_KEY` present as secret on version `1.0` | `9dwdloiP4geFqzvoK` (`SUCCEEDED`) | `q: AAPL`, `hl: en`, `gl: us` | `Ohs7Teox4kpyWWnSc` | `SUCCEEDED` | 5 | Log reported `Searching Google Finance for "AAPL" (hl=en, gl=us)`, `Saved 5 Google Finance search results`, and `Google Finance search scraping completed successfully`. |

Additional historical-prices evidence:

| Actor | Actor ID | Smoke input | Run ID | Status | Default dataset items | Follow-up |
| --- | --- | --- | --- | --- | ---: | --- |
| `google-finance-historical-prices-scraper` | `zqckCpYtvPvxb2oxl` | `symbol: AAPL`, `exchange: NASDAQ`, `start_date: 2024-01-02`, `end_date: 2024-01-05`, `interval: daily`, `hl: en`, `gl: us` | `OvEbp71ywGG28SM6Z` | `FAILED` | 0 | Mimir task `ed6383a3-6dcd-43f3-a82e-4b7d2574b2a5` opened to verify the Scrappa custom date-range contract and decide whether the actor should handle this as a clean no-data condition or adjust schema/docs. |

Notes:

- Monetization was checked before smoke runs through the Apify Actor detail API. All four actors had active paid `PAY_PER_EVENT` pricing; no pricing changes were made.
- Version `1.0` environment variables were checked before smoke runs. All four actors had `SCRAPPA_API_KEY` configured as an Apify secret.
- Latest builds were already `SUCCEEDED` before smoke runs and the successful smoke runs used those build IDs.
- The Scrappa MCP endpoint discovery confirmed `google-finance-historical` exists. Direct MCP calls reproduced the custom-date no-data response and confirmed preset `range` input returns data.
- No source code, pricing, deployment, or listing changes were made in this smoke sweep.
