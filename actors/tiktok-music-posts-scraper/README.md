# TikTok Music Posts Scraper

TikTok Music Posts Scraper is a Rust Apify actor that calls Scrappa's TikTok music posts endpoint. It extracts public TikTok videos that use specific music tracks or sounds. Use it for TikTok sound videos, TikTok music track posts, trend monitoring, creator discovery, campaign research, and content intelligence workflows.

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

The actor checks the Apify run's pay-per-event prices and spending limit before writing result rows. It stops fetching additional music IDs when the run cannot charge another dataset item.

Apify storage and run API requests retry network errors, HTTP 429, and server errors up to eight times with exponential backoff. Scrappa API requests keep the existing single-attempt behavior and 60-second timeout; the actor run timeout remains 120 seconds.

## Local development

Run the focused Rust tests from this directory with cargo test --locked.

Build the Apify image with docker build -f .actor/Dockerfile -t tiktok-music-posts-scraper .

After building, run the local image smoke against a mock Apify and Scrappa server with python3 test/image-smoke.py tiktok-music-posts-scraper.

## Support

For higher-volume usage or direct API access, use Scrappa at https://scrappa.co.
