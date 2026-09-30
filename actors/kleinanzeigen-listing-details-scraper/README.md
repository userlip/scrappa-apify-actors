# Kleinanzeigen Listing Details Scraper for Buyers

The Kleinanzeigen Listing Details Scraper for Buyers collects public record details and identifying fields from Kleinanzeigen. Provide a search phrase or a short list of phrases; the actor saves source fields such as `id`, `title`, `price`, and `price_numeric` to an Apify dataset.

## What data can you extract?

The dataset contains fields returned by Kleinanzeigen. The field names below match the Actor output schema. A source may leave optional values empty or omit fields when they are not available for a result.

| Field | Type | Description |
| --- | --- | --- |
| `id` | text | Listing ID returned for this result. |
| `title` | text | Title returned for this result. |
| `price` | text | Price returned for this result. |
| `price_numeric` | number | Price Numeric returned for this result. |
| `description` | text | Description returned for this result. |
| `location` | text | Location returned for this result. |
| `images` | array | images returned for this result. |
| `seller` | text | seller returned for this result. |
| `attributes` | text | attributes returned for this result. |
| `shipping` | text | shipping returned for this result. |
| `posted_at` | text | Posted At returned for this result. |
| `categories` | text | categories returned for this result. |
| `request_ad_id` | text | Requested ID returned for this result. |
| `request_index` | number | Request Index returned for this result. |

## Use cases

- Collect public record details and identifying fields to research product availability and pricing.
- Compare item, seller, and listing details across a small search batch.
- Export marketplace records for catalog or resale analysis.

## How to use

1. Open the **Input** tab and use the example JSON below.
2. Change the query, URL, identifier, or other fields you need. For multi-target work, use `ad_ids` and start with a short list.
3. Start the Actor. Open the run's default dataset to inspect, download, or export the returned records.

```json
{
  "query": "fahrrad",
  "ad_ids": [
    "1"
  ]
}
```

Apify stores the run output in a dataset. You can download the dataset in JSON, CSV, Excel, XML, or other available formats from the run page.

## Output example

This illustrative record uses synthetic values. It shows the real output field names; optional source values may be null or absent.

```json
{
  "id": "example-123",
  "title": "Example result",
  "price": "129.99",
  "price_numeric": 129.99,
  "description": "Example public text.",
  "location": "Example location",
  "images": []
}
```

## Input fields

| Field | Type | Required | Description |
| --- | --- | --- | --- |
| `ad_id` | string | No | A single Kleinanzeigen listing ID. It can be combined with ad_ids. |
| `ad_ids` | array of string | No | Optional batch of up to 100 listing IDs. Duplicate IDs are fetched once in first-seen order. Constraints: maximum 100 items. |
| `query` | string | No | When no ad_id or ad_ids is supplied, search this query and save the first successful listing detail (up to three candidates). Explicit IDs take priority. |

## Pricing

**Current live price:** $0.25 per 1,000 results.

The price is based on the latest live Apify pricing entry. Per-result charges depend on the number of billed results returned. If the price line lists subscription tiers, the rate shown for each tier applies to that Apify subscription level.

## FAQ

### Is it legal to scrape this data?

This Actor is intended for data that is publicly available from Kleinanzeigen. You are responsible for following the source site's terms, privacy and copyright rules, and the laws that apply to your use of the data. Only collect information you have a lawful basis to use.

### What limits should I expect?

Use the field constraints and limits in the input table. The source controls which records are available, so a narrow query or unavailable page can return fewer results or none. Keep batch lists small when you need a quick first run.

### Can I run it through the API or connect it to other tools?

Yes. Send the same JSON input to `POST https://api.apify.com/v2/actors/thescrappa~kleinanzeigen-listing-details-scraper/runs`. The response includes `defaultDatasetId`; use it to fetch the run's dataset items. See the [Apify Run Actor API](https://docs.apify.com/api/v2/actors-runs-post) and [Actor runs guide](https://docs.apify.com/api/v2/actors-actor-runs). Apify integrations for [Make](https://docs.apify.com/integrations/make), [Zapier](https://docs.apify.com/integrations/zapier), and [n8n](https://docs.apify.com/integrations/n8n) can trigger runs and pass results to the next workflow step. Send dataset rows to Google Sheets through a Sheets step or a workflow integration.

### What happens if a request fails?

Apify reports input validation and source request errors in the run details. Review the error, correct the input, and retry after a temporary source problem. Depending on when a request stops, the dataset may be empty or contain results collected before the failure.

## Related Scrappa Actors

- [Kleinanzeigen Search Scraper for Product Research](https://apify.com/thescrappa/kleinanzeigen-search-scraper)
- [Vinted Item Details Scraper for Product Research](https://apify.com/thescrappa/vinted-item-details-scraper)
- [Vinted Search Scraper for Product Research](https://apify.com/thescrappa/vinted-search-scraper)
- [Vinted User Items Scraper for Product Research](https://apify.com/thescrappa/vinted-user-items-scraper)
- [Vinted User Profile Scraper for Seller Research](https://apify.com/thescrappa/vinted-user-profile-scraper)
