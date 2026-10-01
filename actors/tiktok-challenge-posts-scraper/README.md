# TikTok Hashtag Videos Scraper

Collect public TikTok videos linked to a challenge, with captions, creators and engagement counts. Pass one or more challenge IDs; use TikTok Challenge Search or Challenge Details to find an ID first.

## What data can you extract?

Captions, profile details and engagement counts reflect public TikTok pages; counts can be hidden or absent.

| Field | Type | Description |
| --- | --- | --- |
| `video_id` | text | video ID for the TikTok video, assigned by TikTok; null when the source does not expose it. |
| `aweme_id` | text | TikTok video ID for the TikTok video, assigned by TikTok; null when the source does not expose it. |
| `title` | text | Title of the TikTok video, as shown by TikTok; null when no title is published. |
| `author` | object | Creator profile with ID, username, display name, avatar and follower count from TikTok; null when the source provides no details. |
| `play_count` | number | Number of video plays shown by TikTok, as a whole number; zero is possible, and null means no count was reported. |
| `digg_count` | number | Number of likes shown by TikTok, as a whole number; zero is possible, and null means no count was reported. |
| `comment_count` | number | Number of comments shown by TikTok, as a whole number; zero is possible, and null means no count was reported. |
| `share_count` | number | Number of shares shown by TikTok, as a whole number; zero is possible, and null means no count was reported. |
| `duration` | number | Duration of this TikTok video, in seconds; null when TikTok provides no timing information. |
| `region` | text | Region shown for the TikTok video by TikTok; null when TikTok does not provide the value. |
| `create_time` | number | Creation timestamp for this TikTok video, as a Unix timestamp in seconds; null if TikTok does not supply it. |
| `challenge_id` | text | Numeric ID of the TikTok challenge associated with this video; null if the video has no challenge ID. |
| `scraped_at` | date | Time the page was retrieved shown by TikTok, in ISO 8601 date and time; null if the source omits the date. |

## Use cases

- Campaign managers can monitor public videos using a TikTok challenge or hashtag.
- Researchers can compare captions and engagement across posts under a tag.
- Creators can review examples before planning a themed video.

## How to use

1. Open the Actor’s **Input** tab and start with the JSON below.
2. Add one or more challenge IDs; use TikTok Challenge Search to discover an ID from a challenge name.
3. Start the run and open its default dataset to inspect or download the rows.

```json
{
  "challenge_ids": [
    "1622962893630470"
  ]
}
```

The run dataset can be downloaded as JSON, CSV, Excel or another format offered by Apify.

## Input

| Field | Type | Required | Description |
| --- | --- | --- | --- |
| `challenge_ids` | array of string | No | Up to 20 numeric TikTok challenge IDs. Use TikTok Challenge Search first to discover IDs. Constraints: minimum 1 items; maximum 20 items. |
| `challenge_id` | string | No | Compatibility input used only when challenge_ids is empty. |
| `region` | string | No | Optional two-letter region code, such as US. |
| `cursor` | string | No | Optional cursor applied as the starting point for each challenge. |
| `results_per_challenge` | integer | No | Maximum unique videos saved per challenge (1-500). Total requested results may not exceed 2,000. Constraints: minimum 1; maximum 500. |
| `page_size` | integer | No | Upstream page size (1-50). It is automatically reduced to remaining result and charge capacity. Constraints: minimum 1; maximum 50. |

## Output example

The record below is synthetic. Names and links are examples, and private contact fields are omitted.

```json
{
  "video_id": "aB3dE5fG7hJ",
  "title": "A Saturday walk through the neighborhood market",
  "author": {
    "id": "7210458831",
    "unique_id": "morgan.creates",
    "nickname": "Morgan Creates",
    "avatar": "https://images.example.com/creators/morgan-lee.jpg",
    "follower_count": 18600
  },
  "play_count": 18400,
  "digg_count": 864,
  "comment_count": 27,
  "aweme_id": "7348291056172489012",
  "share_count": 38
}
```

## Pricing

**Current live price:** $0.25 per 1,000 results.

Each saved video, post or comment record counts as one result.

## FAQ

### Is it legal to collect public information?

This Actor is for information visible on the public source pages it reads. You are responsible for checking the source terms, privacy and copyright rules, and the laws that apply to your use.

### How many records will a run return?

The result count depends on the input limits, pagination settings and what the source makes available. A small query or unavailable page can return fewer records, including none.

### Can I call it through the API or connect it to other tools?

Yes. Send the same JSON input to `POST https://api.apify.com/v2/acts/thescrappa~tiktok-challenge-posts-scraper/runs`, then read the run’s default dataset. Make, Zapier and n8n can start runs and pass dataset rows to the next step; Google Sheets can receive rows through those workflows or a dataset export.

### What happens when a request fails?

Check the run log for the source or input error, correct the input and retry after a temporary source issue. A run can contain rows saved before a later request failed.

### How do I find a TikTok challenge ID?

Use TikTok Challenge Search or TikTok Hashtag Details to find it, then pass its ID in challenge_ids.

## Related Scrappa Actors

- [TikTok Ads Scraper](https://apify.com/thescrappa/tiktok-ads-scraper)
- [TikTok Hashtag Details Scraper](https://apify.com/thescrappa/tiktok-challenge-details-scraper)
- [TikTok Challenge Search Scraper](https://apify.com/thescrappa/tiktok-challenge-search-scraper)
- [TikTok Comments Scraper](https://apify.com/thescrappa/tiktok-comments-scraper)
- [TikTok Followers Scraper](https://apify.com/thescrappa/tiktok-followers-scraper)
