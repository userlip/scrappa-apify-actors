# TikTok Hashtag Videos Scraper for Creator Research

The TikTok Hashtag Videos Scraper for Creator Research collects hashtag and challenge video records from TikTok. Provide the fields listed below; the actor saves source fields such as `video_id`, `aweme_id`, `title`, and `author` to an Apify dataset.

## What data can you extract?

The dataset contains fields returned by TikTok. The field names below match the Actor output schema. A source may leave optional values empty or omit fields when they are not available for a result.

| Field | Type | Description |
| --- | --- | --- |
| `video_id` | text | Video ID returned for this result. |
| `aweme_id` | text | Aweme ID returned for this result. |
| `title` | text | Caption returned for this result. |
| `author` | object | Author returned for this result. |
| `play_count` | number | Views returned for this result. |
| `digg_count` | number | Likes returned for this result. |
| `comment_count` | number | Comments returned for this result. |
| `share_count` | number | Shares returned for this result. |
| `duration` | number | Duration returned for this result. |
| `region` | text | Video Region returned for this result. |
| `create_time` | number | Created returned for this result. |
| `challenge_id` | text | Challenge ID returned for this result. |
| `scraped_at` | date | Scraped At returned for this result. |

## Use cases

- Collect hashtag and challenge video records for video and creator research.
- Review returned titles, channels, timestamps, or engagement fields.
- Export video records to a content planning or analysis workflow.

## How to use

1. Open the **Input** tab and use the example JSON below.
2. Change the query, URL, identifier, or other fields you need. For multi-target work, use `challenge_ids` and start with a short list.
3. Start the Actor. Open the run's default dataset to inspect, download, or export the returned records.

```json
{
  "challenge_ids": [
    "1622962893630470"
  ]
}
```

Apify stores the run output in a dataset. You can download the dataset in JSON, CSV, Excel, XML, or other available formats from the run page.

## Output example

This illustrative record uses synthetic values. It shows the real output field names; optional source values may be null or absent.

```json
{
  "video_id": "abc123xyz09",
  "aweme_id": "abc123xyz09",
  "title": "Example result",
  "author": {},
  "play_count": 42,
  "digg_count": 42,
  "comment_count": 42
}
```

## Input fields

| Field | Type | Required | Description |
| --- | --- | --- | --- |
| `challenge_ids` | array of string | No | Up to 20 numeric TikTok challenge IDs. Use TikTok Challenge Search first to discover IDs. Constraints: minimum 1 items; maximum 20 items. |
| `challenge_id` | string | No | Compatibility input used only when challenge_ids is empty. |
| `region` | string | No | Optional two-letter region code, such as US. |
| `cursor` | string | No | Optional cursor applied as the starting point for each challenge. |
| `results_per_challenge` | integer | No | Maximum unique videos saved per challenge (1-500). Total requested results may not exceed 2,000. Constraints: minimum 1; maximum 500. |
| `page_size` | integer | No | Upstream page size (1-50). It is automatically reduced to remaining result and charge capacity. Constraints: minimum 1; maximum 50. |

## Pricing

**Current live price:** $0.25 per 1,000 results.

The price is based on the latest live Apify pricing entry. Per-result charges depend on the number of billed results returned. If the price line lists subscription tiers, the rate shown for each tier applies to that Apify subscription level.
Each saved video is billed through the `challenge-post-result` event at `$0.00025 per video`. No per-video key-value-store records are written.

## FAQ

### Is it legal to scrape this data?

This Actor is intended for data that is publicly available from TikTok. You are responsible for following the source site's terms, privacy and copyright rules, and the laws that apply to your use of the data. Only collect information you have a lawful basis to use.

### What limits should I expect?

Use the field constraints and limits in the input table. The source controls which records are available, so a narrow query or unavailable page can return fewer results or none. Keep batch lists small when you need a quick first run.

### Can I run it through the API or connect it to other tools?

Yes. Send the same JSON input to `POST https://api.apify.com/v2/actors/thescrappa~tiktok-challenge-posts-scraper/runs`. The response includes `defaultDatasetId`; use it to fetch the run's dataset items. See the [Apify Run Actor API](https://docs.apify.com/api/v2/actors-runs-post) and [Actor runs guide](https://docs.apify.com/api/v2/actors-actor-runs). Apify integrations for [Make](https://docs.apify.com/integrations/make), [Zapier](https://docs.apify.com/integrations/zapier), and [n8n](https://docs.apify.com/integrations/n8n) can trigger runs and pass results to the next workflow step. Send dataset rows to Google Sheets through a Sheets step or a workflow integration.

### What happens if a request fails?

Apify reports input validation and source request errors in the run details. Review the error, correct the input, and retry after a temporary source problem. Depending on when a request stops, the dataset may be empty or contain results collected before the failure.

## Related Scrappa Actors

- [TikTok Ads Scraper for Campaign Research](https://apify.com/thescrappa/tiktok-ads-scraper)
- [TikTok Hashtag & Challenge Details Scraper](https://apify.com/thescrappa/tiktok-challenge-details-scraper)
- [TikTok Challenge Search Scraper for Trends](https://apify.com/thescrappa/tiktok-challenge-search-scraper)
- [TikTok Comments Scraper for Audience Research](https://apify.com/thescrappa/tiktok-comments-scraper)
- [TikTok Followers Scraper for Audience Research](https://apify.com/thescrappa/tiktok-followers-scraper)
