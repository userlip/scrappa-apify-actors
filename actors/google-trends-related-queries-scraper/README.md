# Google Trends Related Queries Scraper

Scrappa-powered Apify Actor for Google Trends related queries and keyword research.

This actor is focused on one workflow: expand a seed keyword into related Google Trends queries and topics. It is separate from the Google Trends Interest Scraper, which returns interest-over-time timeline points.

## What You Get

- Top and rising related Google Trends queries when available
- Related Google Trends topics when available
- One Apify dataset row per related query or topic
- Request context on every row: source keyword, geo, time range, language, and search type
- Optional Google Trends autocomplete suggestions in the `OUTPUT` summary

Google Trends may return sparse or empty related-query data for low-volume keywords. For reliable first tests, use broad keywords such as `coffee`, `bitcoin`, `fitness`, or `tesla`.

## Input Example

```json
{
  "query": "coffee",
  "geo": "US",
  "time_range": "1y",
  "hl": "en",
  "search_type": "web",
  "include_autocomplete": false
}
```

## Dataset Output

```json
{
  "position": 1,
  "result_kind": "query",
  "type": "top",
  "query": "coffee near me",
  "topic": null,
  "topic_type": null,
  "value": 100,
  "formatted_value": "100",
  "link": "https://trends.google.com/trends/explore?q=coffee+near+me",
  "source_keyword": "coffee",
  "request_geo": "US",
  "request_time_range": "1y",
  "request_hl": "en",
  "request_search_type": "web",
  "response_time_ms": 623
}
```

The `type` field is `top`, `rising`, or `null` depending on the shape returned by Google Trends. Topic rows use `result_kind: "topic"` and preserve the topic category in `topic_type`.

## Use Cases

SEO keyword research:

```json
{
  "query": "project management software",
  "geo": "US",
  "time_range": "90d",
  "hl": "en"
}
```

Content planning:

```json
{
  "query": "air fryer recipes",
  "geo": "US",
  "time_range": "1y",
  "search_type": "youtube"
}
```

Market monitoring:

```json
{
  "query": "electric vehicles",
  "geo": "US",
  "time_range": "30d",
  "include_autocomplete": true
}
```

## Development

```bash
npm install
npm test
```

## Run Locally

```bash
SCRAPPA_API_KEY=... npm run build
SCRAPPA_API_KEY=... apify run --input='{"query":"coffee","geo":"US","time_range":"1y","hl":"en","search_type":"web"}'
```

## Scrappa API

This actor is a thin wrapper around Scrappa's Google Trends API. It calls:

- `GET https://scrappa.co/api/google-trends/related`
- `GET https://scrappa.co/api/google-trends/autocomplete` only when `include_autocomplete` is true

For higher-volume workloads, direct Scrappa API access avoids Apify run overhead while keeping the same underlying data source.
