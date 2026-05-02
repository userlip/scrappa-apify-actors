# TikTok Profile Scraper

Extract public TikTok profile data through Scrappa. Use it for creator vetting, influencer discovery, audience research, competitive monitoring, and account enrichment workflows.

## Features

- Lookup by TikTok username, full profile URL, or numeric user ID
- Public profile fields including bio, avatar, verification, region, and language
- Audience and engagement metrics including followers, following, likes, videos, and diggs
- Dataset rows optimized for Apify table views
- Full Scrappa response saved to the `OUTPUT` key-value-store record

## Input

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `unique_id` | string | No | TikTok username with or without `@`, or a full profile URL |
| `user_id` | string | No | Numeric TikTok user ID |

Provide at least one of `unique_id` or `user_id`.

## Example Input

```json
{
  "unique_id": "@tiktok"
}
```

## Output

Each TikTok profile is saved as one dataset item:

```json
{
  "user_id": "107955",
  "unique_id": "tiktok",
  "nickname": "TikTok",
  "avatar": "https://...",
  "signature": "The official TikTok account",
  "follower_count": 90000000,
  "following_count": 0,
  "heart_count": 350000000,
  "video_count": 1000,
  "digg_count": 0,
  "verified": true,
  "private_account": false,
  "region": "US",
  "language": "en"
}
```

The full API response is saved to `OUTPUT`.

## Support

For higher-volume usage or direct API access, use Scrappa at https://scrappa.co.
