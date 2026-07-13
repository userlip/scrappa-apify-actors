# Google Finance Indices Scraper

Fetch Google Finance index quotes for market research, dashboards, watchlists, and monitoring workflows. This actor is a thin Scrappa wrapper: provide multiple index symbols in one Apify run and receive one structured dataset row for each successfully matched unique index.

## Use cases

- Track major US benchmarks such as the S&P 500 (`.INX`), Dow Jones Industrial Average (`.DJI`), and NASDAQ Composite (`.IXIC`)
- Build a multi-index watchlist for a dashboard or recurring market-monitoring workflow
- Request custom Google Finance index symbols for a specific exchange or region

## Batch input

Pass up to 50 symbols as a JSON array, or as a comma-separated string. Symbols are trimmed, uppercased, and deduplicated, so a symbol is saved at most once per run.

```json
{
  "indices": [".INX", ".DJI", ".IXIC", ".FTSE"],
  "hl": "en",
  "gl": "us"
}
```

`.INX`, `.DJI`, and `.IXIC` cover the S&P 500, Dow, and NASDAQ Composite. `.FTSE` illustrates a custom symbol; custom symbols are supported, but Google Finance's upstream matching can return no row or a different index. The actor logs mismatches and does not charge for them.

## Output and pricing

Each successful unique index is written as one Apify dataset row with its requested symbol, matched symbol, name, exchange, current price, price change, percent change, previous close, movement direction, locale, and retrieval timestamp.

Each saved dataset row emits one `index-result` event and costs **$0.00025 per result**. Duplicate input symbols, unmatched symbols, and mismatched upstream responses are not saved or charged. Output is dataset-only; the actor does not write an `OUTPUT` key-value-store record.

```json
{
  "id": "INDEXSP:.INX",
  "requested_symbol": ".INX",
  "symbol": ".INX",
  "name": "S&P 500",
  "exchange": "INDEXSP",
  "current_price": 6200.0,
  "price_change": 12.4,
  "percent_change": 0.2,
  "previous_close": 6187.6,
  "movement_direction": "Up",
  "request_hl": "en",
  "request_gl": "us",
  "retrieved_at": "2026-07-11T00:00:00.000Z"
}
```

## Higher-volume access

For higher-volume Google Finance index workloads or direct integration, use the [Scrappa Google Finance Indices API](https://scrappa.co/api/google-finance/indices) directly.
