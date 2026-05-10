# TikTok Hashtag Posts Scraper

TikTok hashtag scraper for extracting public posts from a hashtag or challenge through Scrappa. Use it for trend tracking, content research, viral post discovery, hashtag monitoring, and campaign performance workflows.

## Features

- Lookup by TikTok hashtag, full hashtag URL, or numeric challenge ID
- Fetch a page of public hashtag posts with engagement and media metadata
- Optional region targeting
- Support pagination via `cursor`
- Dataset rows optimized for Apify table views
- Full Scrappa response saved to the `OUTPUT` key-value-store record

## Input

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `hashtag` | string | Yes | TikTok hashtag with or without `#`, full hashtag URL, or numeric challenge ID. Bare numeric values are treated as challenge IDs. |
| `region` | string | No | Optional country or region code, such as `US`. |
| `count` | integer | No | Number of posts to return. Scrappa accepts `1-50`. |
| `cursor` | string | No | Pagination cursor from a previous run. Leave empty for the first page. |

## Example Input

```json
{
  "hashtag": "cosplay",
  "region": "US",
  "count": 10,
  "cursor": "0"
}
```

## Output

Each TikTok hashtag post is saved as one dataset item:

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
  "lookup_challenge_name": "cosplay",
  "lookup_challenge_id": null,
  "resolved_challenge_name": "Cosplay",
  "resolved_challenge_id": "33380",
  "lookup_region": "US"
}
```

The full API response, including pagination metadata, is saved to `OUTPUT`.

## Support

For higher-volume usage or direct API access, use Scrappa at https://scrappa.co.
