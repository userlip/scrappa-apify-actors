# TikTok Search Scraper

Search TikTok videos by keyword, hashtag, product, brand, topic, or creator niche through Scrappa. Use it for trend discovery, social listening, creator research, product research, competitor monitoring, and demand validation workflows.

## Features

- Search TikTok video results with `keywords`, or `query` as an API alias
- Filter by optional TikTok region code
- Control page size with `count`
- Continue searches with `cursor`
- Pass upstream `publish_time` and `sort_type` filters when needed
- Dataset rows optimized for Apify table views
- Full Scrappa response saved to the `OUTPUT` key-value-store record

## Input

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `keywords` | string | No | Keyword, hashtag, product, brand, topic, or creator niche to search for in TikTok videos. Required unless `query` is provided. |
| `query` | string | No | API alias for `keywords`. Ignored when `keywords` is provided. |
| `region` | string | No | Country or region code, such as `US`, `GB`, `JP`, or `DE`. |
| `count` | integer | No | Number of video results to return. Scrappa accepts `1-50`. |
| `cursor` | string | No | Pagination cursor from a previous run. Leave empty for the first page. |
| `publish_time` | integer | No | Optional upstream publish-time filter. Leave empty for TikTok's default search window; broad values can behave like no filter if TikTok has no matching retained results. |
| `sort_type` | integer | No | Optional upstream sort type. Leave empty for TikTok's default ranking. |

## Example Input

```json
{
  "keywords": "basketball",
  "region": "US",
  "count": 10,
  "cursor": "0"
}
```

## Output

Each TikTok video result is saved as one dataset item:

```json
{
  "aweme_id": "7568510388342443294",
  "desc": "Example TikTok video caption",
  "create_time": 1731161993,
  "digg_count": 12345,
  "comment_count": 678,
  "share_count": 90,
  "play_count": 1234567,
  "author": {
    "unique_id": "tiktok",
    "nickname": "TikTok"
  },
  "request_keywords": "basketball",
  "request_region": "US",
  "request_cursor": "0",
  "request_publish_time": null,
  "request_sort_type": null
}
```

The full API response, including pagination metadata, is saved to `OUTPUT`.

## Pricing

This actor is intended to be published with paid per-result pricing. Confirm active paid pricing or earliest allowed scheduled paid pricing in Apify before publication.

## Support

For higher-volume usage or direct API access, use Scrappa at https://scrappa.co.
