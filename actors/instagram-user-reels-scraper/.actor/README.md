# Instagram User Reels Scraper

Collect public Reels from Instagram accounts with captions, engagement counts, posting times, author details, and media links. Add usernames in one run and page through available results.

## What data can you extract?

| Field | Type | Description |
| --- | --- | --- |
| `id` | Integer | Numeric Instagram media identifier for the Reel. |
| `shortcode` | String | Short code used in the public Instagram Reel URL. |
| `media_type` | String | Instagram media type label for the Reel. |
| `caption` | String | Caption text published with the Reel. |
| `hashtags` | Array\<String\> | Hashtags extracted from the Reel caption. |
| `taken_at` | String | ISO 8601 time when the Reel was posted, when reported. |
| `taken_at_timestamp` | Integer | Unix timestamp when the Reel was posted. |
| `like_count` | Integer | Number of likes reported for the Reel. |
| `comment_count` | Integer | Number of comments reported for the Reel. |
| `play_count` | Integer | Number of Reel plays reported by Instagram, when available. |
| `view_count` | Integer | Number of Reel views reported by Instagram, when available. |
| `media` | Array\<Object\> | Media assets associated with the Reel, including video and image URLs. |
| `location` | Object or null | Location attached to the Reel, or null when none is reported. |
| `author` | Object | Public profile details for the account that posted the Reel. |
| `collaborators` | Array\<Object\> | Public accounts credited as Reel collaborators, when present. |
| `tagged_users` | Array\<Object\> | Public accounts tagged in the Reel, when present. |
| `is_paid_partnership` | Boolean | True when Instagram marks the Reel as a paid partnership. |
| `product_type` | String | Instagram publishing format for this Reel. |
| `permalink` | String | Public Instagram URL for the Reel. |
| `author_id` | String | Instagram identifier for the Reel author. |
| `author_username` | String or null | Public handle of the account that posted the Reel. |
| `author_verified` | String | True when Instagram marks the Reel author as verified. |
| `input_username` | String | Instagram username used to retrieve this account’s posts. |
| `scraped_at` | String | UTC date and time when this record was collected. |

## Use cases

- Social analysts can compare Reel views and engagement across public accounts.
- Creator managers can export recent video links and captions by username.
- Researchers can track public Instagram video publishing activity over time.

## How to use

1. Add one public Instagram username to `usernames` for each account.
2. Set `maxPages` to bound cursor pages requested per account.
3. Use `maxResults` to limit the combined dataset size.

```json
{
  "usernames": [
    {
      "username": "nasa"
    }
  ],
  "maxResults": 20,
  "maxPages": 1
}
```

## Input

| Field | Type | Required | Description |
| --- | --- | --- | --- |
| `usernames` | Array\<object\> | Yes | Public Instagram usernames whose Reels you want to collect. |
| `usernames[].username` | string | Yes per entry | Instagram Username sent to the source search. |
| `usernames[].user_id` | string | No | TikTok User ID sent to the source search. |
| `usernames[].max_id` | string | No | Next Page Cursor sent to the source search. |
| `user_id` | string | No | TikTok User ID sent to the source search. |
| `max_id` | string | No | Next Page Cursor sent to the source search. |
| `maxResults` | integer | No | Maximum dataset items to save across this entire run. |
| `maxPages` | integer | No | Maximum pages to request for each batch entry. |

## Output example

```json
{
  "id": 3391234567890123300,
  "shortcode": "CSTbakeReel91",
  "media_type": "Video",
  "caption": "A slow morning and a fresh loaf from the oven.",
  "hashtags": [
    "sourdough",
    "homebaking"
  ],
  "taken_at": "2025-09-18T14:32:11Z",
  "taken_at_timestamp": 1758205931,
  "like_count": 2580,
  "comment_count": 86,
  "play_count": 38200,
  "view_count": 37400,
  "media": [
    {
      "type": "video",
      "images": [
        {
          "url": "https://cdn.instagram.com/media/loaf-cover.jpg",
          "width": 720,
          "height": 1280
        }
      ],
      "thumbnail_url": "https://cdn.instagram.com/media/loaf-thumb.jpg",
      "video_url": "https://cdn.instagram.com/media/loaf-reel.mp4",
      "video_versions": [
        {
          "url": "https://cdn.instagram.com/media/loaf-reel.mp4",
          "width": 720,
          "height": 1280,
          "type": 101
        }
      ],
      "video_duration": 17.4
    }
  ],
  "location": null,
  "author": {
    "id": 584700123456789,
    "username": "harbor.bakes",
    "full_name": "Harbor Bakehouse",
    "profile_pic_url": "https://cdn.instagram.com/profiles/harbor-bakes.jpg",
    "is_verified": false
  },
  "collaborators": [
    {
      "id": "584700123456790",
      "username": "breadclub"
    }
  ],
  "tagged_users": [
    {
      "id": "584700123456791",
      "username": "coffeeandcrumbs"
    }
  ],
  "is_paid_partnership": false,
  "product_type": "clips",
  "permalink": "https://www.instagram.com/reel/CSTbakeReel91/",
  "author_id": 584700123456789,
  "author_username": "harbor.bakes",
  "author_verified": false,
  "input_username": "nasa",
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

Yes. Send a POST request to `https://api.apify.com/v2/acts/thescrappa~instagram-user-reels-scraper/runs` with your Actor input, or use an Apify client library. Read the output from the run dataset.

### Can I connect the results to other tools?

Yes. Apify integrations and APIs can pass dataset results to Make, Zapier, n8n, Google Sheets, and other data workflows.

### What happens when one input fails?

The Actor logs a short source error and continues with the remaining entries. Transient rate limits and gateway errors are retried; if every entry fails, the run reports an error.

### Can I search by numeric user ID instead?

Yes. You can provide `user_id` as an optional input, while each batch entry still needs a username for the account label.

## Related Scrappa Actors

- [Instagram User Posts Scraper](https://apify.com/thescrappa/instagram-user-posts-cheapest-0-20-1000-results)
- [Instagram Post Info \| Cheapest $0.20/1k results](https://apify.com/thescrappa/instagram-post-info-cheapest-0-20-1000-results)
- [TikTok User Posts Scraper](https://apify.com/thescrappa/tiktok-user-posts-scraper)
