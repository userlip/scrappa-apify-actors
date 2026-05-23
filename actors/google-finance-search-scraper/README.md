# Google Finance Search Scraper

Search Google Finance for ticker and finance instrument discovery before running quote or historical-price lookups. This actor is a thin Scrappa-powered wrapper around Scrappa's `/api/google-finance/search` endpoint: Apify validates input, calls Scrappa, and writes one dataset item per matched finance result.

## What you get

- Batch ticker, company, ETF, index, fund, currency, and instrument discovery
- Single-query compatibility with `q`
- Batch mode with `queries`, capped at 25 queries per Apify run
- One dataset item per Google Finance search result, including query metadata
- Raw result fields preserved in `raw_result` so variable finance result types are not lost

## Input

Single query:

```json
{
  "q": "AAPL",
  "hl": "en",
  "gl": "us"
}
```

Batch queries:

```json
{
  "queries": ["AAPL", "Tesla", "S&P 500"],
  "hl": "en",
  "gl": "us"
}
```

When `queries` is provided, it is used instead of `q`. The actor calls Scrappa once per query inside the same Apify run to keep run startup and storage overhead amortized.

## Output

The dataset contains one item per finance search result:

```json
{
  "query": "AAPL",
  "position": 1,
  "name": "Apple Inc",
  "symbol": "AAPL",
  "exchange": "NASDAQ",
  "stock": "AAPL:NASDAQ",
  "type": "Stock",
  "currency": "USD",
  "price": 176.85,
  "price_change": 2.34,
  "percent_change": 1.33,
  "link": "https://www.google.com/finance/quote/AAPL:NASDAQ",
  "google_finance_url": "https://www.google.com/finance/quote/AAPL:NASDAQ",
  "market": null,
  "request_hl": "en",
  "request_gl": "us",
  "raw_result": {
    "stock": "AAPL:NASDAQ",
    "name": "Apple Inc",
    "symbol": "AAPL",
    "exchange": "NASDAQ"
  }
}
```

Queries with no matches are logged as zero-result queries and do not fail the run.

## Notes

Use this actor when you need to discover the Google Finance symbol/exchange pair before calling Google Finance Quote or Historical Prices. For higher-volume Google Finance extraction or direct API access, use Scrappa's Google Finance API at `https://scrappa.co/api/google-finance/search`.
