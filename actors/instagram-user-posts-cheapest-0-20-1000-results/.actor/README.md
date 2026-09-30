# Instagram User Posts Scraper for Audience Research

The Instagram User Posts Scraper for Audience Research collects public post text, media links, and engagement counts from Instagram. Provide the fields listed below; the actor saves source fields such as `request_username`, `id`, `username`, and `shortcode` to an Apify dataset.

## What data can you extract?

The dataset contains fields returned by Instagram. The field names below match the Actor output schema. A source may leave optional values empty or omit fields when they are not available for a result.

| Field | Type | Description |
| --- | --- | --- |
| `request_username` | text | Requested User returned for this result. |
| `id` | text | Post ID returned for this result. |
| `username` | text | Username returned for this result. |
| `shortcode` | text | Shortcode returned for this result. |
| `media_type` | text | Type returned for this result. |
| `caption` | text | Caption returned for this result. |
| `taken_at` | date | Posted At returned for this result. |
| `like_count` | number | Likes returned for this result. |
| `comment_count` | number | Comments returned for this result. |
| `play_count` | number | Plays returned for this result. |
| `permalink` | link | Permalink returned for this result. |

## Use cases

- Collect public post text, media links, and engagement counts for audience and content research.
- Review public profile, post, or engagement fields returned for each item.
- Export the dataset to a social reporting or creator workflow.

## How to use

1. Open the **Input** tab and use the example JSON below.
2. Change the query, URL, identifier, or other fields you need. Set the lookup fields to match the query or identifier you want to collect.
3. Start the Actor. Open the run's default dataset to inspect, download, or export the returned records.

```json
{
  "username": "natgeo"
}
```

Apify stores the run output in a dataset. You can download the dataset in JSON, CSV, Excel, XML, or other available formats from the run page.

## Output example

This illustrative record uses synthetic values. It shows the real output field names; optional source values may be null or absent.

```json
{
  "request_username": "Sample Member",
  "id": "example-123",
  "username": "Sample Member",
  "shortcode": "Example value",
  "media_type": "Example value",
  "caption": "Example value",
  "taken_at": "2026-09-30T10:00:00Z"
}
```

## Input fields

| Field | Type | Required | Description |
| --- | --- | --- | --- |
| `username` | string | Yes | Instagram username without the @ symbol. |
| `max_id` | string | No | Optional pagination cursor from a previous response's next_max_id field. |

## Pricing

**Current live price:** Apify Free tier: $0.30 per 1,000 results; Bronze: $0.25 per 1,000 results; Silver: $0.22 per 1,000 results; Gold, Platinum, and Diamond: $0.20 per 1,000 results.

The price is based on the latest live Apify pricing entry. Per-result charges depend on the number of billed results returned. If the price line lists subscription tiers, the rate shown for each tier applies to that Apify subscription level.

## FAQ

### Is it legal to scrape this data?

This Actor is intended for data that is publicly available from Instagram. You are responsible for following the source site's terms, privacy and copyright rules, and the laws that apply to your use of the data. Only collect information you have a lawful basis to use.

### What limits should I expect?

Use the field constraints and limits in the input table. The source controls which records are available, so a narrow query or unavailable page can return fewer results or none. Keep batch lists small when you need a quick first run.

### Can I run it through the API or connect it to other tools?

Yes. Send the same JSON input to `POST https://api.apify.com/v2/actors/thescrappa~instagram-user-posts-cheapest-0-20-1000-results/runs`. The response includes `defaultDatasetId`; use it to fetch the run's dataset items. See the [Apify Run Actor API](https://docs.apify.com/api/v2/actors-runs-post) and [Actor runs guide](https://docs.apify.com/api/v2/actors-actor-runs). Apify integrations for [Make](https://docs.apify.com/integrations/make), [Zapier](https://docs.apify.com/integrations/zapier), and [n8n](https://docs.apify.com/integrations/n8n) can trigger runs and pass results to the next workflow step. Send dataset rows to Google Sheets through a Sheets step or a workflow integration.

### What happens if a request fails?

Apify reports input validation and source request errors in the run details. Review the error, correct the input, and retry after a temporary source problem. Depending on when a request stops, the dataset may be empty or contain results collected before the failure.

## Related Scrappa Actors

- [Instagram Post Info | Cheapest $0.20/1k results](https://apify.com/thescrappa/instagram-post-info-cheapest-0-20-1000-results)
- [Instagram User Info | Cheapest $0.20/1k results](https://apify.com/thescrappa/instagram-user-info-cheapest-0-20-1000-results)
- [Pinterest Search Scraper for Audience Research](https://apify.com/thescrappa/pinterest-search-scraper)
- [TikTok Search Scraper for Audience Research](https://apify.com/thescrappa/tiktok-search-scraper)
