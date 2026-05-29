# Google Finance Intraday Scraper

Scrape minute-level intraday price and volume graph data from Google Finance for charting, trading dashboards, watchlists, market monitoring, and research workflows. The actor wraps Scrappa's `/api/google-finance/intraday` endpoint and writes one Apify dataset item per intraday graph point.

## What you get

- Minute-level intraday price points
- Price, change, percent change, and volume per point
- Symbol, exchange, and currency metadata
- Batch input so you can fetch multiple tickers in one Apify run
- Run-level summary saved to key-value store record `OUTPUT`

## Input

```json
{
  "symbols": [
    {
      "symbol": "AAPL",
      "exchange": "NASDAQ"
    },
    {
      "symbol": "MSFT",
      "exchange": "NASDAQ"
    }
  ],
  "hl": "en",
  "gl": "us"
}
```

`exchange` is optional, but recommended. Without it, Scrappa attempts to resolve the exchange before fetching intraday graph data, which can add latency and may fail for ambiguous symbols.

## Output

The dataset contains one item per intraday graph point:

```json
{
  "position": 1,
  "date": "Jun 16 2025, 09:30 AM UTC-04:00",
  "date_iso": "2025-06-16T13:30:00.000Z",
  "price": 198.42,
  "change": 1.25,
  "percent_change": 0.63,
  "volume": 3482103,
  "symbol": "AAPL",
  "exchange": "NASDAQ",
  "currency": "USD",
  "request_symbol": "AAPL",
  "request_exchange": "NASDAQ",
  "request_hl": "en",
  "request_gl": "us"
}
```

## Notes

Intraday availability depends on Google Finance and the current trading session. Some symbols may return no data outside supported markets or sessions.

For higher-volume Google Finance data extraction or direct API access, use Scrappa's Google Finance API at `https://scrappa.co/api/google-finance/intraday`.
