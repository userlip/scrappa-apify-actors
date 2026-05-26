# YouTube API Channel Playlists

Fetch playlists for a YouTube channel by channel ID through the Scrappa YouTube API.

This actor is a thin Apify wrapper around Scrappa's YouTube API. Scraping runs on Scrappa infrastructure; Apify handles input validation, run orchestration, and dataset output.

## Input

Provide a YouTube channel ID. Optional pagination and filter fields are passed through to Scrappa when present.

```json
{
  "id": "UCJZv4d5rbIKd4QHMPkcABCw"
}
```

## Output

One dataset item per channel playlist returned by Scrappa.

## Endpoint

`https://scrappa.co/api/youtube/channel-playlists`
