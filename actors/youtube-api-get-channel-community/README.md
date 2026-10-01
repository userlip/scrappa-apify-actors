# YouTube Channel Community Posts Scraper

Read public YouTube community posts with text, publication time, likes and comments. Start with a channel ID, then reuse a returned continuation value to request another community-post page.

## What data can you extract?

Titles, publication details and engagement counts reflect public YouTube pages; some fields are hidden or unavailable for a video.

| Field | Type | Description |
| --- | --- | --- |
| `id` | string | source ID for the channel community post, assigned by YouTube; null when the source does not expose it. |
| `text` | string | Text content from YouTube for this channel community post; null when the source has no text to show. |
| `publishedTime` | string | Publication age displayed by YouTube, such as “2 days ago”; null when YouTube does not show it. |
| `likeCount` | integer/number/string | Number of likes shown by YouTube, as a number or digit string; zero is possible, and null means no count was reported. |
| `commentCount` | integer/number/string | Number of comments shown by YouTube, as a number or digit string; zero is possible, and null means no count was reported. |
| `attachments` | array of objects | Community post attachments with media type, title, URL and thumbnail from YouTube; an empty list when no entries are available. |

## Use cases

- Channel teams can review public post text and displayed publication ages while monitoring community activity.
- Researchers can compare likes and comment counts across a channel’s public community posts.
- Agencies can track new community posts from a list of YouTube channels.

## How to use

1. Open the Actor’s **Input** tab and start with the JSON below.
2. Enter a YouTube channel ID in `id`. Supply a `continuation` token when you want to request a later page.
3. Start the run and open its default dataset to inspect or download the rows.

```json
{
  "id": "UCJZv4d5rbIKd4QHMPkcABCw"
}
```

The run dataset can be downloaded as JSON, CSV, Excel or another format offered by Apify.

## Input

| Field | Type | Required | Description |
| --- | --- | --- | --- |
| `id` | string | Yes | Youtube Channel ID |
| `continuation` | string | No | Pagination token for next page |

## Output example

The record below is synthetic. Names and links are examples, and private contact fields are omitted.

```json
{
  "id": "aB3dE5fG7hJ",
  "text": "We are planning a live session about balcony planters this Friday. Leave your questions below.",
  "publishedTime": "September 25, 2026",
  "likeCount": 864,
  "commentCount": 27,
  "attachments": [
    {
      "type": "image",
      "title": "Community market photo",
      "url": "https://images.example.com/community/market-day.jpg",
      "thumbnail": "https://images.example.com/community/market-day-thumb.jpg"
    }
  ]
}
```

## Pricing

**Current live price:** Apify Free tier: $0.30 per 1,000 results; Bronze: $0.25 per 1,000 results; Silver: $0.22 per 1,000 results; Gold, Platinum, and Diamond: $0.20 per 1,000 results.

Each saved community post row counts as one result.

## FAQ

### Is it legal to collect public information?

This Actor is for information visible on the public source pages it reads. You are responsible for checking the source terms, privacy and copyright rules, and the laws that apply to your use.

### How many records will a run return?

Each request returns the community posts available for the supplied channel ID and continuation value. The number of rows varies with channel activity and the source response.

### Can I call it through the API or connect it to other tools?

Yes. Send the same JSON input to `POST https://api.apify.com/v2/acts/thescrappa~youtube-api-get-channel-community/runs`, then read the run’s default dataset. Make, Zapier and n8n can start runs and pass dataset rows to the next step; Google Sheets can receive rows through those workflows or a dataset export.

### What happens when a request fails?

Check the run log for the source or input error, correct the input and retry after a temporary source issue. A run can contain rows saved before a later request failed.

### How do I get another page of YouTube community posts?

Reuse the channel `id` and pass the `continuation` value from the previous response. YouTube provides a continuation only when more posts are available.

## Related Scrappa Actors

- [YouTube Batch Video Scraper](https://apify.com/thescrappa/youtube-api-batch-videos)
- [YouTube Channel Podcasts Scraper](https://apify.com/thescrappa/youtube-api-channel-podcasts)
- [YouTube Channel Video Scraper](https://apify.com/thescrappa/youtube-api-channel-videos)
- [YouTube Channel About Details Scraper](https://apify.com/thescrappa/youtube-api-get-channel-about-details)
- [YouTube Channel Details Scraper](https://apify.com/thescrappa/youtube-api-get-channel-details)
