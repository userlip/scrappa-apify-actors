# Instagram User Posts Scraper

Extract recent public Instagram posts and reels by username. This Actor is built for creator research, content analysis, competitor monitoring, brand tracking, and social media reporting workflows that need profile post collections rather than single post details.

No Instagram login, cookies, proxy setup, or browser session is required. Provide a username and the Actor returns available public posts to an Apify dataset. The Actor uses a Scrappa API key configured as the `SCRAPPA_API_KEY` environment variable; set this secret if you fork or self-deploy the Actor.

## What It Does

- Fetches recent public posts and reels from an Instagram user profile.
- Returns post fields such as shortcode, media type, caption, timestamp, likes, comments, plays, media URLs, location, author, and permalink when available.
- Supports pagination with `max_id`, using the previous response's `next_max_id` value.
- Saves the full upstream response as `OUTPUT` in the default key-value store for access to pagination metadata, including `more_available` and `next_max_id`, plus raw fields.

Private accounts can still return public metadata that is visible without logging in, but this Actor does not bypass privacy restrictions or access private posts.

## Input

Use one Instagram username per run.

### Input Fields

| Field | Type | Required | Description |
| --- | --- | --- | --- |
| `username` | String | Yes | Instagram username to fetch posts for. Use a handle such as `natgeo`; an optional leading `@` is normalized automatically. |
| `max_id` | String | No | Pagination cursor from a previous response's `next_max_id` field. |

## Example Input

```json
{
  "username": "natgeo"
}
```

Next page:

```json
{
  "username": "natgeo",
  "max_id": "QVFDcF..."
}
```

## Output

Each run saves one dataset item per returned post. If Scrappa returns zero posts, the dataset remains empty and the full response is still available in the default key-value store as `OUTPUT`. The exact fields can vary depending on what Instagram returns, but common fields include:

| Field | Description |
| --- | --- |
| `request_username` | Username requested in the Actor input. |
| `id` | Instagram post ID. |
| `shortcode` | Instagram shortcode. |
| `media_type` | Post media type, such as image, video, carousel, or reel when returned. |
| `caption` | Caption text. |
| `taken_at` | Post timestamp. |
| `like_count` | Number of likes. |
| `comment_count` | Number of comments. |
| `play_count` | Number of video plays, when available. |
| `media` | Media objects such as thumbnails, images, or videos. |
| `location` | Location metadata, when available. |
| `author` | Author metadata. |
| `permalink` | Instagram post permalink. |

## Example Output

```json
{
  "request_username": "natgeo",
  "id": "3819535222330010870",
  "shortcode": "DUBtwxGEqz2",
  "media_type": "video",
  "caption": "Post caption text...",
  "taken_at": "2024-01-15T10:30:00+00:00",
  "like_count": 125000,
  "comment_count": 3500,
  "play_count": 5000000,
  "media": [
    {
      "type": "video",
      "thumbnail_url": "https://...",
      "video_url": "https://..."
    }
  ],
  "author": {
    "username": "natgeo"
  },
  "permalink": "https://www.instagram.com/natgeo/p/DUBtwxGEqz2/"
}
```

## Exporting Results

Results are stored in the Actor's default dataset. From Apify, you can export the dataset as JSON, CSV, Excel, XML, RSS, or HTML table. Use JSON for nested media fields, and CSV or Excel for spreadsheet-friendly post analysis.

## Authentication

This Actor does not require an Instagram account. It does not ask for Instagram credentials, session cookies, or two-factor authentication codes.

## Common Use Cases

- Collect recent posts from public creators, brands, publishers, or competitors.
- Build influencer content and engagement reports.
- Monitor reels and post performance over time.
- Export post collections for BI, CRM, or spreadsheet workflows.
- Feed downstream post-detail enrichment workflows with shortcodes or permalinks.

## Recommended Workflow

1. Run this Actor with a public Instagram username.
2. Export the dataset for the returned page of posts.
3. Open the `OUTPUT` key-value store record. If `more_available` is true, run again with `max_id` set to the returned `next_max_id`.
4. Use the exported post URLs or shortcodes with the Instagram Post Info Actor when you need deeper single-post details.

## Pricing

$0.20 per 1,000 results. No Instagram login required. Requires `SCRAPPA_API_KEY` in the Actor environment.

## Notes And Limits

- Run one username per Actor run.
- Usernames are normalized before lookup, so a leading `@` is removed automatically.
- Availability of fields depends on the public data returned by Instagram and Scrappa.
- This Actor does not access private posts or bypass account privacy settings.
