# YouTube API Video Chapters

Fetch chapter markers for a YouTube video by video ID through the Scrappa YouTube API.

This actor is a thin Apify wrapper around Scrappa's YouTube API. Scraping runs on Scrappa infrastructure; Apify handles input validation, run orchestration, and dataset output.

## Input

Provide a YouTube video ID. Optional pagination and filter fields are passed through to Scrappa when present.

```json
{
  "id": "dQw4w9WgXcQ"
}
```

## Output

The video chapters object returned by Scrappa.

## Endpoint

`https://ytapi.scrappa.co/videos/chapters`
