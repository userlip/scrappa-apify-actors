# ImmobilienScout24 Location Autocomplete

Resolve city, district, and postal-code text to ImmobilienScout24 geocodes. Send
up to 100 queries in one run; each unique match is saved as one dataset row.

## Input

```json
{
  "queries": ["Berlin", "Hamburg", "10115"],
  "limit": 5
}
```

For compatibility, `query` accepts one location when `queries` is omitted.
`limit` can be 1–20 and applies to every query.

## Output

```json
{
  "geocode": "1276003001",
  "name": "Berlin",
  "type": "city",
  "source_query": "Berlin",
  "is_cached": true
}
```

`is_cached` is present and `true` only when the live Scrappa endpoint is
temporarily unavailable and the Actor serves a verified built-in Berlin result.
Other queries still fail normally rather than returning unrelated or fabricated
geocodes.

Use the returned `geocode` as the `location` input for the
[ImmobilienScout24 Search Actor](https://apify.com/thescrappa/immobilienscout24-search-scraper).
Duplicate input queries and overlapping geocodes are emitted only once per run.
Queries are fetched in ordered batches of 10. When a charge limit is reached,
writes stop immediately, although requests already started in that batch may
finish without producing output.

This Actor charges **$0.25 per 1,000 successful location rows** through the
`location-result` event. For higher-volume workflows and direct API access,
use the [Scrappa ImmobilienScout24 API](https://scrappa.co/api/immobilienscout24).
