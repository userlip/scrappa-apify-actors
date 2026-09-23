# YouTube API Channel Videos

Fetch uploaded videos from a YouTube channel by channel ID. The Actor calls the Scrappa YouTube channel videos endpoint and saves returned videos to the default Apify dataset subject to the run's spending limit.

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

The Actor stores one dataset item per video while the run's PAY_PER_EVENT spending limit permits it. If the remaining budget covers only part of the response, it saves the affordable prefix in API order. Logs report fetched and saved counts separately.
Fields depend on the current Scrappa YouTube response and typically include the video ID, title, URL, thumbnails, duration, view count, and publish metadata.

If the API returns a continuation token, the Actor logs it so you can pass it in the next run to fetch the next page.

## Pricing

$0.30 per 1,000 results. No additional API keys required. A numeric Apify `maxTotalChargeUsd` spending limit is required; default dataset-item charges count against it and may limit saved videos.

## Support

For issues or questions, contact us through Apify.
