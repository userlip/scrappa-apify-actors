# TikTok Search Scraper

Search TikTok videos by phrase and collect captions, creator details and play counts. Search video captions or topics, then refine by region, publication time or sort order.

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
| `video_id` | text | video ID for the TikTok video, assigned by TikTok; null when the source does not expose it. |
| `request_keywords` | text | Search keywords passed to TikTok. This input value is copied into the output row; null when it was not supplied. |
| `request_region` | text | Search region passed to TikTok. This input value is copied into the output row; null when it was not supplied. |
| `request_cursor` | text | Pagination cursor passed to TikTok. This input value is copied into the output row; null when it was not supplied. |
| `request_publish_time` | number | Video publication-time filter passed to TikTok. This input value is copied into the output row; null when it was not supplied. |
| `request_sort_type` | number | Result sort order passed to TikTok. This input value is copied into the output row; null when it was not supplied. |

## Use cases

- Social researchers can discover TikTok videos matching a phrase or creator.
- Creators can compare captions and engagement before choosing a topic.
- Campaign teams can monitor posts linked to a product or event.

## How to use

1. Open the Actor’s **Input** tab and start with the JSON below.
2. Set `keywords` to the search term or source identifier you want to look up, then use the optional filters listed below.
3. Start the run and open its default dataset to inspect or download the rows.

```json
{
  "keywords": "basketball",
  "region": "US",
  "count": 10,
  "cursor": "0"
}
```

The run dataset can be downloaded as JSON, CSV, Excel or another format offered by Apify.

## Input

| Field | Type | Required | Description |
| --- | --- | --- | --- |
| `keywords` | string | No | Keyword, hashtag, product, brand, topic, or creator niche to search for in TikTok videos. You can also provide query instead when calling the actor API. |
| `query` | string | No | Optional alias for Search Keywords for API callers. Ignored when Search Keywords is provided. |
| `region` | string | No | Optional country or region code, such as US, GB, JP, or DE. |
| `count` | integer | No | Number of video results to return from this page. Scrappa accepts 1-50. Constraints: minimum 1; maximum 50. |
| `cursor` | string | No | Pagination cursor from a previous run. Leave empty for the first page. |
| `publish_time` | integer | No | Optional upstream publish-time filter. Leave empty to use TikTok's default search window; broad values can behave like no filter if TikTok has no matching retained results. Constraints: minimum 0; maximum 3650. |
| `sort_type` | integer | No | Optional upstream sort type. Leave empty to use TikTok's default ranking. Constraints: minimum 0; maximum 10. |

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
  "video_id": "aB3dE5fG7hJ",
  "create_time": 1790327700
}
```

## Pricing

**Current live price:** $0.20 per 1,000 results.

Each saved video, post or comment record counts as one result.

## FAQ

### Is it legal to collect public information?

This Actor is for information visible on the public source pages it reads. You are responsible for checking the source terms, privacy and copyright rules, and the laws that apply to your use.

### How many records will a run return?

The result count depends on the input limits, pagination settings and what the source makes available. A small query or unavailable page can return fewer records, including none.

### Can I call it through the API or connect it to other tools?

Yes. Send the same JSON input to `POST https://api.apify.com/v2/acts/thescrappa~tiktok-search-scraper/runs`, then read the run’s default dataset. Make, Zapier and n8n can start runs and pass dataset rows to the next step; Google Sheets can receive rows through those workflows or a dataset export.

### What happens when a request fails?

Check the run log for the source or input error, correct the input and retry after a temporary source issue. A run can contain rows saved before a later request failed.

### Can TikTok Search filter videos by region or posting time?

Yes. Supply a phrase through `query` or `keywords`, then use `region`, `publish_time` or `sort_type` when those filters fit your search.

## Related Scrappa Actors

- [TikTok Ads Scraper](https://apify.com/thescrappa/tiktok-ads-scraper)
- [TikTok Hashtag Details Scraper](https://apify.com/thescrappa/tiktok-challenge-details-scraper)
- [TikTok Hashtag Videos Scraper](https://apify.com/thescrappa/tiktok-challenge-posts-scraper)
- [TikTok Challenge Search Scraper](https://apify.com/thescrappa/tiktok-challenge-search-scraper)
- [TikTok Comments Scraper](https://apify.com/thescrappa/tiktok-comments-scraper)
