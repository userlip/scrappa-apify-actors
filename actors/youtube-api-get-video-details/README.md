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

The Actor stores the video details object returned by the API. Fields depend on the current Scrappa YouTube response, and typically include the video ID, title, channel details, thumbnails, duration, engagement counts, description, and publish metadata.

## Pricing

$0.30 per 1,000 results. No additional API keys required.

## Support

For issues or questions, contact us through Apify.
