# Instagram Comments Scraper

Collect public comments from Instagram posts with text, posting times, like totals, and author profile details. Batch post shortcodes and follow available comment pages.

## What data can you extract?

| Field | Type | Description |
| --- | --- | --- |
| `id` | String | Instagram identifier for this comment. |
| `text` | String | Comment text published beneath the Instagram post. |
| `taken_at` | String | ISO 8601 timestamp when the comment was posted, when available. |
| `taken_at_timestamp` | Integer | Unix timestamp for the comment posting time. |
| `like_count` | Integer | Number of likes shown for the comment. |
| `author` | Object | Public profile details associated with the comment author. |
| `author_id` | String | Instagram identifier for the public comment author. |
| `author_username` | String | Public Instagram handle of the comment author. |
| `author_verified` | String | True when Instagram marks the comment author as verified. |
| `input_shortcode` | String | Instagram post shortcode used to retrieve this comment. |
| `scraped_at` | String | UTC date and time when this record was collected. |

## Use cases

- Community managers can review public replies across several posts.
- Researchers can compare comment activity and posting times on public content.
- Brand teams can export comments and author handles for moderation workflows.

## How to use

1. Add each post shortcode to `shortcodes`.
2. Set `maxPages` to limit comment pages requested for each post.
3. Use `maxResults` to cap all saved comments in the run.

```json
{
  "shortcodes": [
    {
      "shortcode": "DdVztIfpeKW"
    }
  ],
  "maxResults": 20,
  "maxPages": 1
}
```

## Input

| Field | Type | Required | Description |
| --- | --- | --- | --- |
| `shortcodes` | Array\<object\> | Yes | Instagram post shortcodes to retrieve public comments. |
| `shortcodes[].shortcode` | string | Yes per entry | Post Shortcode sent to the source search. |
| `shortcodes[].max_id` | string | No | Next Page Cursor sent to the source search. |
| `max_id` | string | No | Next Page Cursor sent to the source search. |
| `maxResults` | integer | No | Maximum dataset items to save across this entire run. |
| `maxPages` | integer | No | Maximum pages to request for each batch entry. |

## Output example

```json
{
  "id": "18012345678901234",
  "text": "The colors in this landscape are beautiful.",
  "taken_at": "2025-09-18T14:32:11Z",
  "taken_at_timestamp": 1758205931,
  "like_count": 86,
  "author": {
    "id": "584700123456789",
    "username": "coastline.creator",
    "profile_pic_url": "https://cdn.instagram.com/profiles/coastline-creator.jpg",
    "is_verified": false
  },
  "author_id": "584700123456789",
  "author_username": "coastline.creator",
  "author_verified": false,
  "input_shortcode": "DdVztIfpeKW",
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

Yes. Send a POST request to `https://api.apify.com/v2/acts/thescrappa~instagram-post-comments-scraper/runs` with your Actor input, or use an Apify client library. Read the output from the run dataset.

### Can I connect the results to other tools?

Yes. Apify integrations and APIs can pass dataset results to Make, Zapier, n8n, Google Sheets, and other data workflows.

### What happens when one input fails?

The Actor logs a short source error and continues with the remaining entries. Transient rate limits and gateway errors are retried; if every entry fails, the run reports an error.

### How do I find a post shortcode?

Open the Instagram post URL and use the letters and numbers after `/p/` or `/reel/`.

## Related Scrappa Actors

- [Instagram Post Info \| Cheapest $0.20/1k results](https://apify.com/thescrappa/instagram-post-info-cheapest-0-20-1000-results)
- [Instagram User Posts Scraper](https://apify.com/thescrappa/instagram-user-posts-cheapest-0-20-1000-results)
- [TikTok Comments Scraper](https://apify.com/thescrappa/tiktok-comments-scraper)
