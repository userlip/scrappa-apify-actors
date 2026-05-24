# YouTube API Channel Statistics

Fetch YouTube channel statistics by channel ID through the Scrappa YouTube API.

This actor is a thin Apify wrapper around Scrappa's YouTube API. Scraping runs on Scrappa infrastructure; Apify handles input validation, run orchestration, and dataset output.

## Input

Provide a YouTube channel ID. Optional pagination and filter fields are passed through to Scrappa when present.

```json
{
  "id": "UCJZv4d5rbIKd4QHMPkcABCw"
}
```

## Output

The channel statistics object returned by Scrappa.

## Endpoint

`https://ytapi.scrappa.co/channels/stats`
