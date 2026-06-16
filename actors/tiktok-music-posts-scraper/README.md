# TikTok Music Posts Scraper

TikTok Music Posts Scraper extracts public TikTok videos that use specific music tracks or sounds through Scrappa. Use it for TikTok sound videos, TikTok music track posts, trend monitoring, creator discovery, campaign research, and content intelligence workflows.

## Features

- Lookup by one or more TikTok music IDs in a single Apify run
- Fetch a page of public posts for each music track with engagement and media metadata
- Support pagination via `cursor`
- Dataset rows optimized for Apify table views
- One dataset item per returned TikTok post
- Compact `OUTPUT` summary for compatibility

## Input

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `musicIds` | array of strings | No | TikTok music track IDs. Batch multiple music IDs in one run to reduce Apify run overhead. |
| `music_id` | string or number | No | Legacy single music ID input. Prefer `musicIds` for new integrations. |
| `count` | integer | No | Number of posts to return for each music ID. Scrappa accepts `1-50`. |
| `cursor` | string or number | No | Pagination cursor from a previous run. Leave empty for the first page. The same cursor is applied to every music ID. |

Provide at least one value in `musicIds` or `music_id`.

## Example Input

```json
{
  "musicIds": ["7002634556977908485"],
  "count": 10,
  "cursor": "0"
}
```

## Output

Each TikTok music post is saved as one dataset item:

```json
{
  "aweme_id": "7568510388342443294",
  "desc": "Example post caption",
  "create_time": 1731161993,
  "digg_count": 12345,
  "comment_count": 678,
  "share_count": 90,
  "play_count": 1234567,
  "author": {
    "unique_id": "tiktok",
    "nickname": "TikTok"
  },
  "request_music_id": "7002634556977908485"
}
```

The `OUTPUT` key-value-store record contains a compact summary with processed music IDs, per-music pagination cursors, and total posts extracted. Dataset output is the primary result channel.

## Pricing

This actor is intended for paid per-result usage, aligned with one dataset item per TikTok post returned.

## Support

For higher-volume usage or direct API access, use Scrappa at https://scrappa.co.
