# Google Finance Markets Scraper

Scrape Google Finance market movers, trend rows, and finance news for market monitoring, watchlist enrichment, index dashboards, and trading research workflows. The actor wraps Scrappa's `/api/google-finance/markets` endpoint and writes one Apify dataset item per market row or news result.

## What you get

- Trend views for indexes, most active, gainers, losers, climate leaders, cryptocurrencies, and currencies
- Optional regional index filters for Americas, Europe/Middle East/Africa, and Asia-Pacific
- Finance news returned with trend responses
- Full Scrappa response saved to key-value store record `OUTPUT`

## Input

Market movers:

```json
{
  "trend": "gainers",
  "hl": "en",
  "gl": "us"
}
```

Most-active stocks:

```json
{
  "trend": "most-active",
  "hl": "en",
  "gl": "us"
}
```

Regional indexes:

```json
{
  "trend": "indexes",
  "index_market": "americas",
  "hl": "en",
  "gl": "us"
}
```

## Output

The dataset contains one item per market row or news result:

```json
{
  "item_type": "market_row",
  "section": "gainers",
  "trend": "gainers",
  "trend_group": null,
  "position": 1,
  "stock": "AAPL:NASDAQ",
  "name": "Apple Inc",
  "symbol": "AAPL",
  "exchange": "NASDAQ",
  "price": 189.98,
  "currency": "USD",
  "price_movement_direction": "Up",
  "price_movement_value": 3.25,
  "price_movement_percentage": 1.74,
  "request_trend": "gainers",
  "request_index_market": null,
  "request_hl": "en",
  "request_gl": "us"
}
```

Trend responses can also include finance news rows:

```json
{
  "item_type": "news_result",
  "section": "finance-news",
  "title": "Markets climb as investors weigh earnings",
  "source": "Example Finance",
  "date": "2026-05-15 13:30:00",
  "link": "https://example.com/markets-news",
  "snippet": "Major indexes moved higher...",
  "request_trend": "most-active"
}
```

## Notes

Use `trend` to choose the market view. Trend responses include market rows and may include related finance news. For higher-volume Google Finance data extraction or direct API access, use Scrappa's Google Finance API at `https://scrappa.co/api/google-finance/markets`.
