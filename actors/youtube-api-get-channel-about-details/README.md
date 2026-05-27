# YouTube API Channel About Details

Fetch YouTube channel about details by channel ID through the Scrappa YouTube API.

This actor is a thin Apify wrapper around Scrappa's YouTube API. Scraping runs on Scrappa infrastructure; Apify handles input validation, run orchestration, and dataset output.

Set `SCRAPPA_API_KEY` as an Actor secret before running this wrapper.

## Input

Provide one or more YouTube channel IDs. Use `ids` for normal batch runs so one Apify run can return multiple channel about/details records. The legacy `id` field is still accepted for existing integrations.

```json
{
  "ids": "UCJZv4d5rbIKd4QHMPkcABCw,UC_x5XG1OV2P6uZZ5FSM9Ttw"
}
```

## Output

One dataset item per channel about/details object returned by Scrappa.

## Endpoint

`https://scrappa.co/api/youtube/channel`
