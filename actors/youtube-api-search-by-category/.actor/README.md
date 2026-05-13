# YouTube Search By Category Scraper

Get YouTube search results from predefined category searches such as education, music, gaming, news, sports, and technology. The Actor returns one dataset item per YouTube result, so you can export videos to JSON, CSV, Excel, or connect them to downstream Apify integrations.

## What You Can Get

- Category-based YouTube video results
- Titles, descriptions, thumbnails, video IDs, durations, view counts, and publish times
- Channel metadata including channel ID, name, URL, thumbnail, and verification status
- Badges such as `CC`, `New`, `4K`, or other labels returned by YouTube
- Live, Shorts, and Premium flags when available
- Continuation tokens in logs when another page is available

## Input

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `category` | array | Yes | Category to search in. Examples: `education`, `music`, `gaming`, `news`, `sports`, `tech`. |
| `sort` | array | No | Sort order. One of `relevance`, `rating`, `upload_date`, or `view_count`. Defaults to `relevance`. |
| `duration` | array | No | Video duration filter. One of `short`, `medium`, or `long`. |
| `upload_date` | array | No | Upload date filter. One of `hour`, `today`, `week`, `month`, or `year`. |
| `limit` | integer | No | Maximum results to request, from `1` to `1024`. The upstream API can return a smaller page with a continuation token. |
| `continuation` | string | No | Continuation token for the next page of results. |
| `contentType` | array | No | Filter by content type: `live`, `recorded`, or `premiere`. |
| `features` | string | No | Comma-separated feature filters such as `hd,cc`. |

Apify select fields are arrays in JSON input. Use a single selected value in each array, for example `"category": ["education"]`.

## Example Inputs

### Education Videos

```json
{
  "category": ["education"],
  "sort": ["relevance"],
  "duration": ["short"],
  "limit": 10
}
```

### High-View Music Videos

```json
{
  "category": ["music"],
  "sort": ["view_count"],
  "limit": 20
}
```

### Recent Tech Videos

```json
{
  "category": ["tech"],
  "sort": ["upload_date"],
  "upload_date": ["week"],
  "limit": 20
}
```

### Continue to the Next Page

```json
{
  "category": ["education"],
  "sort": ["relevance"],
  "limit": 20,
  "continuation": "PASTE_CONTINUATION_TOKEN_FROM_PREVIOUS_RUN"
}
```

When the API returns a next-page token, the Actor logs it as `Continuation token available for next page: ...`.

## Output

Each result is saved as one dataset item. Fields vary by YouTube result, but video results commonly include:

```json
{
  "type": "Video",
  "id": "KVLTxKyxioA",
  "title": "The Science of Teaching, Effective Education, and Great Schools",
  "description": "Scientific evidence suggests that the secret to thriving students...",
  "thumbnail": "https://i.ytimg.com/vi/KVLTxKyxioA/hq720.jpg",
  "duration": "6:21",
  "viewCount": "824,846 views",
  "publishedTime": "8 years ago",
  "channel": {
    "id": "UC-RKpEc4eE9PwJaupN91xYQ",
    "name": "Sprouts",
    "thumbnail": "https://yt3.ggpht.com/...",
    "verified": true,
    "isVerifiedArtist": false,
    "url": "https://www.youtube.com/@sprouts"
  },
  "badges": ["CC"],
  "isLive": false,
  "isShort": false,
  "isPremium": false,
  "expandableMetadata": {
    "label": "Summary"
  }
}
```

## Pricing

$0.30 per 1,000 results. No additional API keys required.

## Support

For issues or questions, contact us through Apify.
