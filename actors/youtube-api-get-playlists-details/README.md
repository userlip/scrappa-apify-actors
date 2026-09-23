# YouTube API Get Playlist Details

Fetch detailed metadata and videos for a single YouTube playlist by playlist ID. The Actor calls the Scrappa YouTube playlist endpoint and saves the response to the default Apify dataset.

## Input

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `id` | string | Yes | YouTube playlist ID. |

## Example Input

```json
{
  "id": "PLrAXtmErZgOeiKm4sgNOknGvNjby9efdf"
}
```

## Output

The Actor saves playlist detail results from the API to the default Apify dataset in their original order, limited to the rows allowed by the run's `PAY_PER_EVENT` spending limit. A response array may be trimmed to fit the remaining budget. Before a non-empty dataset write, missing or invalid run pricing metadata fails the Actor rather than writing rows.

## Pricing

$0.30 per 1,000 results. No additional API keys required.

## Support

For issues or questions, contact us through Apify.
