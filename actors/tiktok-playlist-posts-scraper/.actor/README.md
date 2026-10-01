# TikTok Playlist Videos Scraper

Collect TikTok playlist videos with captions, author profiles, media links, engagement counts, and sound references. Batch playlist IDs and follow source cursors within your limits.

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
| `anchors` | String or null | Commercial anchor data, or null when none is supplied. |
| `anchors_extras` | String | Serialized metadata for commercial anchors. |
| `is_ad` | Boolean | True when TikTok marks the post as an advertisement. |
| `commerce_info` | Object | Commerce and promotion flags returned for the post. |
| `commercial_video_info` | String | Commercial status information for the post. |
| `item_comment_settings` | Integer | Numeric comment-setting code returned for the video. |
| `mentioned_users` | String | Serialized user mentions attached to the video. |
| `author` | Object | Public TikTok profile details for the video author. |
| `is_nff_or_nr` | Boolean | TikTok content status flag, or null when not supplied. |
| `is_top` | Integer | Numeric flag indicating whether the video is pinned or top-ranked. |
| `author_id` | String | TikTok profile identifier for the author. |
| `author_username` | String | Public TikTok handle of the video author. |
| `author_nickname` | String | Display name on the TikTok profile. |
| `author_avatar` | String | Public TikTok profile image URL. |
| `input_mix_id` | String | TikTok playlist identifier used to retrieve its videos. |
| `scraped_at` | String | UTC date and time when this record was collected. |

## Use cases

- Music researchers can review videos grouped in public TikTok playlists.
- Social analysts can compare creator and engagement data across playlists.
- Content teams can export video links and captions from curated lists.

## How to use

1. Add each TikTok playlist ID to `playlist_ids`.
2. Set `count` for the number of videos requested per source page.
3. Choose `maxPages` and `maxResults` to limit each run.

```json
{
  "playlist_ids": [
    {
      "mix_id": "7681171537575824159"
    }
  ],
  "count": 10,
  "maxResults": 20,
  "maxPages": 1
}
```

## Input

| Field | Type | Required | Description |
| --- | --- | --- | --- |
| `playlist_ids` | Array\<object\> | Yes | TikTok playlist IDs whose videos you want to collect. |
| `playlist_ids[].mix_id` | string | Yes per entry | Playlist ID sent to the source search. |
| `playlist_ids[].count` | integer | No | Results Per Page sent to the source search. |
| `playlist_ids[].cursor` | string | No | Pagination Cursor sent to the source search. |
| `count` | integer | No | Results Per Page sent to the source search. |
| `cursor` | string | No | Pagination Cursor sent to the source search. |
| `maxResults` | integer | No | Maximum dataset items to save across this entire run. |
| `maxPages` | integer | No | Maximum pages to request for each batch entry. |

## Output example

```json
{
  "aweme_id": "7419000123456789012",
  "video_id": "7419000123456789012",
  "region": "GB",
  "title": "A simple guide to crisp sourdough at home",
  "content_desc": [
    "field recording",
    "ambient music"
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
  "author_id": "7123456789012345678",
  "author_username": "harborbakehouse",
  "author_nickname": "Harbor Bakehouse",
  "author_avatar": "https://p16-sign.tiktokcdn.com/tos-maliva-avt-0068/harborbakehouse.jpeg",
  "input_mix_id": "7681171537575824159",
  "scraped_at": "2026-10-01T12:00:00Z"
}
```

## Pricing

$0.30 per 1,000 results. Apify saves one dataset item per result and applies the Actor’s per-result price to saved items.

Pagination follows the source response. Set **maxPages** per batch entry and **maxResults** across the run.

## FAQ

### Is scraping this information legal?

Rules depend on the source, location, data type, and intended use. Check applicable laws and source terms, and make sure your workflow follows privacy and data protection requirements.

### What limits apply?

Submit up to 100 batch entries per run. Use **maxResults** to cap saved items and **maxPages** to limit pages for each entry. Results also depend on source availability and your Scrappa API plan.

### Can I start runs through the Apify API?

Yes. Send a POST request to `https://api.apify.com/v2/acts/thescrappa~tiktok-playlist-posts-scraper/runs` with your Actor input, or use an Apify client library. Read the output from the run dataset.

### Can I connect the results to other tools?

Yes. Apify integrations and APIs can pass dataset results to Make, Zapier, n8n, Google Sheets, and other data workflows.

### What happens when one input fails?

The Actor logs a short source error and continues with the remaining entries. Transient rate limits and gateway errors are retried; if every entry fails, the run reports an error.

### How is a playlist ID different from a collection ID?

A playlist groups videos through TikTok’s playlist feature, while a collection is a saved set of videos. Use the matching ID and Actor for the URL type.

## Related Scrappa Actors

- [TikTok User Posts Scraper](https://apify.com/thescrappa/tiktok-user-posts-scraper)
- [TikTok Music Posts Scraper](https://apify.com/thescrappa/tiktok-music-posts-scraper)
- [TikTok Video Details Scraper](https://apify.com/thescrappa/tiktok-video-scraper)
