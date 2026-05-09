# Google Finance Quote Scraper

Scrape complete ticker quote data from Google Finance for market research, portfolio monitoring, enrichment workflows, and investment dashboards. The actor wraps Scrappa's `/api/google-finance/quote` endpoint and returns a structured Apify dataset item for the requested ticker.

## What you get

- Current ticker quote summary: name, symbol, exchange, price, currency, price change, percent change, and market status
- Key statistics such as previous close, day range, year range, market cap, volume, P/E ratio, dividend yield, and other fields exposed by Google Finance
- Company profile/about data
- Financial statement rows with quarterly or annual period filtering
- Recent news items
- Related tickers from Google Finance discovery sections
- Full Scrappa response saved to key-value store record `OUTPUT`

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

`exchange` is optional, but recommended. Without it, Scrappa attempts to resolve the exchange before fetching the quote, which can add latency and may fail for ambiguous symbols.

## Output

The dataset contains one quote item for the requested ticker:

```json
{
  "symbol": "AAPL",
  "exchange": "NASDAQ",
  "name": "Apple Inc",
  "current_price": 198.53,
  "currency": "USD",
  "price_change": 1.18,
  "percent_change": 0.6,
  "market_status": "Closed",
  "key_stats": {
    "Previous close": "197.35",
    "Market cap": "2.96T USD"
  },
  "about": {
    "description": "Apple Inc. is an American multinational technology company..."
  },
  "financials": [],
  "news": [],
  "related_tickers": [],
  "request_symbol": "AAPL",
  "request_exchange": "NASDAQ",
  "request_period_type": "quarterly",
  "request_hl": "en",
  "request_gl": "us"
}
```

## Notes

For higher-volume Google Finance data extraction or direct API access, use Scrappa's Google Finance API at `https://scrappa.co/api/google-finance/quote`.
