# Instagram User Posts Scraper

List recent public Instagram posts with captions, media types and publication times. Choose a public username and use the returned pagination cursor to continue through available posts.

## What data can you extract?

Post and profile details reflect information visible on public Instagram pages; optional fields can be absent.

| Field | Type | Description |
| --- | --- | --- |
| `request_username` | text | Public account username passed to Instagram. This input value is copied into the output row; null when it was not supplied. |
| `id` | text | source ID for the Instagram post, assigned by Instagram; null when the source does not expose it. |
| `username` | text | Username shown for the Instagram post by Instagram, in the format used by the source; null when it is omitted. |
| `shortcode` | text | Instagram post shortcode for the Instagram post, assigned by Instagram; null when the source does not expose it. |
| `media_type` | text | Content format, such as video, image or carousel; null when Instagram does not provide the value. |
| `caption` | text | Post caption from Instagram for this Instagram post; null when the source has no text to show. |
| `taken_at` | date | Time the photo was posted shown by Instagram, in ISO 8601 date and time; null if the source omits the date. |
| `like_count` | number | Number of likes shown by Instagram, as a whole number; zero is possible, and null means no count was reported. |
| `comment_count` | number | Number of comments shown by Instagram, as a whole number; zero is possible, and null means no count was reported. |
| `play_count` | number | Number of video plays shown by Instagram, as a whole number; zero is possible, and null means no count was reported. |
| `permalink` | link | Permalink for this Instagram post on Instagram; null when the source does not provide a URL. |

## Use cases

- Social teams can review public post captions and engagement details.
- Brand researchers can compare post data while monitoring a campaign.
- Creators can archive source-linked posts for a recurring content review.

## How to use

1. Open the Actor’s **Input** tab and start with the JSON below.
2. Set `username` to the search term or source identifier you want to look up, then use the optional filters listed below.
3. Start the run and open its default dataset to inspect or download the rows.

```json
{
  "username": "natgeo"
}
```

The run dataset can be downloaded as JSON, CSV, Excel or another format offered by Apify.

## Input

| Field | Type | Required | Description |
| --- | --- | --- | --- |
| `username` | string | Yes | Instagram username without the @ symbol. |
| `max_id` | string | No | Optional pagination cursor from a previous response's next_max_id field. |

## Output example

The record below is synthetic. Names and links are examples, and private contact fields are omitted.

```json
{
  "play_count": 18400,
  "id": "17984273651028457",
  "username": "riley.hart.studio",
  "shortcode": "C9aKpWmR4tQ",
  "media_type": "video",
  "caption": "Three ways to style a vintage wool coat for fall. Which look is your favorite?",
  "taken_at": "2026-09-25T09:15:00Z",
  "like_count": 864
}
```

## Pricing

**Current live price:** Apify Free tier: $0.30 per 1,000 results; Bronze: $0.25 per 1,000 results; Silver: $0.22 per 1,000 results; Gold, Platinum, and Diamond: $0.20 per 1,000 results.

Each saved dataset record counts as one result.

## FAQ

### Is it legal to collect public information?

This Actor is for information visible on the public source pages it reads. You are responsible for checking the source terms, privacy and copyright rules, and the laws that apply to your use.

### How many records will a run return?

The result count depends on the input limits, pagination settings and what the source makes available. A small query or unavailable page can return fewer records, including none.

### Can I call it through the API or connect it to other tools?

Yes. Send the same JSON input to `POST https://api.apify.com/v2/acts/thescrappa~instagram-user-posts-cheapest-0-20-1000-results/runs`, then read the run’s default dataset. Make, Zapier and n8n can start runs and pass dataset rows to the next step; Google Sheets can receive rows through those workflows or a dataset export.

### What happens when a request fails?

Check the run log for the source or input error, correct the input and retry after a temporary source issue. A run can contain rows saved before a later request failed.

### Can Instagram User Posts continue from an earlier page?

Use the public `username` and pass `max_id` from a previous response when one is provided. The profile must have accessible public posts.

## Related Scrappa Actors

- [Instagram Post Info | Cheapest $0.20/1k results](https://apify.com/thescrappa/instagram-post-info-cheapest-0-20-1000-results)
- [Instagram User Info | Cheapest $0.20/1k results](https://apify.com/thescrappa/instagram-user-info-cheapest-0-20-1000-results)
- [Pinterest Search Scraper](https://apify.com/thescrappa/pinterest-search-scraper)
- [TikTok Search Scraper](https://apify.com/thescrappa/tiktok-search-scraper)
