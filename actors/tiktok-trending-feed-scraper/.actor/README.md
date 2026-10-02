# TikTok Trending Feed Scraper

Capture TikTok’s public trending feed for a selected market with video captions, creator profiles, engagement totals, and sound information. Each row represents a trending post.

## What data can you extract?

| Field | Type | Description |
| --- | --- | --- |
| `aweme_id` | String | TikTok identifier for the video post. |
| `video_id` | String | Alternate video identifier returned with the post. |
| `region` | String | Market code associated with the feed result. |
| `title` | String | Short description or caption attached to the video. |
| `content_desc` | Array\<String\> | Content phrases associated with the post. |
| `cover` | String | URL of the default video cover image. |
| `ai_dynamic_cover` | String | URL of the animated or dynamic cover when supplied. |
| `origin_cover` | String | URL of the original cover image. |
| `duration` | Integer | Video length in seconds. |
| `play` | String | Playback reference returned for the video. |
| `wmplay` | String | Playback reference for a watermarked version, when supplied. |
| `size` | Integer | Reported size of the video media asset in bytes. |
| `wm_size` | Integer | Reported size of the watermarked media asset in bytes. |
| `music` | String | Music identifier or reference attached to the post. |
| `play_count` | Integer | Number of plays reported for the video. |
| `digg_count` | Integer | Number of likes reported for the video. |
| `comment_count` | Integer | Number of comments reported for the video. |
| `share_count` | Integer | Number of shares reported for the video. |
| `download_count` | Integer | Number of downloads reported for the video. |
| `collect_count` | Integer | Number of saves reported for the video. |
| `create_time` | Integer | Unix timestamp when the video was published. |
| `anchors` | Object or null | Commercial anchor data, or null when none is supplied. |
| `anchors_extras` | String | Serialized metadata for commercial anchors. |
| `is_ad` | Boolean | True when TikTok marks the post as an advertisement. |
| `commerce_info` | Object or null | Commerce and promotion flags, or null when none are supplied. |
| `commercial_video_info` | String | Commercial status information for the post. |
| `item_comment_settings` | Integer | Numeric comment-setting code returned for the video. |
| `mentioned_users` | String | Serialized user mentions attached to the video. |
| `author` | Object | Public TikTok profile details for the video author. |
| `is_nff_or_nr` | Boolean or null | TikTok content status flag, or null when not supplied. |
| `is_top` | Integer | Numeric flag indicating whether the video is pinned or top-ranked. |
| `music_info` | Object | Sound title, author, duration, and cover details for the video. |
| `images` | Array\<String\> | Image URLs associated with photo or carousel content. |
| `author_id` | String | TikTok profile identifier for the author. |
| `author_username` | String | Public TikTok handle of the video author. |
| `author_nickname` | String | Display name on the TikTok profile. |
| `author_avatar` | String | Public TikTok profile image URL. |
| `music_title` | String | Sound title associated with the TikTok video. |
| `music_author` | String | Sound creator name when music details are returned. |
| `input_region` | String | TikTok market code used to select trending videos. |
| `scraped_at` | String | UTC date and time when this record was collected. |

## Use cases

- Social analysts can review public videos gaining reach in a selected market.
- Content teams can compare trending captions, sounds, and engagement.
- Researchers can save regional TikTok feed snapshots for later analysis.

## How to use

1. Add a TikTok region code such as `GB` to `regions`.
2. Optionally set `count` to request up to 20 feed items.
3. Use `maxResults` to limit the number of video rows saved.

```json
{
  "regions": [
    {
      "region": "GB"
    }
  ],
  "count": 10,
  "maxResults": 20
}
```

## Input

| Field | Type | Required | Description |
| --- | --- | --- | --- |
| `regions` | Array\<object\> | Yes | TikTok market codes for the trending video feed. |
| `regions[].region` | string | Yes per entry | Region sent to the source search. |
| `regions[].count` | integer | No | Results Per Page sent to the source search. |
| `count` | integer | No | Results Per Page sent to the source search. |
| `maxResults` | integer | No | Maximum dataset items to save across this entire run. |

