# TikTok User Posts Scraper

Extract public TikTok posts for a creator through Scrappa. Use it for content research, creator benchmarking, trend tracking, posting cadence analysis, and engagement monitoring workflows.

## Features

- Lookup by TikTok username, full profile URL, or numeric user ID
- Fetch a page of public posts with engagement and media metadata
- Support pagination via `cursor`
- Dataset rows optimized for Apify table views
- Full Scrappa response saved to the `OUTPUT` key-value-store record

## Input

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `profile` | string | Yes | TikTok username with or without `@`, full profile URL, or numeric user ID. Bare numeric values are treated as user IDs; prefix numeric usernames with `@`. |
| `count` | integer | No | Number of posts to return. Scrappa accepts `1-50`. |
| `cursor` | string | No | Pagination cursor from a previous run. Leave empty for the first page. |

## Example Input

```json
{
  "profile": "@tiktok",
  "count": 10,
  "cursor": "0"
}
```

## Output

Each TikTok post is saved as one dataset item:

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
  "lookup_unique_id": "@tiktok",
  "lookup_user_id": null
}
```

The full API response, including pagination metadata, is saved to `OUTPUT`.

## Support

For higher-volume usage or direct API access, use Scrappa at https://scrappa.co.
