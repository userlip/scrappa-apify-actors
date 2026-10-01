# TikTok Video Details Scraper

Look up a public TikTok video with its caption, creator and engagement metrics. Paste a public video URL or submit several URLs, then enable the optional HD setting when required.

## What data can you extract?

Captions, profile details and engagement counts reflect public TikTok pages; counts can be hidden or absent.

| Field | Type | Description |
| --- | --- | --- |
| `title` | text | Title of the TikTok video, as shown by TikTok; null when no title is published. |
| `author` | object | Creator profile with ID, username, display name, avatar and follower count from TikTok; null when the source provides no details. |
| `play_count` | number | Number of video plays shown by TikTok, as a whole number; zero is possible, and null means no count was reported. |
| `digg_count` | number | Number of likes shown by TikTok, as a whole number; zero is possible, and null means no count was reported. |
| `comment_count` | number | Number of comments shown by TikTok, as a whole number; zero is possible, and null means no count was reported. |
| `share_count` | number | Number of shares shown by TikTok, as a whole number; zero is possible, and null means no count was reported. |
| `download_count` | number | Number of downloads shown by TikTok, as a whole number; zero is possible, and null means no count was reported. |
| `collect_count` | number | Number of collections shown by TikTok, as a whole number; zero is possible, and null means no count was reported. |
| `duration` | number | Duration of this TikTok video, in seconds; null when TikTok provides no timing information. |
| `create_time` | number | Creation timestamp for this TikTok video, as a Unix timestamp in seconds; null if TikTok does not supply it. |
| `aweme_id` | text | TikTok video ID for the TikTok video, assigned by TikTok; null when the source does not expose it. |
| `id` | text | source ID for the TikTok video, assigned by TikTok; null when the source does not expose it. |
| `play` | link | Play for this TikTok video on TikTok; null when the source does not provide a URL. |
| `wmplay` | link | Wmplay for this TikTok video on TikTok; null when the source does not provide a URL. |
| `hdplay` | link | Hdplay for this TikTok video on TikTok; null when the source does not provide a URL. |
| `cover` | link | Cover for this TikTok video on TikTok; null when the source does not provide a URL. |
| `request_url` | link | Source page url passed to TikTok. This input value is copied into the output row; null when it was not supplied. |
| `request_index` | number | Zero-based position of this request in the submitted TikTok input batch; null for a single-item lookup. |
| `request_hd` | boolean | High-definition image filter passed to TikTok. This input value is copied into the output row; null when it was not supplied. |
| `result_found` | boolean | Whether the requested result was found; false is a reported value, while null means TikTok provided no flag. |
| `error_message` | text | Diagnostic text for the TikTok lookup; null when the request completes without an error. |

## Use cases

- Social researchers can discover TikTok videos matching a phrase or creator.
- Creators can compare captions and engagement before choosing a topic.
- Campaign teams can monitor posts linked to a product or event.

## How to use

1. Open the Actor’s **Input** tab and start with the JSON below.
2. Put the supported targets in `urls` and use the identifier or URL format required by TikTok.
3. Start the run and open its default dataset to inspect or download the rows.

```json
{
  "urls": [
    "https://www.tiktok.com/@tiktok/video/7568510388342443294"
  ]
}
```

The run dataset can be downloaded as JSON, CSV, Excel or another format offered by Apify.

## Input

| Field | Type | Required | Description |
| --- | --- | --- | --- |
| `urls` | array of string | No | One or more TikTok video URLs, photo URLs, short URLs, or numeric video IDs. The actor pushes one dataset item for each requested URL. Constraints: minimum 1 items; maximum 100 items. |
| `url` | string | No | Legacy single URL field for API callers. Ignored when TikTok Video URLs is provided. |
| `hd` | boolean | No | Ask Scrappa to include HD playback fields when upstream data is available. |

## Output example

The record below is synthetic. Names and links are examples, and private contact fields are omitted.

```json
{
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
  "share_count": 38,
  "download_count": 116,
  "collect_count": 207
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

Yes. Send the same JSON input to `POST https://api.apify.com/v2/acts/thescrappa~tiktok-video-scraper/runs`, then read the run’s default dataset. Make, Zapier and n8n can start runs and pass dataset rows to the next step; Google Sheets can receive rows through those workflows or a dataset export.

### What happens when a request fails?

Check the run log for the source or input error, correct the input and retry after a temporary source issue. A run can contain rows saved before a later request failed.

### Can I submit several TikTok video URLs in one run?

Yes. Put URLs in `urls`, or use `url` for a single video. The optional `hd` setting controls the high-definition media option when available.

## Related Scrappa Actors

- [TikTok Ads Scraper](https://apify.com/thescrappa/tiktok-ads-scraper)
- [TikTok Hashtag Details Scraper](https://apify.com/thescrappa/tiktok-challenge-details-scraper)
- [TikTok Hashtag Videos Scraper](https://apify.com/thescrappa/tiktok-challenge-posts-scraper)
- [TikTok Challenge Search Scraper](https://apify.com/thescrappa/tiktok-challenge-search-scraper)
- [TikTok Comments Scraper](https://apify.com/thescrappa/tiktok-comments-scraper)
