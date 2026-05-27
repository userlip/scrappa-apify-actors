# YouTube API Channel Playlists

Fetch playlists for a YouTube channel by channel ID through the Scrappa YouTube API.

This actor is a thin Apify wrapper around Scrappa's YouTube API. Scraping runs on Scrappa infrastructure; Apify handles input validation, run orchestration, and dataset output.

Set `SCRAPPA_API_KEY` as an Actor secret before running this wrapper.

## Input

Provide one or more YouTube channel IDs. Use `ids` for normal batch runs; the legacy `id` field still works for a single channel.

```json
{
  "ids": "UCJZv4d5rbIKd4QHMPkcABCw,UC_x5XG1OV2P6uZZ5FSM9Ttw"
}
```

## Output

One dataset item per channel playlist returned by Scrappa.

## Endpoint

`https://scrappa.co/api/youtube/channel-playlists`
