# YouTube API Channel Statistics

Fetch YouTube channel statistics by channel ID through the Scrappa YouTube API.

This actor is a thin Apify wrapper around Scrappa's YouTube API. Scraping runs on Scrappa infrastructure; Apify handles input validation, run orchestration, and dataset output.

Set `SCRAPPA_API_KEY` as an Actor secret before running this wrapper.

## Input

Provide one or more YouTube channel IDs. Use `ids` for normal batch runs so one Apify run can return multiple channel-statistic records. The legacy `id` field is still accepted for existing integrations.

```json
{
  "ids": "UCJZv4d5rbIKd4QHMPkcABCw,UC_x5XG1OV2P6uZZ5FSM9Ttw"
}
```

## Output

One dataset item per channel statistics object returned by Scrappa; failed channel requests add an error item. Rows are saved in response order up to the Apify run's default-dataset-item spending limit, and later rows are omitted when it is exhausted.

This preflight limits default-dataset-item charges, not Scrappa API requests. Since those API requests finish before the budget lookup, they may still occur when no dataset rows can be saved.

## Endpoint

`https://scrappa.co/api/youtube/channel`
