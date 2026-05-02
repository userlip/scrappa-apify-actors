# TikTok Comments Scraper

Extract comments from public TikTok video URLs through Scrappa. Use it for creator research, campaign monitoring, social listening, sentiment analysis, and comment export workflows.

## Features

- Comment text, IDs, timestamps, like counts, and reply counts
- Comment author details including TikTok user ID, username, nickname, and avatar
- Pagination support with `cursor`
- Dataset rows optimized for Apify table views
- Full Scrappa response saved to the `OUTPUT` key-value-store record

## Input

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `url` | string | Yes | Public TikTok video URL |
| `count` | integer | No | Number of comments to return, 1-50 |
| `cursor` | string | No | Pagination cursor from a previous response |

## Example Input

```json
{
  "url": "https://www.tiktok.com/@tiktok/video/7568510388342443294",
  "count": 20
}
```

## Output

Each TikTok comment is saved as one dataset item:

```json
{
  "comment_id": "7093219663211053829",
  "text": "Great video",
  "create_time": 1710000000,
  "digg_count": 42,
  "reply_count": 3,
  "user": {
    "user_id": "123456789",
    "unique_id": "example_user",
    "nickname": "Example User",
    "avatar": "https://..."
  },
  "video_url": "https://www.tiktok.com/@tiktok/video/7568510388342443294"
}
```

The full API response is saved to `OUTPUT`, including `data.hasMore` and `data.cursor` for the next page.

## Pagination

Run the actor once without `cursor`. If `OUTPUT.data.hasMore` is true, run it again with `cursor` set to `OUTPUT.data.cursor`.

## Support

For higher-volume usage or direct API access, use Scrappa at https://scrappa.co.
