# YouTube API Playlists

Search YouTube playlists by query through the Scrappa YouTube API.

This actor is a thin Apify wrapper around Scrappa's YouTube API. Scraping runs on Scrappa infrastructure; Apify handles input validation, run orchestration, and dataset output.

Set `SCRAPPA_API_KEY` as an Actor secret before running this wrapper.

## Input

Provide a playlist search query. Optional pagination and filter fields are passed through to Scrappa when present.

```json
{
  "q": "web scraping",
  "limit": 10,
  "sort": "relevance"
}
```

## Output

The actor writes an affordable prefix of Scrappa's playlist results in API order. Before posting, it reads the Apify run's resolved pay-per-event prices, existing charged-event counts, and numeric spending limit; if the pricing metadata is unavailable or invalid, the actor fails without writing dataset rows.

## Endpoint

`https://scrappa.co/api/youtube/search`
