# TikTok Comments Scraper for Audience Research

The TikTok Comments Scraper for Audience Research collects reviews, ratings, and comment details from TikTok. Provide one or more public URLs; the actor saves source fields such as `comment_type`, `text`, `user`, and `digg_count` to an Apify dataset.

## What data can you extract?

The dataset contains fields returned by TikTok. The field names below match the Actor output schema. A source may leave optional values empty or omit fields when they are not available for a result.

| Field | Type | Description |
| --- | --- | --- |
| `comment_type` | text | Type returned for this result. |
| `text` | text | Comment returned for this result. |
| `user` | object | User returned for this result. |
| `digg_count` | number | Likes returned for this result. |
| `reply_count` | number | Replies returned for this result. |
| `create_time` | number | Created returned for this result. |
| `comment_id` | text | Comment ID returned for this result. |
| `parent_comment_id` | text | Parent Comment ID returned for this result. |
| `video_id` | text | Video ID returned for this result. |
| `video_url` | link | Video URL returned for this result. |

## Use cases

- Collect reviews, ratings, and comment details for audience and content research.
- Review public profile, post, or engagement fields returned for each item.
- Export the dataset to a social reporting or creator workflow.

## How to use

1. Open the **Input** tab and use the example JSON below.
2. Change the query, URL, identifier, or other fields you need. Set the lookup fields to match the query or identifier you want to collect.
3. Start the Actor. Open the run's default dataset to inspect, download, or export the returned records.

```json
{
  "url": "https://www.tiktok.com/@tiktok/video/7568510388342443294",
  "count": 5
}
```

Apify stores the run output in a dataset. You can download the dataset in JSON, CSV, Excel, XML, or other available formats from the run page.

## Output example

This illustrative record uses synthetic values. It shows the real output field names; optional source values may be null or absent.

```json
{
  "comment_type": "Example public text.",
  "text": "Example public text.",
  "user": {},
  "digg_count": 42,
  "reply_count": 42,
  "create_time": 42,
  "comment_id": "Example public text."
}
```

## Input fields

| Field | Type | Required | Description |
| --- | --- | --- | --- |
| `url` | string | Yes | Full TikTok video URL to fetch comments from. |
| `count` | integer | No | Number of comments to return from this page. Scrappa accepts 1-50. Constraints: minimum 1; maximum 50. |
| `cursor` | string | No | Pagination cursor from a previous run. Leave empty for the first page. |
| `includeReplies` | boolean | No | When enabled, fetch replies for each top-level comment that has replies. Reply rows are added to the dataset with comment_type='reply' and parent_comment_id. |
| `maxRepliesPerComment` | integer | No | Maximum replies to fetch for each top-level comment when reply collection is enabled. Scrappa fetches replies in pages of up to 50. Constraints: minimum 1; maximum 500. |

## Pricing

**Current live price:** Apify Free tier: $0.30 per 1,000 results; Bronze: $0.25 per 1,000 results; Silver: $0.22 per 1,000 results; Gold, Platinum, and Diamond: $0.20 per 1,000 results.

The price is based on the latest live Apify pricing entry. Per-result charges depend on the number of billed results returned. If the price line lists subscription tiers, the rate shown for each tier applies to that Apify subscription level.

## FAQ

### Is it legal to scrape this data?

This Actor is intended for data that is publicly available from TikTok. You are responsible for following the source site's terms, privacy and copyright rules, and the laws that apply to your use of the data. Only collect information you have a lawful basis to use.

### What limits should I expect?

Use the field constraints and limits in the input table. The source controls which records are available, so a narrow query or unavailable page can return fewer results or none. Keep batch lists small when you need a quick first run.

### Can I run it through the API or connect it to other tools?

Yes. Send the same JSON input to `POST https://api.apify.com/v2/actors/thescrappa~tiktok-comments-scraper/runs`. The response includes `defaultDatasetId`; use it to fetch the run's dataset items. See the [Apify Run Actor API](https://docs.apify.com/api/v2/actors-runs-post) and [Actor runs guide](https://docs.apify.com/api/v2/actors-actor-runs). Apify integrations for [Make](https://docs.apify.com/integrations/make), [Zapier](https://docs.apify.com/integrations/zapier), and [n8n](https://docs.apify.com/integrations/n8n) can trigger runs and pass results to the next workflow step. Send dataset rows to Google Sheets through a Sheets step or a workflow integration.

### What happens if a request fails?

Apify reports input validation and source request errors in the run details. Review the error, correct the input, and retry after a temporary source problem. Depending on when a request stops, the dataset may be empty or contain results collected before the failure.

## Related Scrappa Actors

- [TikTok Ads Scraper for Campaign Research](https://apify.com/thescrappa/tiktok-ads-scraper)
- [TikTok Hashtag & Challenge Details Scraper](https://apify.com/thescrappa/tiktok-challenge-details-scraper)
- [TikTok Hashtag Videos Scraper for Creator Research](https://apify.com/thescrappa/tiktok-challenge-posts-scraper)
- [TikTok Challenge Search Scraper for Trends](https://apify.com/thescrappa/tiktok-challenge-search-scraper)
- [TikTok Followers Scraper for Audience Research](https://apify.com/thescrappa/tiktok-followers-scraper)
