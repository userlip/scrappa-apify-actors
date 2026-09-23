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

The actor attempts one normalized dataset item per channel. For pay-per-event runs, each successful or error row consumes a default-dataset-item event against the run's total spending limit, and charges already recorded for other events reduce the remaining row allowance. When no row is affordable, remaining channels are still fetched but their rows are skipped, so the dataset may contain fewer items than input channels. Missing or invalid run pricing fails before channel requests; dataset write failures fail the actor.

## Endpoint

`https://scrappa.co/api/youtube/channel`
