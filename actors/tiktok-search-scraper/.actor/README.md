# TikTok Search Scraper for Audience Research

The TikTok Search Scraper for Audience Research collects search results, names, and source links from TikTok. Provide a search phrase or a short list of phrases; the actor saves source fields such as `desc`, `author`, `digg_count`, and `comment_count` to an Apify dataset.

## What data can you extract?

The dataset contains fields returned by TikTok. The field names below match the Actor output schema. A source may leave optional values empty or omit fields when they are not available for a result.

| Field | Type | Description |
| --- | --- | --- |
| `desc` | text | Caption returned for this result. |
| `author` | object | Author returned for this result. |
| `digg_count` | number | Likes returned for this result. |
| `comment_count` | number | Comments returned for this result. |
| `share_count` | number | Shares returned for this result. |
| `play_count` | number | Views returned for this result. |
| `create_time` | number | Created returned for this result. |
| `aweme_id` | text | Aweme ID returned for this result. |
| `video_id` | text | Video ID returned for this result. |
| `request_keywords` | text | Search returned for this result. |
| `request_region` | text | Region returned for this result. |
| `request_cursor` | text | Cursor returned for this result. |
| `request_publish_time` | number | Publish Filter returned for this result. |
| `request_sort_type` | number | Sort returned for this result. |

## Use cases

- Collect search results, names, and source links for audience and content research.
- Review public profile, post, or engagement fields returned for each item.
- Export the dataset to a social reporting or creator workflow.

## How to use

1. Open the **Input** tab and use the example JSON below.
2. Change the query, URL, identifier, or other fields you need. For multi-target work, use `keywords` and start with a short list.
3. Start the Actor. Open the run's default dataset to inspect, download, or export the returned records.

```json
{
  "keywords": "basketball",
  "region": "US",
  "count": 10,
  "cursor": "0"
}
```

Apify stores the run output in a dataset. You can download the dataset in JSON, CSV, Excel, XML, or other available formats from the run page.

## Output example

This illustrative record uses synthetic values. It shows the real output field names; optional source values may be null or absent.

```json
{
  "desc": "Example value",
  "author": {},
  "digg_count": 42,
  "comment_count": 42,
  "share_count": 42,
  "play_count": 42,
  "create_time": 42
}
```

## Input fields

| Field | Type | Required | Description |
| --- | --- | --- | --- |
| `keywords` | string | No | Keyword, hashtag, product, brand, topic, or creator niche to search for in TikTok videos. You can also provide query instead when calling the actor API. |
| `query` | string | No | Optional alias for Search Keywords for API callers. Ignored when Search Keywords is provided. |
| `region` | string | No | Optional country or region code, such as US, GB, JP, or DE. |
| `count` | integer | No | Number of video results to return from this page. Scrappa accepts 1-50. Constraints: minimum 1; maximum 50. |
| `cursor` | string | No | Pagination cursor from a previous run. Leave empty for the first page. |
| `publish_time` | integer | No | Optional upstream publish-time filter. Leave empty to use TikTok's default search window; broad values can behave like no filter if TikTok has no matching retained results. Constraints: minimum 0; maximum 3650. |
| `sort_type` | integer | No | Optional upstream sort type. Leave empty to use TikTok's default ranking. Constraints: minimum 0; maximum 10. |

## Pricing

**Current live price:** $0.20 per 1,000 results.

The price is based on the latest live Apify pricing entry. Per-result charges depend on the number of billed results returned. If the price line lists subscription tiers, the rate shown for each tier applies to that Apify subscription level.

## FAQ

### Is it legal to scrape this data?

This Actor is intended for data that is publicly available from TikTok. You are responsible for following the source site's terms, privacy and copyright rules, and the laws that apply to your use of the data. Only collect information you have a lawful basis to use.

### What limits should I expect?

Use the field constraints and limits in the input table. The source controls which records are available, so a narrow query or unavailable page can return fewer results or none. Keep batch lists small when you need a quick first run.

### Can I run it through the API or connect it to other tools?

Yes. Send the same JSON input to `POST https://api.apify.com/v2/actors/thescrappa~tiktok-search-scraper/runs`. The response includes `defaultDatasetId`; use it to fetch the run's dataset items. See the [Apify Run Actor API](https://docs.apify.com/api/v2/actors-runs-post) and [Actor runs guide](https://docs.apify.com/api/v2/actors-actor-runs). Apify integrations for [Make](https://docs.apify.com/integrations/make), [Zapier](https://docs.apify.com/integrations/zapier), and [n8n](https://docs.apify.com/integrations/n8n) can trigger runs and pass results to the next workflow step. Send dataset rows to Google Sheets through a Sheets step or a workflow integration.

### What happens if a request fails?

Apify reports input validation and source request errors in the run details. Review the error, correct the input, and retry after a temporary source problem. Depending on when a request stops, the dataset may be empty or contain results collected before the failure.

## Related Scrappa Actors

- [TikTok Ads Scraper for Campaign Research](https://apify.com/thescrappa/tiktok-ads-scraper)
- [TikTok Hashtag & Challenge Details Scraper](https://apify.com/thescrappa/tiktok-challenge-details-scraper)
- [TikTok Hashtag Videos Scraper for Creator Research](https://apify.com/thescrappa/tiktok-challenge-posts-scraper)
- [TikTok Challenge Search Scraper for Trends](https://apify.com/thescrappa/tiktok-challenge-search-scraper)
- [TikTok Comments Scraper for Audience Research](https://apify.com/thescrappa/tiktok-comments-scraper)
