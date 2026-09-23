# YouTube API Channel About Details

Fetch YouTube channel about details by channel ID through the Scrappa YouTube API.

This actor calls Scrappa's YouTube API for each requested channel, maps its response to the legacy channel-about-details shape, and writes successful results and per-channel errors to the Apify dataset.

Set `SCRAPPA_API_KEY` as an Actor secret before running this wrapper.

## Input

Provide one or more YouTube channel IDs. Use `ids` for normal batch runs so one Apify run can return multiple channel about/details records. The legacy `id` field is still accepted for existing integrations.

```json
{
  "ids": "UCJZv4d5rbIKd4QHMPkcABCw,UC_x5XG1OV2P6uZZ5FSM9Ttw"
}
```

## Output

One normalized dataset item is written per channel. Failed requests also emit `{ "id", "error", "success": false }` rows; the actor run fails when every channel request fails or a dataset write itself fails.

## Endpoint

`https://scrappa.co/api/youtube/channel`
