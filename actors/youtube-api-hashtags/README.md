# YouTube API Hashtags

Search YouTube videos by hashtag through the Scrappa YouTube API.

This actor is a thin Apify wrapper around Scrappa's YouTube API. Scraping runs on Scrappa infrastructure; Apify handles input validation, run orchestration, and dataset output.

Set `SCRAPPA_API_KEY` as an Actor secret before running this wrapper.

## Input

Provide a hashtag, with or without `#`. Optional pagination, sort, duration, and upload date fields are passed through to Scrappa when present.

```json
{
  "hashtag": "javascript",
  "limit": 10,
  "sort": "relevance"
}
```

## Output

The actor saves as many returned results as the run's pay-per-event spending limit allows, up to one dataset item per result. It reads run pricing before writing and fails without publishing results if that lookup fails.

## Endpoint

`https://scrappa.co/api/youtube/search`
