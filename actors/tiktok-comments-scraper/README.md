# TikTok Comments Scraper

Read public TikTok video comments with text, commenter details, likes and replies. Paste a public video URL and choose whether to include replies when the source provides them.

## What data can you extract?

Captions, profile details and engagement counts reflect public TikTok pages; counts can be hidden or absent.

| Field | Type | Description |
| --- | --- | --- |
| `comment_type` | text | Comment type shown for the video comment by TikTok, in the format used by the source; null when it is omitted. |
| `text` | text | Text content from TikTok for this video comment; null when the source has no text to show. |
| `user` | object | Commenter profile with ID, username, display name and avatar from TikTok; null when the source provides no details. |
| `digg_count` | number | Number of likes shown by TikTok, as a whole number; zero is possible, and null means no count was reported. |
| `reply_count` | number | Number of replies shown by TikTok, as a whole number; zero is possible, and null means no count was reported. |
| `create_time` | number | Creation timestamp for this video comment, as a Unix timestamp in seconds; null if TikTok does not supply it. |
| `comment_id` | text | comment ID for the video comment, assigned by TikTok; null when the source does not expose it. |
| `parent_comment_id` | text | parent comment id for the video comment, assigned by TikTok; null when the source does not expose it. |
| `video_id` | text | video ID for the video comment, assigned by TikTok; null when the source does not expose it. |
| `video_url` | link | Video url for this video comment on TikTok; null when the source does not provide a URL. |

## Use cases

- Community managers can review public questions and replies under a video.
- Researchers can compare comment text and engagement to study audience response.
- Creators can collect feedback before planning a follow-up post.

## How to use

1. Open the Actor’s **Input** tab and start with the JSON below.
2. Set `url` to the search term or source identifier you want to look up, then use the optional filters listed below.
3. Start the run and open its default dataset to inspect or download the rows.

```json
{
  "url": "https://www.tiktok.com/@tiktok/video/7568510388342443294",
  "count": 5
}
```

The run dataset can be downloaded as JSON, CSV, Excel or another format offered by Apify.

## Input

| Field | Type | Required | Description |
| --- | --- | --- | --- |
| `url` | string | Yes | Full TikTok video URL to fetch comments from. |
| `count` | integer | No | Number of comments to return from this page. Scrappa accepts 1-50. Constraints: minimum 1; maximum 50. |
| `cursor` | string | No | Pagination cursor from a previous run. Leave empty for the first page. |
| `includeReplies` | boolean | No | When enabled, fetch replies for each top-level comment that has replies. Reply rows are added to the dataset with comment_type='reply' and parent_comment_id. |
| `maxRepliesPerComment` | integer | No | Maximum replies to fetch for each top-level comment when reply collection is enabled. Scrappa fetches replies in pages of up to 50. Constraints: minimum 1; maximum 500. |

## Output example

The record below is synthetic. Names and links are examples, and private contact fields are omitted.

```json
{
  "comment_type": "comment",
  "text": "Three ways to style a vintage wool coat for fall. Which look is your favorite?",
  "user": {
    "id": "8042167953",
    "unique_id": "riley.reads",
    "nickname": "Riley Park",
    "avatar": "https://images.example.com/creators/riley-park.jpg"
  },
  "digg_count": 864,
  "reply_count": 4,
  "create_time": 1790327700,
  "video_id": "aB3dE5fG7hJ",
  "comment_id": "comment_demo_04"
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

Yes. Send the same JSON input to `POST https://api.apify.com/v2/acts/thescrappa~tiktok-comments-scraper/runs`, then read the run’s default dataset. Make, Zapier and n8n can start runs and pass dataset rows to the next step; Google Sheets can receive rows through those workflows or a dataset export.

### What happens when a request fails?

Check the run log for the source or input error, correct the input and retry after a temporary source issue. A run can contain rows saved before a later request failed.

### Can TikTok Comments include replies to a comment?

Set `includeReplies` to true to request replies, and use `maxRepliesPerComment` to cap replies per comment. A `cursor` can continue a paginated request when the response provides one.

## Related Scrappa Actors

- [TikTok Ads Scraper](https://apify.com/thescrappa/tiktok-ads-scraper)
- [TikTok Hashtag Details Scraper](https://apify.com/thescrappa/tiktok-challenge-details-scraper)
- [TikTok Hashtag Videos Scraper](https://apify.com/thescrappa/tiktok-challenge-posts-scraper)
- [TikTok Challenge Search Scraper](https://apify.com/thescrappa/tiktok-challenge-search-scraper)
- [TikTok Followers Scraper](https://apify.com/thescrappa/tiktok-followers-scraper)
