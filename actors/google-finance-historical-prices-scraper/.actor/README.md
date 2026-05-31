# Google Finance Historical Prices Scraper

Scrape historical price and volume data from Google Finance for stock backtesting, charting, portfolio research, and market monitoring workflows. The actor wraps Scrappa's `/api/google-finance/historical` endpoint and writes one Apify dataset item per historical price point.

## What you get

- Daily, weekly, or monthly historical price points
- Close price, change, percent change, and volume per point
- Symbol, exchange, currency, and previous close metadata
- Preset ranges from 1 day to max
- Full Scrappa response saved to key-value store record `OUTPUT`

## Input

```json
{
  "symbol": "AAPL",
  "exchange": "NASDAQ",
  "range": 6,
  "interval": "daily",
  "hl": "en",
  "gl": "us"
}
```

`exchange` is optional, but recommended. Without it, Scrappa attempts to resolve the exchange before fetching historical prices, which can add latency and may fail for ambiguous symbols.

Use `range` for the most stable historical data. The Scrappa API contract also exposes `start_date` and `end_date`; when you use custom dates, send both fields and do not send `range`. If Scrappa returns no data for a custom date range, the actor exits successfully with an empty dataset and writes the no-data details to `OUTPUT`.

## Output

The dataset contains one item per historical price point:

```json
{
  "position": 1,
  "date": 1739480400,
  "date_iso": "2025-02-14",
  "close": 241.53,
  "change": 0,
  "percent_change": 0,
  "volume": 53614054,
  "symbol": "AAPL",
  "exchange": "NASDAQ",
  "currency": "USD",
  "previous_close": 275.5,
  "request_symbol": "AAPL",
  "request_exchange": "NASDAQ",
  "request_range": 6,
  "request_start_date": null,
  "request_end_date": null,
  "request_interval": "daily",
  "request_hl": "en",
  "request_gl": "us"
}
```

## Notes

For higher-volume Google Finance data extraction or direct API access, use Scrappa's Google Finance API at `https://scrappa.co/api/google-finance/historical`.
