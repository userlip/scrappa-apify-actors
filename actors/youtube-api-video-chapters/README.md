# YouTube API Video Chapters

Fetch chapter markers for a YouTube video by video ID through the Scrappa YouTube API.

This actor is a thin Apify wrapper around Scrappa's YouTube API. Scraping runs on Scrappa infrastructure; Apify handles input validation, run orchestration, and dataset output.

Set `SCRAPPA_API_KEY` as an Actor secret before running this wrapper.

## Input

Provide one or more YouTube video IDs. Use `ids` for batch runs; the legacy `id` field still works for a single video.

```json
{
  "ids": "dQw4w9WgXcQ,aqz-KE-bpKQ"
}
```

## Output

Successful response objects are written as one dataset item per video ID; top-level response arrays may produce multiple items and are trimmed to the run's remaining dataset-item capacity. Failed requests produce per-video error items while capacity remains. The actor continues fetching every ID after the budget is exhausted but skips further dataset writes.

## Endpoint

`https://scrappa.co/api/youtube/chapters`
