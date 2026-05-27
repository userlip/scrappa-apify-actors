# YouTube API Channel Podcasts

Fetch podcast videos from a YouTube channel by channel ID. The Actor calls the Scrappa YouTube API and saves each returned video to the default Apify dataset.

Set `SCRAPPA_API_KEY` as an Actor secret before running this wrapper.

## Input

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `ids` | string | No | Comma-separated YouTube channel IDs. Prefer this for batch runs. |
| `id` | string | No | Legacy single YouTube channel ID. |
| `sort` | string | No | Sort order for the channel videos page before podcast filtering: `newest`, `popular`, or `oldest`. Defaults to `newest`. |
| `continuation` | string | No | Pagination token returned by a previous run. |

## Example Input

```json
{
  "ids": "UCJZv4d5rbIKd4QHMPkcABCw,UC_x5XG1OV2P6uZZ5FSM9Ttw",
  "sort": "newest"
}
```

## Output

The Actor stores one dataset item per podcast video returned by the API. Fields depend on the current Scrappa YouTube response and typically include the video ID, title, URL, thumbnails, duration, view count, and publish metadata.

Podcast filtering is applied after Scrappa returns the channel videos page. If a channel has many regular uploads before older podcast content, use a continuation token to keep paging through results.

If the API returns a continuation token, the Actor logs it so you can pass it in the next run to fetch the next page.

## Pricing

$0.30 per 1,000 results.

## Support

For issues or questions, contact us through Apify.
