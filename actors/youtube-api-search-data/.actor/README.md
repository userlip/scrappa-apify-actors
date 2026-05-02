# YouTube API Search Data

Search YouTube for videos, channels, playlists, movies, live streams, and filtered result sets. The Actor returns one dataset item per YouTube search result, so you can export results to JSON, CSV, Excel, or connect them to downstream Apify integrations.

## What You Can Get

- YouTube video, channel, playlist, movie, and mixed search results
- Titles, descriptions, thumbnails, result IDs, durations, view counts, and publish times
- Channel metadata including channel ID, name, URL, thumbnail, and verification status
- Badges such as `CC`, `New`, `4K`, or other labels returned by YouTube
- Live, Shorts, and Premium flags when available
- Chapter or extra result metadata when YouTube returns expandable result details

## Input

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `q` | string | Yes | YouTube search query. |
| `sort` | array | No | Sort order. One of `relevance`, `rating`, `upload_date`, or `view_count`. Defaults to `relevance`. |
| `duration` | array | No | Video duration filter. One of `short`, `medium`, or `long`. |
| `upload_date` | array | No | Upload date filter. One of `hour`, `today`, `week`, `month`, or `year`. The actor maps this to Scrappa `publishedAfter`. |
| `limit` | integer | No | Maximum results to return, from `1` to `20`. |
| `continuation` | string | No | Continuation token for the next page of results. |
| `type` | array | No | Result type. One of `all`, `video`, `channel`, `playlist`, or `movie`. |

Apify select fields are arrays in JSON input. Use a single selected value in each array, for example `"type": ["video"]`.

## Example Inputs

### Recent Tutorial Videos

```json
{
  "q": "javascript tutorial",
  "type": ["video"],
  "sort": ["upload_date"],
  "upload_date": ["week"],
  "duration": ["medium"],
  "limit": 20
}
```

### High-View Product Reviews

```json
{
  "q": "iphone 16 pro review",
  "type": ["video"],
  "sort": ["view_count"],
  "limit": 20
}
```

### AI News Videos

```json
{
  "q": "ai news live",
  "type": ["video"],
  "limit": 20
}
```

### Channels Matching a Query

```json
{
  "q": "web development",
  "type": ["channel"],
  "sort": ["relevance"],
  "limit": 20
}
```

### Continue to the Next Page

```json
{
  "q": "javascript tutorial",
  "type": ["video"],
  "sort": ["relevance"],
  "limit": 20,
  "continuation": "PASTE_CONTINUATION_TOKEN_FROM_PREVIOUS_RUN"
}
```

When the API returns a next-page token, the Actor logs it as `Continuation token available for next page: ...`.

## Output

Each YouTube search result is saved as one dataset item. Fields vary by result type, but video results commonly include:

```json
{
  "type": "Video",
  "id": "W6NZfCO5SIk",
  "title": "JavaScript Course for Beginners - Your First Step to Web Development",
  "description": "Learn JavaScript basics with this quick, beginner-friendly course...",
  "thumbnail": "https://i.ytimg.com/vi/W6NZfCO5SIk/hq720.jpg",
  "duration": "48:17",
  "viewCount": "14,962,240 views",
  "publishedTime": "8 years ago",
  "channel": {
    "id": "UCWv7vMbMWH4-V0ZXdmDpPBA",
    "name": "Programming with Mosh",
    "thumbnail": "https://yt3.ggpht.com/...",
    "verified": true,
    "isVerifiedArtist": false,
    "url": "https://www.youtube.com/@programmingwithmosh"
  },
  "badges": ["CC"],
  "isLive": false,
  "isShort": false,
  "isPremium": false,
  "expandableMetadata": null
}
```

## Dataset Columns

The default dataset view highlights the most useful columns for scanning and exporting search results:

| Column | Description |
|--------|-------------|
| `type` | Result type, such as `Video`, `Channel`, or `Playlist`. |
| `id` | YouTube result ID, such as a video ID or channel ID. |
| `title` | Result title. |
| `channel.name` | Channel name for video results. |
| `channel.url` | Channel URL when provided by YouTube. |
| `duration` | Video duration when available. |
| `viewCount` | Display view count text returned by YouTube. |
| `publishedTime` | Relative publish time returned by YouTube. |
| `thumbnail` | Result thumbnail image URL. |
| `badges` | Search result badges, labels, or availability markers. |
| `isLive` | Whether the result is live. |
| `isShort` | Whether the result is a Short. |
| `isPremium` | Whether the result is marked as Premium. |

## Pricing

$0.30 per 1,000 results. No additional API keys required.

## Support

For issues or questions, contact us through Apify.
