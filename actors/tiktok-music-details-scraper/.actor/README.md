# TikTok Music Details Scraper

Look up public TikTok music pages and save the sound title, author, duration, cover, album, and linked video count. Submit multiple sound URLs in one run.

## What data can you extract?

| Field | Type | Description |
| --- | --- | --- |
| `code` | Integer | TikTok response status code; zero indicates a successful lookup. |
| `msg` | String | Status message returned with the music detail record. |
| `processed_time` | Number | Source processing time in seconds. |
| `data` | Object | Sound identifier, title, author, duration, cover, and associated video count. |
| `music_id` | String | TikTok music identifier returned for the sound. |
| `music_title` | String | Title displayed for the TikTok sound. |
| `music_play` | String | Public TikTok URL for the sound page. |
| `music_cover` | String | Cover image URL associated with the sound. |
| `music_author` | String | Sound author or creator name returned by TikTok. |
| `music_duration` | String | Sound duration in seconds. |
| `music_album` | String | Album name when TikTok reports one. |
| `video_count` | String | Number of videos associated with the sound. |
| `input_url` | String | TikTok sound URL submitted to retrieve music details. |
| `scraped_at` | String | UTC date and time when this record was collected. |

## Use cases

- Music teams can check details and linked video counts for TikTok sounds.
- Social analysts can connect popular audio to its public TikTok music page.
- Content researchers can organize sound metadata from a list of URLs.

## How to use

1. Add each public TikTok music URL to `urls`.
2. Set `maxResults` to cap one record per submitted sound.
3. Read the nested `data` object or use the flattened sound columns.

```json
{
  "urls": [
    {
      "url": "https://www.tiktok.com/music/original-sound-6689804660171082501"
    }
  ],
  "maxResults": 20
}
```

## Input

| Field | Type | Required | Description |
| --- | --- | --- | --- |
| `urls` | Array\<object\> | Yes | Public TikTok music URLs to look up. |
| `urls[].url` | string | Yes per entry | TikTok Music URL sent to the source search. |
| `maxResults` | integer | No | Maximum dataset items to save across this entire run. |

## Output example

```json
{
  "code": 0,
  "msg": "success",
  "processed_time": 0.5,
  "data": {
    "id": "6689804660171082501",
    "title": "Morning Table",
    "play": "https://www.tiktok.com/music/morning-table-6689804660171082501",
    "cover": "https://p16-sign.tiktokcdn.com/music/morning-table.jpeg",
    "author": "Harbor Sound",
    "original": true,
    "duration": 31,
    "album": "Morning Light",
    "video_count": 245
  },
  "music_id": "6689804660171082501",
  "music_title": "Morning Table",
  "music_play": "https://www.tiktok.com/music/morning-table-6689804660171082501",
  "music_cover": "https://p16-sign.tiktokcdn.com/music/morning-table.jpeg",
  "music_author": "Harbor Sound",
  "music_duration": 31,
  "music_album": "Morning Light",
  "video_count": 245,
  "input_url": "https://www.tiktok.com/music/original-sound-6689804660171082501",
  "scraped_at": "2026-10-01T12:00:00Z"
}
```

## Pricing

$0.50 per 1,000 results. Apify saves one dataset item per result and applies the Actor’s per-result price to saved items.

Set **maxResults** to cap the number of dataset items saved in one run.

## FAQ

### Is scraping this information legal?

Rules depend on the source, location, data type, and intended use. Check applicable laws and source terms, and make sure your workflow follows privacy and data protection requirements.

### What limits apply?

Submit up to 100 batch entries per run. Use **maxResults** to cap saved items. Results also depend on source availability and your Scrappa API plan.

### Can I start runs through the Apify API?

Yes. Send a POST request to `https://api.apify.com/v2/acts/thescrappa~tiktok-music-details-scraper/runs` with your Actor input, or use an Apify client library. Read the output from the run dataset.

### Can I connect the results to other tools?

Yes. Apify integrations and APIs can pass dataset results to Make, Zapier, n8n, Google Sheets, and other data workflows.

### What happens when one input fails?

The Actor logs a short source error and continues with the remaining entries. Transient rate limits and gateway errors are retried; if every entry fails, the run reports an error.

### Can I pass a video URL instead of a music URL?

Use the public TikTok music page URL for this Actor. Video URLs belong in a video details workflow.

## Related Scrappa Actors

- [TikTok Music Posts Scraper](https://apify.com/thescrappa/tiktok-music-posts-scraper)
- [TikTok Video Details Scraper](https://apify.com/thescrappa/tiktok-video-scraper)
- [TikTok User Posts Scraper](https://apify.com/thescrappa/tiktok-user-posts-scraper)
