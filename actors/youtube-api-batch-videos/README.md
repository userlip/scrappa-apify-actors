# YouTube API Batch Videos

Fetch details for multiple YouTube videos in one run. The Actor accepts comma-separated YouTube video IDs and saves the Scrappa YouTube API response to the default Apify dataset.

## Input

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `ids` | string | Yes | Comma-separated YouTube video IDs. The upstream API supports up to 50 IDs per request. |

## Example Input

```json
{
  "ids": "7eul_Vt6SZY,6QQQKJJBJOY"
}
```

## Output

The Actor stores one dataset item per video returned by the API. Fields depend on the current Scrappa YouTube response, and typically include video identifiers, title, channel metadata, thumbnails, duration, view counts, and publish metadata.

## Pricing

$0.30 per 1,000 results. No additional API keys required.

## Support

For issues or questions, contact us through Apify.
