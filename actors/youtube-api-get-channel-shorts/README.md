# YouTube API Channel Shorts

Fetch Shorts videos for a YouTube channel by channel ID through the Scrappa YouTube API.

This actor is a thin Apify wrapper around Scrappa's YouTube API. Scraping runs on Scrappa infrastructure; Apify handles input validation, run orchestration, and dataset output.

Set `SCRAPPA_API_KEY` as an Actor secret before running this wrapper.

## Input

Provide one or more YouTube channel IDs. `ids` and the legacy `id` field are combined and deduplicated; `continuation` can only be used for a single channel. Optional `sort` is passed through to Scrappa. The actor scans at most 10 pages per channel to collect up to 10 Shorts.

```json
{
  "ids": "UCJZv4d5rbIKd4QHMPkcABCw,UC_x5XG1OV2P6uZZ5FSM9Ttw",
  "sort": "newest"
}
```

## Output

One dataset row per detected Short, kept in Scrappa page order.

## Endpoint

`https://scrappa.co/api/youtube/channel-videos`
