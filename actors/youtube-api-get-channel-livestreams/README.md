# YouTube API Channel Livestreams

Fetch livestream videos for a YouTube channel by channel ID through the Scrappa YouTube API.

This actor is a thin Apify wrapper around Scrappa's YouTube API. Scraping runs on Scrappa infrastructure; Apify handles input validation, run orchestration, and dataset output.

Set `SCRAPPA_API_KEY` as an Actor secret before running this wrapper.

## Input

Provide one or more YouTube channel IDs. Use `ids` for batch runs; the legacy `id` field still works. A `continuation` token is only valid for one channel. The actor follows continuation tokens for at most 10 pages per channel, stopping once it has at least 10 livestreams or the API has no next page. `sort` is passed to Scrappa.

```json
{
  "ids": "UCJZv4d5rbIKd4QHMPkcABCw,UC_x5XG1OV2P6uZZ5FSM9Ttw",
  "sort": "newest"
}
```

## Output

One dataset item is saved for each livestream found within the run's remaining pay-per-event spending limit. Before the first dataset write, the actor reads the Apify run's resolved event prices and charged-event counts, then skips rows that exceed `maxTotalChargeUsd`. The next continuation token, when available, is logged for a subsequent run.

## Endpoint

`https://scrappa.co/api/youtube/channel-videos`
