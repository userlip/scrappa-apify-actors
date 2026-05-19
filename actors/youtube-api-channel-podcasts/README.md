# YouTube API Channel Podcasts

Fetch podcast videos from a YouTube channel by channel ID. The Actor calls the Scrappa YouTube channel podcasts endpoint and saves each returned video to the default Apify dataset.

## Input

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `id` | string | Yes | YouTube channel ID. |
| `sort` | string | No | Sort order: `newest`, `popular`, or `oldest`. Defaults to `newest`. |
| `continuation` | string | No | Pagination token returned by a previous run. |

## Example Input

```json
{
  "id": "UCJZv4d5rbIKd4QHMPkcABCw",
  "sort": "newest"
}
```

## Output

The Actor stores one dataset item per podcast video returned by the API. Fields depend on the current Scrappa YouTube response and typically include the video ID, title, URL, thumbnails, duration, view count, and publish metadata.

If the API returns a continuation token, the Actor logs it so you can pass it in the next run to fetch the next page.

## Pricing

$0.30 per 1,000 results. No additional API keys required.

## Support

For issues or questions, contact us through Apify.
