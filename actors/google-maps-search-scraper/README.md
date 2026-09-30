# Google Maps Search Scraper for Lead Research

The Google Maps Search Scraper for Lead Research collects local business details, ratings, and addresses from Google Maps. Provide a search phrase or a short list of phrases; the actor saves source fields such as `name`, `type`, `rating`, and `review_count` to an Apify dataset.

## What data can you extract?

The dataset contains fields returned by Google Maps. The field names below match the Actor output schema. A source may leave optional values empty or omit fields when they are not available for a result.

| Field | Type | Description |
| --- | --- | --- |
| `name` | text | Name returned for this result. |
| `type` | text | Type returned for this result. |
| `rating` | number | Rating returned for this result. |
| `review_count` | number | Reviews returned for this result. |
| `full_address` | text | Address returned for this result. |
| `latitude` | number | Latitude returned for this result. |
| `longitude` | number | Longitude returned for this result. |
| `phone_numbers` | array | Phone numbers returned for this result. |
| `website` | link | Website returned for this result. |
| `business_id` | text | Business ID returned for this result. |
| `place_id` | text | Place ID returned for this result. |
| `timezone` | text | Timezone returned for this result. |

## Use cases

- Collect local business details, ratings, and addresses to support lead generation.
- Compare records across the input queries or entities you provide.
- Export structured results to research and reporting workflows.

## How to use

1. Open the **Input** tab and use the example JSON below.
2. Change the query, URL, identifier, or other fields you need. Set the lookup fields to match the query or identifier you want to collect.
3. Start the Actor. Open the run's default dataset to inspect, download, or export the returned records.

```json
{
  "query": "starbucks times square new york",
  "gl": "us"
}
```

Apify stores the run output in a dataset. You can download the dataset in JSON, CSV, Excel, XML, or other available formats from the run page.

## Output example

This illustrative record uses synthetic values. It shows the real output field names; optional source values may be null or absent.

```json
{
  "name": "Example value",
  "type": "Example value",
  "rating": 4.7,
  "review_count": 42,
  "full_address": "Example location",
  "latitude": 40.7128,
  "longitude": -74.006
}
```

## Input fields

| Field | Type | Required | Description |
| --- | --- | --- | --- |
| `query` | string | Yes | What to search for (e.g., 'restaurants in NYC', 'coffee shops', 'plumber') |
| `hl` | string | No | Two-letter language code, optionally with a two-letter region (e.g., 'en', 'de', 'es', 'fr', 'en-US', 'de-DE') |
| `gl` | string | No | ISO 3166-1 alpha-2 country code for region-specific results (e.g., 'us', 'de', 'fr', 'uk') |
| `debug` | boolean | No | Enable Scrappa debug output for troubleshooting. This is only useful for accounts with debug access. |
| `use_cache` | boolean | No | Use cached results if available to reduce costs and speed up results |
| `maximum_cache_age` | integer | No | Maximum age of cached results in seconds. Set to 0 to always fetch fresh data. Constraints: minimum 0. |
| `fallback_zoom` | integer | No | Zoom level used if the simple search endpoint has a transient upstream failure and the actor retries through advanced search. Constraints: minimum 3; maximum 21. |

## Pricing

**Current live price:** $0.30 per 1,000 results.

The price is based on the latest live Apify pricing entry. Per-result charges depend on the number of billed results returned. If the price line lists subscription tiers, the rate shown for each tier applies to that Apify subscription level.

## FAQ

### Is it legal to scrape this data?

This Actor is intended for data that is publicly available from Google Maps. You are responsible for following the source site's terms, privacy and copyright rules, and the laws that apply to your use of the data. Only collect information you have a lawful basis to use.

### What limits should I expect?

Use the field constraints and limits in the input table. The source controls which records are available, so a narrow query or unavailable page can return fewer results or none. Keep batch lists small when you need a quick first run.

### Can I run it through the API or connect it to other tools?

Yes. Send the same JSON input to `POST https://api.apify.com/v2/actors/thescrappa~google-maps-search-scraper/runs`. The response includes `defaultDatasetId`; use it to fetch the run's dataset items. See the [Apify Run Actor API](https://docs.apify.com/api/v2/actors-runs-post) and [Actor runs guide](https://docs.apify.com/api/v2/actors-actor-runs). Apify integrations for [Make](https://docs.apify.com/integrations/make), [Zapier](https://docs.apify.com/integrations/zapier), and [n8n](https://docs.apify.com/integrations/n8n) can trigger runs and pass results to the next workflow step. Send dataset rows to Google Sheets through a Sheets step or a workflow integration.

### What happens if a request fails?

Apify reports input validation and source request errors in the run details. Review the error, correct the input, and retry after a temporary source problem. Depending on when a request stops, the dataset may be empty or contain results collected before the failure.

## Related Scrappa Actors

- [Google Maps Advanced Search Scraper for Sales](https://apify.com/thescrappa/google-maps-advanced-search-scraper)
- [Google Maps Autocomplete Scraper for Local Search](https://apify.com/thescrappa/google-maps-autocomplete-scraper)
- [Google Maps Business Details Scraper for Sales](https://apify.com/thescrappa/google-maps-business-details-scraper)
- [Google Maps Directions Scraper for Travel Planning](https://apify.com/thescrappa/google-maps-directions-scraper)
- [Google Maps Photos Scraper for Place Research](https://apify.com/thescrappa/google-maps-photos-scraper)
