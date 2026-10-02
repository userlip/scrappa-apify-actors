# Instagram Trending Reels Scraper

Capture Instagram’s public trending Reels selection with captions, engagement counts, author handles, and video links. Each run records a current snapshot of the global feed.

## What data can you extract?

| Field | Type | Description |
| --- | --- | --- |
| `id` | String | Instagram media identifier for the trending Reel. |
| `shortcode` | String | Short code used in the public Reel URL. |
| `media_type` | String | Instagram format label for this trending item. |
| `caption` | String | Caption text displayed with the Reel. |
| `taken_at` | String | ISO 8601 time when the Reel was posted, when reported. |
| `taken_at_timestamp` | Integer | Unix timestamp when the Reel was posted. |
| `like_count` | Integer | Number of likes reported for the Reel. |
| `comment_count` | Integer | Number of comments reported for the Reel. |
| `play_count` | Integer or null | Number of video plays reported for the Reel, when available. |
| `has_audio` | Boolean | True when Instagram reports audio on the Reel. |
| `video_url` | String | URL of the Reel video asset, when available. |
| `thumbnail_url` | String | URL of the Reel preview image. |
| `author` | Object | Public profile details for the account that posted the Reel. |
| `permalink` | String | Public Instagram URL for the Reel. |
| `author_id` | String | Instagram identifier for the posting account. |
| `author_username` | String | Public Instagram handle of the posting account. |
| `author_verified` | String | True when Instagram marks the posting account as verified. |
| `input_feed` | String | Fixed feed label used to identify this global trending snapshot. |
| `scraped_at` | String | UTC date and time when this record was collected. |

## Use cases

- Social teams can review currently surfaced public Reels for creative patterns.
- Researchers can archive trending captions, creators, and engagement counts.
- Content strategists can compare trending video topics across repeated runs.

## How to use

1. Keep one `trending` entry in `feeds` to request the global feed.
2. Set `maxResults` to cap the number of Reel rows saved.
3. Schedule repeated runs if you want to compare public feed snapshots.

```json
{
  "feeds": [
    {
      "feed": "trending"
    }
  ],
  "maxResults": 10
}
```

## Input

| Field | Type | Required | Description |
| --- | --- | --- | --- |
| `feeds` | Array\<object\> | Yes | Use one entry to request Instagram’s global public trending Reels feed. |
| `feeds[].feed` | string | Yes per entry | Fixed label for the global trending Reels snapshot. |
| `maxResults` | integer | No | Maximum dataset items to save across this entire run. |

## Output example

```json
{
  "id": "1801234567890123456",
  "shortcode": "CSTtrendReel52",
  "media_type": "Video",
  "caption": "A quiet walk along the coast at golden hour.",
  "taken_at": "2025-09-18T14:32:11Z",
  "taken_at_timestamp": 1758205931,
  "like_count": 18400,
  "comment_count": 420,
  "play_count": 96800,
  "has_audio": true,
  "video_url": "https://cdn.instagram.com/media/coast-walk.mp4",
  "thumbnail_url": "https://cdn.instagram.com/media/coast-walk-cover.jpg",
  "author": {
    "id": "584700123456794",
    "username": "coastline.creator",
    "profile_pic_url": "https://cdn.instagram.com/profiles/coastline-creator.jpg",
    "is_verified": false
  },
  "permalink": "https://www.instagram.com/reel/CSTtrendReel52/",
  "author_id": "584700123456794",
  "author_username": "coastline.creator",
  "author_verified": false,
  "input_feed": "trending",
  "scraped_at": "2026-10-01T12:00:00Z"
}
```

## Pricing

$0.30 per 1,000 results. Apify saves one dataset item per result and applies the Actor’s per-result price to saved items.

Set **maxResults** to cap the number of dataset items saved in one run.

## FAQ

### Is scraping this information legal?

Rules depend on the source, location, data type, and intended use. Check applicable laws and source terms, and make sure your workflow follows privacy and data protection requirements.

### What limits apply?

Submit up to 100 batch entries per run. Use **maxResults** to cap saved items. Results also depend on source availability and your Scrappa API plan.

### Can I start runs through the Apify API?

Yes. Send a POST request to `https://api.apify.com/v2/acts/thescrappa~instagram-trending-reels-scraper/runs` with your Actor input, or use an Apify client library. Read the output from the run dataset.

### Can I connect the results to other tools?

Yes. Apify integrations and APIs can pass dataset results to Make, Zapier, n8n, Google Sheets, and other data workflows.

### What happens when one input fails?

The Actor logs a short source error and continues with the remaining entries. Transient rate limits and gateway errors are retried; if every entry fails, the run reports an error.

### Can I choose a country or hashtag?

No. This source feed returns the global trending selection and does not accept a region or search phrase.

## Related Scrappa Actors

- [Instagram User Reels Scraper](https://apify.com/thescrappa/instagram-user-reels-scraper)
- [Instagram User Posts Scraper](https://apify.com/thescrappa/instagram-user-posts-cheapest-0-20-1000-results)
- [TikTok Trending Feed Scraper](https://apify.com/thescrappa/tiktok-trending-feed-scraper)
