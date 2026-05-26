# YouTube API Channel Shorts

Fetch Shorts videos for a YouTube channel by channel ID through the Scrappa YouTube API.

This actor is a thin Apify wrapper around Scrappa's YouTube API. Scraping runs on Scrappa infrastructure; Apify handles input validation, run orchestration, and dataset output.

## Input

Provide a YouTube channel ID. Optional pagination and filter fields are passed through to Scrappa when present.

```json
{
  "id": "UCJZv4d5rbIKd4QHMPkcABCw",
  "sort": "newest"
}
```

## Output

One dataset item per Shorts video returned by Scrappa.

## Endpoint

`https://scrappa.co/api/youtube/channel-videos`
