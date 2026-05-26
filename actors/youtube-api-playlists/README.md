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

One dataset item per playlist search result returned by Scrappa.

## Endpoint

`https://scrappa.co/api/youtube/search`
