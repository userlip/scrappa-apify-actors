# Google Trends Interest Scraper

Scrape Google Trends interest-over-time data for a keyword and export a clean timeline of dated 0-100 interest values. Use it for SEO prioritization, content planning, market research, product demand checks, and competitor monitoring.

## What You Get

- One dataset item per Google Trends timeline point
- Date, Unix timestamp, and normalized interest value
- Average, maximum, and minimum interest values
- Request context for keyword, location, time range, language, and search type
- Full Scrappa API response saved to key-value store as `OUTPUT`

## Input

```json
{
  "q": "tesla",
  "geo": "US",
  "time_range": "1y",
  "hl": "en",
  "search_type": "web"
}
```

## Output Example

```json
{
  "position": 1,
  "date": "2024-01-01",
  "timestamp": 1704067200,
  "value": 45,
  "average": 52.3,
  "max_value": 100,
  "min_value": 12,
  "request_q": "tesla",
  "request_geo": "US",
  "request_time_range": "1y",
  "request_hl": "en",
  "request_search_type": "web",
  "response_time_ms": 587
}
```

## Parameters

- `q`: keyword or phrase to analyze
- `geo`: Google Trends region code such as `US`, `GB`, `DE`, or `Worldwide`
- `time_range`: `1h`, `4h`, `1d`, `7d`, `30d`, `90d`, `1y`, `5y`, or `all`
- `hl`: two-letter language code such as `en`, `de`, `es`, or `fr`
- `search_type`: `web`, `images`, `news`, `youtube`, or `shopping`

## Notes

The actor is powered by Scrappa's Google Trends API. For high-volume API access, use Scrappa directly at https://scrappa.co/.
