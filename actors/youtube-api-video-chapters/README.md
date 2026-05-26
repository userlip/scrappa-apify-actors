# YouTube API Video Chapters

Fetch chapter markers for a YouTube video by video ID through the Scrappa YouTube API.

This actor is a thin Apify wrapper around Scrappa's YouTube API. Scraping runs on Scrappa infrastructure; Apify handles input validation, run orchestration, and dataset output.

## Input

Provide one or more YouTube video IDs. Use `ids` for batch runs; the legacy `id` field still works for a single video.

```json
{
  "ids": "dQw4w9WgXcQ,aqz-KE-bpKQ"
}
```

## Output

One dataset item per video ID. Successful items contain the video chapters object returned by Scrappa. Failed items contain the video ID and error message so valid IDs in the same batch are still processed.

## Endpoint

`https://scrappa.co/api/youtube/chapters`
