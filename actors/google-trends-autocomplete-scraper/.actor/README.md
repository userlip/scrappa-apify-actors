# Google Trends Autocomplete Scraper

Get Google Trends autocomplete suggestions from Scrappa's live Google Trends API.

Use it for SEO keyword expansion, content ideation, and localized market research. The actor saves one dataset item per suggestion and includes request context on every row.

## Input

```json
{
  "query": "tesla",
  "geo": "US",
  "hl": "en"
}
```

`q` is accepted as an alias for `query` for compatibility with direct Scrappa API inputs.

## Output

```json
{
  "position": 1,
  "suggestion": "tesla stock",
  "type": null,
  "source_keyword": "tesla",
  "request_geo": "US",
  "request_hl": "en",
  "response_time_ms": 412
}
```

This actor is a thin Apify wrapper around `GET https://scrappa.co/api/google-trends/autocomplete`. For high-volume usage, direct Scrappa API access avoids Apify run overhead.
