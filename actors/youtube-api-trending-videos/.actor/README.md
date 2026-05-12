# YouTube Trending Videos

Get YouTube trending videos by category. The Actor returns one dataset item per video, so results can be exported to JSON, CSV, Excel, or connected to Apify integrations.

## What You Can Get

- Trending YouTube video IDs, titles, descriptions, thumbnails, durations, view counts, and publish times
- Channel metadata including channel ID, name, URL, thumbnail, and verification status
- Badges such as `LIVE`, `4K`, `CC`, `New`, or other labels returned by YouTube
- Live, Shorts, and Premium flags when available
- Pagination metadata is logged when the Scrappa endpoint returns a continuation token

## Input

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `category` | array | No | Trending category. One of `education`, `music`, `gaming`, `news`, `sports`, `entertainment`, `howto`, `tech`, or `science`. |
| `type` | array | No | Trending type. Currently `now`. |

Apify select fields are arrays in JSON input. Use a single selected value in each array, for example `"category": ["music"]`.

## Example Input

```json
{
  "category": ["music"],
  "type": ["now"]
}
```

## Output

Each trending video is saved as one dataset item:

```json
{
  "type": "Video",
  "id": "k2qgadSvNyU",
  "title": "Dua Lipa - New Rules (Official Music Video)",
  "description": "The official music video for Dua Lipa - New Rules...",
  "thumbnail": "https://i.ytimg.com/vi/k2qgadSvNyU/hqdefault.jpg",
  "duration": "3:45",
  "viewCount": "3,276,277,203 views",
  "publishedTime": "8 years ago",
  "channel": {
    "id": "UC-J-KZfRV8c13fOCkhXdLiQ",
    "name": "Dua Lipa",
    "thumbnail": "https://yt3.ggpht.com/...",
    "verified": false,
    "isVerifiedArtist": true,
    "url": "https://www.youtube.com/channel/UC-J-KZfRV8c13fOCkhXdLiQ"
  },
  "badges": ["4K"],
  "isLive": false,
  "isShort": false,
  "isPremium": false,
  "expandableMetadata": null
}
```

## Pricing

$0.30 per 1,000 results. No additional API keys required.

## Support

For issues or questions, contact us through Apify.
