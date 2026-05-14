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

The Actor stores the playlist details object returned by the API. Fields depend on the current Scrappa YouTube response, and typically include the playlist ID, title, channel details, thumbnails, video count, videos, description, and continuation token when more videos are available.

## Pricing

$0.30 per 1,000 results. No additional API keys required.

## Support

For issues or questions, contact us through Apify.
