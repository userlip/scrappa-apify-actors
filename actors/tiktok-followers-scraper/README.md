# TikTok Followers Scraper

Extract public TikTok follower lists for a creator through Scrappa. Use it for audience research, influencer discovery, creator vetting, social graph analysis, and follower sampling workflows.

## Features

- Lookup by TikTok username, full profile URL, or numeric user ID
- Resolve username inputs to the numeric TikTok `user_id` required by the followers endpoint
- Fetch a page of public followers with profile and verification metadata
- Support pagination via Scrappa's `time` marker
- Dataset rows optimized for Apify table views
- Full Scrappa response saved to the `OUTPUT` key-value-store record

## Input

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `profile` | string | Yes | TikTok username with or without `@`, full profile URL, or numeric user ID. Bare numeric values are treated as user IDs; prefix numeric usernames with `@`. |
| `count` | integer | No | Number of followers to return. Scrappa accepts `1-50`. |
| `time` | integer | No | Follower pagination token/time marker from a previous run. Leave empty for the first page. |

The actor also accepts `cursor` as a compatibility alias and sends it to Scrappa as `time`.

## Example Input

```json
{
  "profile": "@tiktok",
  "count": 10
}
```

## Output

Each TikTok follower is saved as one dataset item:

```json
{
  "user_id": "107955",
  "unique_id": "tiktok",
  "nickname": "TikTok",
  "avatar": "https://example.com/avatar.jpeg",
  "follower_count": 162300000,
  "verified": true,
  "lookup_unique_id": "@tiktok",
  "lookup_user_id": "107955"
}
```

The full API response, including pagination metadata, is saved to `OUTPUT`.

## Runtime

The Rust actor calls Scrappa's `/tiktok/user/profile` endpoint to resolve usernames, then calls `/tiktok/user/followers` with the numeric user ID. Each Scrappa request uses the `X-API-Key` header and a 60-second timeout. The actor does not retry failed upstream requests; the configured Apify run deadline is 120 seconds.

Each returned follower is written as one default dataset item. For pay-per-event runs with a positive `maxTotalChargeUsd`, the actor reads the event prices and charged event counts and saves only the rows affordable under that cap. Other pricing models and uncapped runs (zero, null, or absent `maxTotalChargeUsd`) save every follower. Apify automatically charges its `apify-default-dataset-item` event for each saved row. `OUTPUT` always keeps the complete Scrappa response, including followers beyond a positive run cap.

The Rust tests use loopback mocks and a dummy API token. Run them from this directory with `cargo test --locked`. Build the local image from this directory with `docker build -f .actor/Dockerfile -t tiktok-followers-scraper:local .`.

## Support

For higher-volume usage or direct API access, use Scrappa at https://scrappa.co.