## Output example

```json
{
  "aweme_id": "7419000123456789012",
  "video_id": "7419000123456789012",
  "region": "GB",
  "title": "A simple guide to crisp sourdough at home",
  "content_desc": [
    "fresh sourdough",
    "home baking"
  ],
  "cover": "https://p16-sign.tiktokcdn.com/obj/tos-useast5-p-0068/loaf-cover.jpeg",
  "ai_dynamic_cover": "https://p16-sign.tiktokcdn.com/obj/tos-useast5-p-0068/loaf-dynamic.jpeg",
  "origin_cover": "https://p16-sign.tiktokcdn.com/obj/tos-useast5-p-0068/loaf-origin.jpeg",
  "duration": 36,
  "play": "https://v16-webapp.tiktok.com/video/loaf-play.mp4",
  "wmplay": "https://v16-webapp.tiktok.com/video/loaf-watermarked.mp4",
  "size": 4821032,
  "wm_size": 4938200,
  "music": "7418000123456789012",
  "play_count": 128000,
  "digg_count": 9800,
  "comment_count": 310,
  "share_count": 740,
  "download_count": 210,
  "collect_count": 1400,
  "create_time": 1758205931,
  "anchors": null,
  "anchors_extras": "[]",
  "is_ad": false,
  "commerce_info": {
    "auction_ad_invited": false,
    "with_comment_filter_words": false,
    "adv_promotable": false,
    "branded_content_type": 0,
    "organic_log_extra": "{}",
    "is_diversion_ad": 0
  },
  "commercial_video_info": "organic",
  "item_comment_settings": 0,
  "mentioned_users": "[]",
  "author": {
    "id": "7123456789012345678",
    "unique_id": "harborbakehouse",
    "nickname": "Harbor Bakehouse",
    "avatar": "https://p16-sign.tiktokcdn.com/tos-maliva-avt-0068/harborbakehouse.jpeg"
  },
  "is_nff_or_nr": false,
  "is_top": 0,
  "music_info": {
    "id": "7418000123456789012",
    "title": "Warm Morning",
    "play": "https://www.tiktok.com/music/warm-morning-7418000123456789012",
    "cover": "https://p16-sign.tiktokcdn.com/music/warm-morning.jpeg",
    "author": "North Pier Audio",
    "original": false,
    "duration": 42,
    "album": "Morning Light"
  },
  "images": [
    "https://p16-sign.tiktokcdn.com/obj/loaf-step-one.jpeg",
    "https://p16-sign.tiktokcdn.com/obj/loaf-step-two.jpeg"
  ],
  "author_id": "7123456789012345678",
  "author_username": "harborbakehouse",
  "author_nickname": "Harbor Bakehouse",
  "author_avatar": "https://p16-sign.tiktokcdn.com/tos-maliva-avt-0068/harborbakehouse.jpeg",
  "music_title": "Warm Morning",
  "music_author": "North Pier Audio",
  "input_region": "GB",
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

Yes. Send a POST request to `https://api.apify.com/v2/acts/thescrappa~tiktok-trending-feed-scraper/runs` with your Actor input, or use an Apify client library. Read the output from the run dataset.

### Can I connect the results to other tools?

Yes. Apify integrations and APIs can pass dataset results to Make, Zapier, n8n, Google Sheets, and other data workflows.

### What happens when one input fails?

The Actor logs a short source error and continues with the remaining entries. Transient rate limits and gateway errors are retried; if every entry fails, the run reports an error.

### Why does the US feed sometimes fail?

Feed availability can vary by market and source response. Choose another supported region if a market is temporarily unavailable.

## Related Scrappa Actors

- [TikTok Search Scraper](https://apify.com/thescrappa/tiktok-search-scraper)
- [TikTok Hashtag Posts Scraper](https://apify.com/thescrappa/tiktok-hashtag-posts-scraper)
- [Instagram Trending Reels Scraper](https://apify.com/thescrappa/instagram-trending-reels-scraper)
