# TikTok Following Scraper

Extract the public TikTok accounts a creator follows through Scrappa. Use it for audience research, influencer discovery, creator vetting, social graph analysis, competitive monitoring, and following-list sampling workflows.

## Features

- Lookup by TikTok username, full profile URL, or numeric user ID
- Fetch public followed accounts with profile and verification metadata
- Automatically paginate through Scrappa's `time` marker when more than 50 accounts are requested
- Dataset rows optimized for Apify table views
- Scrappa response or compact run summary saved to the `OUTPUT` key-value-store record

## Input

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `profile` | string | Yes | TikTok username with or without `@`, full profile URL, or numeric user ID. Bare numeric values are treated as user IDs; prefix numeric usernames with `@`. |
| `count` | integer | No | Maximum number of followed accounts to return. Accepts any positive integer; the actor fetches multiple Scrappa pages when needed. |
| `time` | integer | No | Following pagination token/time marker from a previous run. Leave empty for the first page. |

The actor also accepts `cursor` as a compatibility alias and sends it to Scrappa as `time`.

## Example Input

```json
{
  "profile": "@tiktok",
  "count": 10
}
```

## Output

Each followed TikTok account is saved as one dataset item:

```json
{
  "user_id": "107955",
  "unique_id": "tiktok",
  "nickname": "TikTok",
  "avatar": "https://example.com/avatar.jpeg",
  "follower_count": 162300000,
  "verified": true,
  "lookup_unique_id": "@tiktok",
  "lookup_user_id": null
}
```

For requests up to 50 accounts, the full API response is saved to `OUTPUT`. For larger requests, `OUTPUT` contains a compact run summary with pagination metadata.

## Support

For higher-volume usage or direct API access, use Scrappa at https://scrappa.co.
