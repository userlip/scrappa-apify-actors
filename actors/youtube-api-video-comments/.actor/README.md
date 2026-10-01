# YouTube Video Comments Scraper

Read public YouTube video comments with text, publication time, likes and replies. Enter a YouTube video ID and pass the continuation value to request another page.

## What data can you extract?

Titles, publication details and engagement counts reflect public YouTube pages; some fields are hidden or unavailable for a video.

| Field | Type | Description |
| --- | --- | --- |
| `id` | string | source ID for the video comment, assigned by YouTube; null when the source does not expose it. |
| `text` | string | Text content from YouTube for this video comment; null when the source has no text to show. |
| `publishedTime` | string | Publication age displayed by YouTube, such as “2 days ago”; null when YouTube does not show it. |
| `likeCount` | integer/number/string | Number of likes shown by YouTube, as a number or digit string; zero is possible, and null means no count was reported. |
| `replyCount` | integer/number/string | Number of replies shown by YouTube, as a number or digit string; zero is possible, and null means no count was reported. |
| `isPinned` | boolean | Whether the comment is pinned; false is a reported value, while null means YouTube provided no flag. |
| `isHearted` | boolean | Whether the channel owner hearted the comment; false is a reported value, while null means YouTube provided no flag. |
| `isOwner` | boolean | Whether the author owns the channel; false is a reported value, while null means YouTube provided no flag. |
| `author` | object | Creator profile with ID, username, display name, avatar and follower count from YouTube; null when the source provides no details. |
| `replies` | array of objects | Comment replies with text, author, publication time and like count from YouTube; an empty list when no entries are available. |

## Use cases

- Channel managers can review public questions and replies before preparing a response.
- Researchers can compare comment themes and reply counts across videos.
- Video teams can use audience feedback to inform a later episode or tutorial.

## How to use

1. Open the Actor’s **Input** tab and start with the JSON below.
2. Set `id` to the search term or source identifier you want to look up, then use the optional filters listed below.
3. Start the run and open its default dataset to inspect or download the rows.

```json
{
  "id": "dQw4w9WgXcQ",
  "sort": [
    "TOP_COMMENTS"
  ]
}
```

The run dataset can be downloaded as JSON, CSV, Excel or another format offered by Apify.

## Input

| Field | Type | Required | Description |
| --- | --- | --- | --- |
| `id` | string | Yes | The YouTube video ID to fetch comments for. |
| `continuation` | string | No | Pagination token returned by the previous run to continue to the next comments page. |
| `sort` | array of string | No | Sort order for the comments feed. Constraints: maximum 1 items. |

## Output example

The record below is synthetic. Names and links are examples, and private contact fields are omitted.

```json
{
  "id": "UgxDemoComment7qR8",
  "text": "The sunlight tip at 2:14 helped my basil recover. Thanks for explaining it clearly.",
  "author": {
    "id": "UCaBcdEFghIJKlMNopQRSTuv",
    "name": "Harborlight Studio",
    "url": "https://www.youtube.com/@HarborlightStudio",
    "thumbnail": "https://images.example.com/channels/harborlight-studio.jpg",
    "subscriberCount": 18600
  },
  "publishedTime": "September 25, 2026",
  "likeCount": 864,
  "replyCount": 18,
  "replies": [
    {
      "id": "reply_demo_08",
      "text": "The waterfront route is lovely in the morning.",
      "author": {
        "id": "UCaBcdEFghIJKlMNopQRSTuv",
        "name": "Harborlight Studio"
      }
    }
  ],
  "isPinned": false
}
```

## Pricing

**Current live price:** Apify Free tier: $0.30 per 1,000 results; Bronze: $0.25 per 1,000 results; Silver: $0.22 per 1,000 results; Gold, Platinum, and Diamond: $0.20 per 1,000 results.

Each saved video, post or comment record counts as one result.

## FAQ

### Is it legal to collect public information?

This Actor is for information visible on the public source pages it reads. You are responsible for checking the source terms, privacy and copyright rules, and the laws that apply to your use.

### How many records will a run return?

The result count depends on the input limits, pagination settings and what the source makes available. A small query or unavailable page can return fewer records, including none.

### Can I call it through the API or connect it to other tools?

Yes. Send the same JSON input to `POST https://api.apify.com/v2/acts/thescrappa~youtube-api-video-comments/runs`, then read the run’s default dataset. Make, Zapier and n8n can start runs and pass dataset rows to the next step; Google Sheets can receive rows through those workflows or a dataset export.

### What happens when a request fails?

Check the run log for the source or input error, correct the input and retry after a temporary source issue. A run can contain rows saved before a later request failed.

### How do I fetch another page of YouTube comments?

Send the video `id` and the `continuation` value from the prior response. YouTube supplies a continuation only when another page can be requested.

## Related Scrappa Actors

- [YouTube Batch Video Scraper](https://apify.com/thescrappa/youtube-api-batch-videos)
- [YouTube Channel Podcasts Scraper](https://apify.com/thescrappa/youtube-api-channel-podcasts)
- [YouTube Channel Video Scraper](https://apify.com/thescrappa/youtube-api-channel-videos)
- [YouTube Channel About Details Scraper](https://apify.com/thescrappa/youtube-api-get-channel-about-details)
- [YouTube Channel Community Posts Scraper](https://apify.com/thescrappa/youtube-api-get-channel-community)
