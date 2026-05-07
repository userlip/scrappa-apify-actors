# TikTok Following Scraper

Extract the public TikTok accounts a creator follows through Scrappa. Use it for audience research, influencer discovery, creator vetting, social graph analysis, competitive monitoring, and following-list sampling workflows.

## Features

- Lookup by TikTok username, full profile URL, or numeric user ID
- Fetch a page of public followed accounts with profile and verification metadata
- Support pagination via Scrappa's `time` marker
- Dataset rows optimized for Apify table views
- Full Scrappa response saved to the `OUTPUT` key-value-store record

## Input

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `profile` | string | Yes | TikTok username with or without `@`, full profile URL, or numeric user ID. Bare numeric values are treated as user IDs; prefix numeric usernames with `@`. |
| `count` | integer | No | Number of followed accounts to return. Scrappa accepts `1-50`. |
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

The full API response, including pagination metadata, is saved to `OUTPUT`.

## Support

For higher-volume usage or direct API access, use Scrappa at https://scrappa.co.
