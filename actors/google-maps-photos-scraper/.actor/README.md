# Google Maps Photos Scraper for Place Research

The Google Maps Photos Scraper for Place Research collects business photo records and image URLs from Google Maps. Provide the fields listed below; the actor saves source fields such as `photo_url_large`, `width`, `height`, and `contributor_name` to an Apify dataset.

## What data can you extract?

The dataset contains fields returned by Google Maps. The field names below match the Actor output schema. A source may leave optional values empty or omit fields when they are not available for a result.

| Field | Type | Description |
| --- | --- | --- |
| `photo_url_large` | link | Photo returned for this result. |
| `width` | number | Width returned for this result. |
| `height` | number | Height returned for this result. |
| `contributor_name` | text | Contributor returned for this result. |
| `posted_at` | text | Posted returned for this result. |
| `photo_id` | text | ID returned for this result. |

## Use cases

- Collect business photo records and image URLs to support lead generation.
- Compare records across the input queries or entities you provide.
- Export structured results to research and reporting workflows.

## How to use

1. Open the **Input** tab and use the example JSON below.
2. Change the query, URL, identifier, or other fields you need. For multi-target work, use `business_ids` and start with a short list.
3. Start the Actor. Open the run's default dataset to inspect, download, or export the returned records.

```json
{
  "business_ids": [
    "0x808fba02425dad8f:0x6c296c66619367e0",
    "ChIJj61dQgK6j4AR4GeTYWZsKWw"
  ]
}
```

Apify stores the run output in a dataset. You can download the dataset in JSON, CSV, Excel, XML, or other available formats from the run page.

## Output example

This illustrative record uses synthetic values. It shows the real output field names; optional source values may be null or absent.

```json
{
  "photo_url_large": "https://example.com/image.jpg",
  "width": 42,
  "height": 42,
  "contributor_name": "Example value",
  "posted_at": "2026-09-30T10:00:00Z",
  "photo_id": "https://example.com/image.jpg"
}
```

## Input fields

| Field | Type | Required | Description |
| --- | --- | --- | --- |
| `business_ids` | array of string | No | Recommended. Process many Google Maps business IDs, ChIJ... Place IDs, or supported Maps URLs in one Apify run so run startup and storage overhead are shared across photo results. Constraints: minimum 1 items; maximum 10 items. |
| `business_id` | string | No | Backward-compatible single Google Maps business ID, ChIJ... Place ID, or supported Maps URL. Prefer business_ids for normal usage, especially when processing more than one business. |
| `use_cache` | boolean | No | Use cached results if available to reduce costs and speed up results |
| `maximum_cache_age` | integer | No | Maximum age of cached results in seconds. Set to 0 to always fetch fresh data. Constraints: minimum 0. |

## Pricing

**Current live price:** $0.00005 per Actor Start event; plus $0.30 per 1,000 results.

The price is based on the latest live Apify pricing entry. Per-result charges depend on the number of billed results returned. If the price line lists subscription tiers, the rate shown for each tier applies to that Apify subscription level.

## FAQ

### Is it legal to scrape this data?

This Actor is intended for data that is publicly available from Google Maps. You are responsible for following the source site's terms, privacy and copyright rules, and the laws that apply to your use of the data. Only collect information you have a lawful basis to use.

### What limits should I expect?

Use the field constraints and limits in the input table. The source controls which records are available, so a narrow query or unavailable page can return fewer results or none. Keep batch lists small when you need a quick first run.

### Can I run it through the API or connect it to other tools?

Yes. Send the same JSON input to `POST https://api.apify.com/v2/actors/thescrappa~google-maps-photos-scraper/runs`. The response includes `defaultDatasetId`; use it to fetch the run's dataset items. See the [Apify Run Actor API](https://docs.apify.com/api/v2/actors-runs-post) and [Actor runs guide](https://docs.apify.com/api/v2/actors-actor-runs). Apify integrations for [Make](https://docs.apify.com/integrations/make), [Zapier](https://docs.apify.com/integrations/zapier), and [n8n](https://docs.apify.com/integrations/n8n) can trigger runs and pass results to the next workflow step. Send dataset rows to Google Sheets through a Sheets step or a workflow integration.

### What happens if a request fails?

Apify reports input validation and source request errors in the run details. Review the error, correct the input, and retry after a temporary source problem. Depending on when a request stops, the dataset may be empty or contain results collected before the failure.

## Related Scrappa Actors

- [Google Maps Advanced Search Scraper for Sales](https://apify.com/thescrappa/google-maps-advanced-search-scraper)
- [Google Maps Autocomplete Scraper for Local Search](https://apify.com/thescrappa/google-maps-autocomplete-scraper)
- [Google Maps Business Details Scraper for Sales](https://apify.com/thescrappa/google-maps-business-details-scraper)
- [Google Maps Directions Scraper for Travel Planning](https://apify.com/thescrappa/google-maps-directions-scraper)
- [Google Maps Reviews Scraper for Local Reputation](https://apify.com/thescrappa/google-maps-reviews-scraper)
