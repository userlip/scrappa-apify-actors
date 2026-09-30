# TikTok Ads Scraper for Campaign Research

The TikTok Ads Scraper for Campaign Research collects TikTok ad creative and campaign details from TikTok. Provide one or more public URLs; the actor saves source fields such as `ad_id`, `advertiser_name`, `brand_name`, and `advertiser_id` to an Apify dataset.

## What data can you extract?

The dataset contains fields returned by TikTok. The field names below match the Actor output schema. A source may leave optional values empty or omit fields when they are not available for a result.

| Field | Type | Description |
| --- | --- | --- |
| `ad_id` | text | Ad ID returned for this result. |
| `advertiser_name` | text | Advertiser returned for this result. |
| `brand_name` | text | Brand returned for this result. |
| `advertiser_id` | text | Advertiser ID returned for this result. |
| `account_name` | text | Account returned for this result. |
| `industry` | text | Industry returned for this result. |
| `objective` | text | Objective returned for this result. |
| `creative_text` | text | Creative Text returned for this result. |
| `landing_page` | link | Landing Page returned for this result. |
| `destination` | link | Destination returned for this result. |
| `cta` | text | CTA returned for this result. |
| `video_url` | link | Video URL returned for this result. |
| `cover` | link | Cover returned for this result. |
| `media_urls` | array | Media URLs returned for this result. |
| `region` | text | Region returned for this result. |
| `country` | text | Country returned for this result. |
| `language` | text | Language returned for this result. |
| `category` | text | Category returned for this result. |
| `like_count` | number | Likes returned for this result. |
| `comment_count` | number | Comments returned for this result. |
| `share_count` | number | Shares returned for this result. |
| `cached` | boolean | Cached returned for this result. |
| `request_url` | link | Request URL returned for this result. |
| `request_ad_id` | text | Request Ad ID returned for this result. |
| `request_index` | number | Request # returned for this result. |
| `result_found` | boolean | Found returned for this result. |
| `error_message` | text | Error returned for this result. |

## Use cases

- Collect TikTok ad creative and campaign details for audience and content research.
- Review public profile, post, or engagement fields returned for each item.
- Export the dataset to a social reporting or creator workflow.

## How to use

1. Open the **Input** tab and use the example JSON below.
2. Change the query, URL, identifier, or other fields you need. For multi-target work, use `urls` and start with a short list.
3. Start the Actor. Open the run's default dataset to inspect, download, or export the returned records.

```json
{
  "urls": [
    "https://ads.tiktok.com/business/creativecenter/topads/7213160569871581185/pc/en?countryCode=US&period=30"
  ]
}
```

Apify stores the run output in a dataset. You can download the dataset in JSON, CSV, Excel, XML, or other available formats from the run page.

## Output example

This illustrative record uses synthetic values. It shows the real output field names; optional source values may be null or absent.

```json
{
  "ad_id": "example-123",
  "advertiser_name": "Example value",
  "brand_name": "Example value",
  "advertiser_id": "example-123",
  "account_name": "42",
  "industry": "Example value",
  "objective": "Example value"
}
```

## Input fields

| Field | Type | Required | Description |
| --- | --- | --- | --- |
| `urls` | array of string | No | One or more TikTok Creative Center ad URLs. The actor pushes one dataset item for each requested ad URL. Constraints: minimum 1 items; maximum 100 items. |
| `url` | string | No | Legacy single URL field for API callers. Ignored when TikTok Creative Center Ad URLs is provided. |

## Pricing

**Current live price:** $0.20 per 1,000 results.

The price is based on the latest live Apify pricing entry. Per-result charges depend on the number of billed results returned. If the price line lists subscription tiers, the rate shown for each tier applies to that Apify subscription level.

## FAQ

### Is it legal to scrape this data?

This Actor is intended for data that is publicly available from TikTok. You are responsible for following the source site's terms, privacy and copyright rules, and the laws that apply to your use of the data. Only collect information you have a lawful basis to use.

### What limits should I expect?

Use the field constraints and limits in the input table. The source controls which records are available, so a narrow query or unavailable page can return fewer results or none. Keep batch lists small when you need a quick first run.

### Can I run it through the API or connect it to other tools?

Yes. Send the same JSON input to `POST https://api.apify.com/v2/actors/thescrappa~tiktok-ads-scraper/runs`. The response includes `defaultDatasetId`; use it to fetch the run's dataset items. See the [Apify Run Actor API](https://docs.apify.com/api/v2/actors-runs-post) and [Actor runs guide](https://docs.apify.com/api/v2/actors-actor-runs). Apify integrations for [Make](https://docs.apify.com/integrations/make), [Zapier](https://docs.apify.com/integrations/zapier), and [n8n](https://docs.apify.com/integrations/n8n) can trigger runs and pass results to the next workflow step. Send dataset rows to Google Sheets through a Sheets step or a workflow integration.

### What happens if a request fails?

Apify reports input validation and source request errors in the run details. Review the error, correct the input, and retry after a temporary source problem. Depending on when a request stops, the dataset may be empty or contain results collected before the failure.

## Related Scrappa Actors

- [TikTok Hashtag & Challenge Details Scraper](https://apify.com/thescrappa/tiktok-challenge-details-scraper)
- [TikTok Hashtag Videos Scraper for Creator Research](https://apify.com/thescrappa/tiktok-challenge-posts-scraper)
- [TikTok Challenge Search Scraper for Trends](https://apify.com/thescrappa/tiktok-challenge-search-scraper)
- [TikTok Comments Scraper for Audience Research](https://apify.com/thescrappa/tiktok-comments-scraper)
- [TikTok Followers Scraper for Audience Research](https://apify.com/thescrappa/tiktok-followers-scraper)
