# YouTube API Get Video Details

Fetch detailed metadata for a single YouTube video by video ID. The Actor calls the Scrappa YouTube video details endpoint and saves the response to the default Apify dataset.

## Input

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `id` | string | Yes | YouTube video ID. |

## Example Input

```json
{
  "id": "dQw4w9WgXcQ"
}
```

## Output

The Actor saves the response returned by the API subject to the run's `maxTotalChargeUsd` limit. An object counts as one dataset item; an array counts one item per element and is saved in original order only up to the remaining affordable item capacity. Charges already recorded for other priced events in the run reduce that capacity; if no capacity remains, no items are written.

## Pricing

$0.30 per 1,000 results. No additional API keys required.

## Support

For issues or questions, contact us through Apify.
