# Instagram Post Info | Cheapest $0.20/1k results

Look up a public Instagram post to see its caption, media type and engagement counts. Paste a public post URL or its shortcode to retrieve the details for that post.

## What data can you extract?

Post and profile details reflect information visible on public Instagram pages; optional fields can be absent.

| Field | Type | Description |
| --- | --- | --- |
| `success` | boolean | Whether the lookup completed successfully; false is a reported value, while null means Instagram provided no flag. |
| `data` | object | Instagram post details with shortcode, caption, publication time, likes, comments and media type; null when the source provides no details. |
| `error` | string | Diagnostic text for the Instagram lookup; null when the request completes without an error. |

## Use cases

- Social teams can review public post captions and engagement details.
- Brand researchers can compare post data while monitoring a campaign.
- Creators can archive source-linked posts for a recurring content review.

## How to use

1. Open the Actor’s **Input** tab and start with the JSON below.
2. Set `url` to the search term or source identifier you want to look up, then use the optional filters listed below.
3. Start the run and open its default dataset to inspect or download the rows.

```json
{
  "url": "https://www.instagram.com/instagram/p/DdUYPr8Piav/"
}
```

The run dataset can be downloaded as JSON, CSV, Excel or another format offered by Apify.

## Input

| Field | Type | Required | Description |
| --- | --- | --- | --- |
| `url` | string | No | Full Instagram post URL or shortcode. Include the username to enable an exact-match fallback through the account’s recent posts, for example https://www.instagram.com/instagram/p/DdUYPr8Piav/. |
| `shortcode` | string | No | Instagram post shortcode. Use this when you do not have the full post URL. Example shortcode: DUBtwxGEqz2. |
| `media_id` | string | No | Deprecated legacy input name. Use the URL or shortcode field above for new integrations. This actor treats media_id as the Instagram post shortcode. |

## Output example

The record below is synthetic. Names and links are examples, and private contact fields are omitted.

```json
{
  "success": true,
  "data": {
    "shortcode": "DJx8sQmP4ab",
    "caption": "Three easy ways to style a vintage wool coat for cooler days.",
    "taken_at": "2026-09-25T09:15:00Z",
    "like_count": 864,
    "comment_count": 27,
    "media_type": "video"
  }
}
```

## Pricing

**Current live price:** $0.20 per 1,000 results.

Each saved dataset record counts as one result.

## FAQ

### Is it legal to collect public information?

This Actor is for information visible on the public source pages it reads. You are responsible for checking the source terms, privacy and copyright rules, and the laws that apply to your use.

### How many records will a run return?

The result count depends on the input limits, pagination settings and what the source makes available. A small query or unavailable page can return fewer records, including none.

### Can I call it through the API or connect it to other tools?

Yes. Send the same JSON input to `POST https://api.apify.com/v2/acts/thescrappa~instagram-post-info-cheapest-0-20-1000-results/runs`, then read the run’s default dataset. Make, Zapier and n8n can start runs and pass dataset rows to the next step; Google Sheets can receive rows through those workflows or a dataset export.

### What happens when a request fails?

Check the run log for the source or input error, correct the input and retry after a temporary source issue. A run can contain rows saved before a later request failed.

### Can I use an Instagram shortcode instead of a post URL?

Yes. Submit the shortcode field or paste the public post URL in the url field.

## Related Scrappa Actors

- [Instagram User Info | Cheapest $0.20/1k results](https://apify.com/thescrappa/instagram-user-info-cheapest-0-20-1000-results)
- [Instagram User Posts Scraper](https://apify.com/thescrappa/instagram-user-posts-cheapest-0-20-1000-results)
- [Pinterest Search Scraper](https://apify.com/thescrappa/pinterest-search-scraper)
- [TikTok Search Scraper](https://apify.com/thescrappa/tiktok-search-scraper)
