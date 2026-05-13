# TikTok Comments Scraper

Extract comments and optional nested replies from public TikTok video URLs through Scrappa. Use it for creator research, campaign monitoring, social listening, sentiment analysis, and comment export workflows.

## Features

- Comment text, IDs, timestamps, like counts, and reply counts
- Comment author details including TikTok user ID, username, nickname, and avatar
- Optional reply collection through Scrappa's TikTok comment replies endpoint
- Flat dataset rows with `comment_type` and `parent_comment_id` for reply/thread analysis
- Pagination support with `cursor`
- Dataset rows optimized for Apify table views
- Full top-level comments response saved to the `OUTPUT` key-value-store record

## Input

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `url` | string | Yes | Public TikTok video URL |
| `count` | integer | No | Number of comments to return, 1-50 |
| `cursor` | string | No | Pagination cursor from a previous response |
| `includeReplies` | boolean | No | Fetch replies for top-level comments with replies |
| `maxRepliesPerComment` | integer | No | Maximum replies to fetch per top-level comment, 1-500 |

## Example Input

```json
{
  "url": "https://www.tiktok.com/@tiktok/video/7568510388342443294",
  "count": 20,
  "includeReplies": true,
  "maxRepliesPerComment": 50
}
```

## Output

Each TikTok comment is saved as one dataset item:

```json
{
  "comment_type": "comment",
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
  "parent_comment_id": null,
  "parent_comment_text": null,
  "video_id": "7568510388342443294",
  "video_url": "https://www.tiktok.com/@tiktok/video/7568510388342443294"
}
```

When `includeReplies` is enabled, replies are saved as additional dataset items:

```json
{
  "comment_type": "reply",
  "comment_id": "7093220000000000000",
  "text": "Reply text",
  "create_time": 1710000100,
  "digg_count": 8,
  "reply_count": 0,
  "user": {
    "user_id": "987654321",
    "unique_id": "reply_user",
    "nickname": "Reply User",
    "avatar": "https://..."
  },
  "parent_comment_id": "7093219663211053829",
  "parent_comment_text": "Great video",
  "video_id": "7568510388342443294",
  "video_url": "https://www.tiktok.com/@tiktok/video/7568510388342443294"
}
```

The full top-level comments API response is always saved to `OUTPUT`, including `data.hasMore` and `data.cursor` for the next comment page. When replies are enabled, raw reply responses are saved separately to `REPLIES_OUTPUT` and grouped by parent comment ID.

## Pagination

Run the actor once without `cursor`. If `OUTPUT.data.hasMore` is true, run it again with `cursor` set to `OUTPUT.data.cursor`. This `OUTPUT` pagination path stays the same whether `includeReplies` is enabled or disabled.

Reply collection is sequential to avoid overwhelming the API. For high-reply videos, use `count` and `maxRepliesPerComment` to keep runs within the actor timeout.

## Support

For higher-volume usage or direct API access, use Scrappa at https://scrappa.co.
