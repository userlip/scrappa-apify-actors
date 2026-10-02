# Instagram Tagged Posts Scraper

Find public Instagram posts that tag an account and collect captions, engagement, media links, and publisher profiles. Add multiple usernames and follow available result pages.

## What data can you extract?

| Field | Type | Description |
| --- | --- | --- |
| `id` | String | Instagram media identifier for the post where the account is tagged. |
| `shortcode` | String | Short code used in the public Instagram post URL. |
| `media_type` | String | Instagram format label for the tagged post. |
| `caption` | String | Caption text published with the tagged post. |
| `hashtags` | Array\<String\> | Hashtags extracted from the tagged post caption. |
| `taken_at` | String or null | ISO 8601 posting time when Instagram provides it. |
| `taken_at_timestamp` | Integer or null | Unix posting timestamp when Instagram provides it. |
| `like_count` | Integer | Number of likes reported for the tagged post. |
| `comment_count` | Integer | Number of comments reported for the tagged post. |
| `play_count` | Integer or null | Video play count when the tagged post is a video; otherwise null. |
| `view_count` | Integer or null | Video view count when the tagged post is a video; otherwise null. |
| `media` | Array\<Object\> | Image or video assets attached to the tagged post. |
| `location` | Object or null | Location attached to the tagged post, or null when none is reported. |
| `author` | Object | Public profile details for the account that published the tagged post. |
| `collaborators` | Array\<Object\> | Public collaborator profiles attached to the post, when present. |
| `tagged_users` | Array\<Object\> | Public profile records tagged in the post, when present. |
| `is_paid_partnership` | Boolean | True when Instagram marks the post as a paid partnership. |
| `product_type` | String | Instagram publishing format for the tagged post. |
| `permalink` | String | Public Instagram URL for the tagged post. |
| `accessibility_caption` | String or null | Accessibility description of the media, or null when Instagram has none. |
| `author_id` | String | Instagram identifier for the account that published the post. |
| `author_username` | String or null | Public handle of the account that published the post, when available. |
| `author_verified` | String | True when Instagram marks the posting account as verified. |
| `input_username` | String | Instagram username used to retrieve this account’s posts. |
| `scraped_at` | String | UTC date and time when this record was collected. |

## Use cases

- Brand teams can monitor public posts that tag a company account.
- Creator managers can review tagged content across several public profiles.
- Researchers can study how accounts appear in public community posts.

## How to use

1. Add each public account username to `usernames`.
2. Set `maxPages` to limit cursor pagination for each account.
3. Export post captions, authors, and links with `maxResults` set to your run cap.

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
| `usernames` | Array\<object\> | Yes | Public Instagram usernames whose tagged posts you want to retrieve. |
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
  "id": "3930123456789012345",
  "shortcode": "CSTtaggedPost28",
  "media_type": "Carousel",
  "caption": "Weekend market finds and the people who made them.",
  "hashtags": [
    "citymarket",
    "smallbatch"
  ],
  "taken_at": "2025-09-18T14:32:11Z",
  "taken_at_timestamp": 1758205931,
  "like_count": 913,
  "comment_count": 27,
  "play_count": null,
  "view_count": null,
  "media": [
    {
      "type": "image",
      "thumbnail_url": "https://cdn.instagram.com/media/market-table.jpg"
    }
  ],
  "location": null,
  "author": {
    "id": "584700123456792",
    "username": "maker.studio",
    "full_name": "Maker Studio",
    "profile_pic_url": "https://cdn.instagram.com/profiles/maker-studio.jpg",
    "is_verified": false
  },
  "collaborators": [
    {
      "id": "584700123456793",
      "username": "harbor.bakes"
    }
  ],
  "tagged_users": [
    {
      "id": "584700123456789",
      "username": "harbor.bakes"
    }
  ],
  "is_paid_partnership": false,
  "product_type": "carousel_container",
  "permalink": "https://www.instagram.com/p/CSTtaggedPost28/",
  "accessibility_caption": "A table of handmade ceramics and fresh bread.",
  "author_id": "584700123456792",
  "author_username": "maker.studio",
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

Yes. Send a POST request to `https://api.apify.com/v2/acts/thescrappa~instagram-tagged-posts-scraper/runs` with your Actor input, or use an Apify client library. Read the output from the run dataset.

### Can I connect the results to other tools?

Yes. Apify integrations and APIs can pass dataset results to Make, Zapier, n8n, Google Sheets, and other data workflows.

### What happens when one input fails?

The Actor logs a short source error and continues with the remaining entries. Transient rate limits and gateway errors are retried; if every entry fails, the run reports an error.

### Does this include private posts?

No. Results depend on public tagged-post data available from Instagram.

## Related Scrappa Actors

- [Instagram User Posts Scraper](https://apify.com/thescrappa/instagram-user-posts-cheapest-0-20-1000-results)
- [Instagram User Info \| Cheapest $0.20/1k results](https://apify.com/thescrappa/instagram-user-info-cheapest-0-20-1000-results)
- [TikTok Hashtag Posts Scraper](https://apify.com/thescrappa/tiktok-hashtag-posts-scraper)
