# TikTok Hashtag Posts Scraper

Find public TikTok videos using a hashtag, with captions, creators and engagement counts. Enter a hashtag and optionally choose a region or result count.

## What data can you extract?

Captions, profile details and engagement counts reflect public TikTok pages; counts can be hidden or absent.

| Field | Type | Description |
| --- | --- | --- |
| `desc` | text | Video caption from TikTok for this TikTok video; null when the source has no text to show. |
| `author` | object | Creator profile with ID, username, display name, avatar and follower count from TikTok; null when the source provides no details. |
| `digg_count` | number | Number of likes shown by TikTok, as a whole number; zero is possible, and null means no count was reported. |
| `comment_count` | number | Number of comments shown by TikTok, as a whole number; zero is possible, and null means no count was reported. |
| `share_count` | number | Number of shares shown by TikTok, as a whole number; zero is possible, and null means no count was reported. |
| `play_count` | number | Number of video plays shown by TikTok, as a whole number; zero is possible, and null means no count was reported. |
| `create_time` | number | Creation timestamp for this TikTok video, as a Unix timestamp in seconds; null if TikTok does not supply it. |
| `aweme_id` | text | TikTok video ID for the TikTok video, assigned by TikTok; null when the source does not expose it. |
| `lookup_challenge_name` | text | Lookup challenge name shown for the TikTok video by TikTok, in the format used by the source; null when it is omitted. |
| `lookup_challenge_id` | text | lookup challenge id for the TikTok video, assigned by TikTok; null when the source does not expose it. |
| `resolved_challenge_name` | text | Resolved challenge name shown for the TikTok video by TikTok, in the format used by the source; null when it is omitted. |
| `resolved_challenge_id` | text | resolved challenge id for the TikTok video, assigned by TikTok; null when the source does not expose it. |
| `lookup_region` | text | Lookup region shown for the TikTok video by TikTok, in the format used by the source; null when it is omitted. |

## Use cases

- Campaign managers can monitor public videos using a TikTok challenge or hashtag.
- Researchers can compare captions and engagement across posts under a tag.
- Creators can review examples before planning a themed video.

## How to use

1. Open the Actor’s **Input** tab and start with the JSON below.
2. Set `hashtag` to the search term or source identifier you want to look up, then use the optional filters listed below.
3. Start the run and open its default dataset to inspect or download the rows.

```json
{
  "hashtag": "cosplay",
  "region": "US",
  "count": 10,
  "cursor": "0"
}
```

The run dataset can be downloaded as JSON, CSV, Excel or another format offered by Apify.

## Input

| Field | Type | Required | Description |
| --- | --- | --- | --- |
| `hashtag` | string | Yes | TikTok hashtag with or without #, full HTTPS TikTok hashtag URL, or numeric challenge ID. Bare numeric values are treated as challenge IDs. |
| `region` | string | No | Optional country or region code, such as US. |
| `count` | integer | No | Number of posts to return from this page. Scrappa accepts 1-50. Constraints: minimum 1; maximum 50. |
| `cursor` | string/number | No | Pagination cursor from a previous run. Leave empty for the first page. API callers may also pass numeric cursors. |

## Output example

The record below is synthetic. Names and links are examples, and private contact fields are omitted.

```json
{
  "desc": "Three ways to style a vintage wool coat for fall. Which look is your favorite?",
  "author": {
    "id": "7210458831",
    "unique_id": "morgan.creates",
    "nickname": "Morgan Creates",
    "avatar": "https://images.example.com/creators/morgan-lee.jpg",
    "follower_count": 18600
  },
  "digg_count": 864,
  "comment_count": 27,
  "share_count": 38,
  "play_count": 18400,
  "create_time": 1790327700,
  "aweme_id": "7348291056172489012"
}
```

## Pricing

**Current live price:** $0.30 per 1,000 results.

Each saved video, post or comment record counts as one result.

## FAQ

### Is it legal to collect public information?

This Actor is for information visible on the public source pages it reads. You are responsible for checking the source terms, privacy and copyright rules, and the laws that apply to your use.

### How many records will a run return?

The result count depends on the input limits, pagination settings and what the source makes available. A small query or unavailable page can return fewer records, including none.

### Can I call it through the API or connect it to other tools?

Yes. Send the same JSON input to `POST https://api.apify.com/v2/acts/thescrappa~tiktok-hashtag-posts-scraper/runs`, then read the run’s default dataset. Make, Zapier and n8n can start runs and pass dataset rows to the next step; Google Sheets can receive rows through those workflows or a dataset export.

### What happens when a request fails?

Check the run log for the source or input error, correct the input and retry after a temporary source issue. A run can contain rows saved before a later request failed.

### Should I include the # symbol in a TikTok hashtag?

Enter the hashtag in `hashtag` using the format shown in the Input tab. You can also choose a `region` and `count`; a `cursor` continues only when TikTok provides one.

## Related Scrappa Actors

- [TikTok Ads Scraper](https://apify.com/thescrappa/tiktok-ads-scraper)
- [TikTok Hashtag Details Scraper](https://apify.com/thescrappa/tiktok-challenge-details-scraper)
- [TikTok Hashtag Videos Scraper](https://apify.com/thescrappa/tiktok-challenge-posts-scraper)
- [TikTok Challenge Search Scraper](https://apify.com/thescrappa/tiktok-challenge-search-scraper)
- [TikTok Comments Scraper](https://apify.com/thescrappa/tiktok-comments-scraper)
