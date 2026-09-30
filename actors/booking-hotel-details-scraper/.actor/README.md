# Booking.com Hotel Details Scraper for Travel

The Booking.com Hotel Details Scraper for Travel collects hotel listings, availability, and property details from Booking.com. Provide one or more public URLs; the actor saves source fields such as `title`, `canonical_url`, `hotel_schema`, and `aggregate_rating` to an Apify dataset.

## What data can you extract?

The dataset contains fields returned by Booking.com. The field names below match the Actor output schema. A source may leave optional values empty or omit fields when they are not available for a result.

| Field | Type | Description |
| --- | --- | --- |
| `title` | text | Title returned for this result. |
| `canonical_url` | link | Canonical URL returned for this result. |
| `hotel_schema` | object | Hotel Schema returned for this result. |
| `aggregate_rating` | object | Aggregate Rating returned for this result. |
| `json_ld` | object | JSON-LD returned for this result. |
| `parsed` | boolean | Parsed returned for this result. |
| `request_index` | number | Request Index returned for this result. |
| `request_input_type` | text | Input Type returned for this result. |
| `request_url` | link | Request URL returned for this result. |
| `request_country` | text | Country returned for this result. |
| `request_slug` | text | Slug returned for this result. |
| `request_success` | boolean | Success returned for this result. |
| `error_message` | text | Error returned for this result. |

## Use cases

- Collect hotel listings, availability, and property details for a destination, route, or travel date.
- Compare returned options and details before planning a trip.
- Export travel records to a spreadsheet or booking research workflow.

## How to use

1. Open the **Input** tab and use the example JSON below.
2. Change the query, URL, identifier, or other fields you need. For multi-target work, use `urls` and start with a short list.
3. Start the Actor. Open the run's default dataset to inspect, download, or export the returned records.

```json
{
  "country": "fr",
  "slug": "ritz-paris",
  "urls": [
    "https://www.booking.com/hotel/fr/ritz-paris.html"
  ]
}
```

Apify stores the run output in a dataset. You can download the dataset in JSON, CSV, Excel, XML, or other available formats from the run page.

## Output example

This illustrative record uses synthetic values. It shows the real output field names; optional source values may be null or absent.

```json
{
  "title": "Example result",
  "canonical_url": "https://example.com/result/1",
  "hotel_schema": {},
  "aggregate_rating": {},
  "json_ld": {},
  "parsed": true,
  "request_index": 42
}
```

## Input fields

| Field | Type | Required | Description |
| --- | --- | --- | --- |
| `url` | string | No | Full Booking.com hotel URL for a single hotel detail request. If URL is provided, it takes precedence over Country and Slug. |
| `country` | string | No | Two-letter country code from the Booking.com hotel URL, such as fr, us, de, or gb. Use with Slug. |
| `slug` | string | No | Booking.com hotel slug, such as ritz-paris. The trailing .html is optional. |
| `urls` | array of string | No | Optional list of Booking.com hotel URLs to process in one actor run. Constraints: maximum 10 items. |
| `hotels` | array of object | No | Optional list of hotel request objects. Each item can include { "url" } or { "country", "slug" }. Constraints: maximum 10 items. |

## Pricing

**Current live price:** $0.30 per 1,000 results.

The price is based on the latest live Apify pricing entry. Per-result charges depend on the number of billed results returned. If the price line lists subscription tiers, the rate shown for each tier applies to that Apify subscription level.

## FAQ

### Is it legal to scrape this data?

This Actor is intended for data that is publicly available from Booking.com. You are responsible for following the source site's terms, privacy and copyright rules, and the laws that apply to your use of the data. Only collect information you have a lawful basis to use.

### What limits should I expect?

Use the field constraints and limits in the input table. The source controls which records are available, so a narrow query or unavailable page can return fewer results or none. Keep batch lists small when you need a quick first run.

### Can I run it through the API or connect it to other tools?

Yes. Send the same JSON input to `POST https://api.apify.com/v2/actors/thescrappa~booking-hotel-details-scraper/runs`. The response includes `defaultDatasetId`; use it to fetch the run's dataset items. See the [Apify Run Actor API](https://docs.apify.com/api/v2/actors-runs-post) and [Actor runs guide](https://docs.apify.com/api/v2/actors-actor-runs). Apify integrations for [Make](https://docs.apify.com/integrations/make), [Zapier](https://docs.apify.com/integrations/zapier), and [n8n](https://docs.apify.com/integrations/n8n) can trigger runs and pass results to the next workflow step. Send dataset rows to Google Sheets through a Sheets step or a workflow integration.

### What happens if a request fails?

Apify reports input validation and source request errors in the run details. Review the error, correct the input, and retry after a temporary source problem. Depending on when a request stops, the dataset may be empty or contain results collected before the failure.

## Related Scrappa Actors

- [Booking.com Search Scraper for Travel Planning](https://apify.com/thescrappa/booking-search-scraper)
- [Google Flights Search Scraper for Travel Planning](https://apify.com/thescrappa/google-flights-search-scraper)
- [Google Hotels Autocomplete Scraper for Stays](https://apify.com/thescrappa/google-hotels-autocomplete-scraper)
- [Google Hotels Search Scraper for Travel Planning](https://apify.com/thescrappa/google-hotels-search-scraper)
- [Google Maps Directions Scraper for Travel Planning](https://apify.com/thescrappa/google-maps-directions-scraper)
