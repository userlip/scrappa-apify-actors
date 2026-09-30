# Redfin Valuation Scraper for Property Research

The Redfin Valuation Scraper for Property Research collects property estimates, price ranges, and market metrics from Redfin. Provide one or more public URLs; the actor saves source fields such as `success`, `status`, `message`, and `property_id` to an Apify dataset.

## What data can you extract?

The dataset contains fields returned by Redfin. The field names below match the Actor output schema. A source may leave optional values empty or omit fields when they are not available for a result.

| Field | Type | Description |
| --- | --- | --- |
| `success` | boolean | Success returned for this result. |
| `status` | text | Status returned for this result. |
| `message` | text | Message returned for this result. |
| `property_id` | number | Property ID returned for this result. |
| `listing_id` | number | Listing ID returned for this result. |
| `predicted_value` | number | Predicted Value returned for this result. |
| `predicted_value_low` | number | Predicted Low returned for this result. |
| `predicted_value_high` | number | Predicted High returned for this result. |
| `last_sold_price` | number | Last Sold Price returned for this result. |
| `last_sold_date` | date | Last Sold Date returned for this result. |
| `beds` | number | Beds returned for this result. |
| `baths` | number | Baths returned for this result. |
| `sqft` | number | Square Feet returned for this result. |
| `lot_size` | number | Lot Size returned for this result. |
| `year_built` | number | Year Built returned for this result. |
| `comparables_count` | number | Comparables returned for this result. |
| `comparables` | text | comparables returned for this result. |
| `request_index` | number | Request Index returned for this result. |
| `request_property_id` | number | Request Property ID returned for this result. |
| `request_listing_id` | number | Request Listing ID returned for this result. |
| `request_url` | link | Request URL returned for this result. |

## Use cases

- Collect property estimates, price ranges, and market metrics for a target area or property search.
- Compare listing, price, and location fields across a set of properties.
- Prepare property research exports for spreadsheets or market reports.

## How to use

1. Open the **Input** tab and use the example JSON below.
2. Change the query, URL, identifier, or other fields you need. For multi-target work, use `property_ids` and start with a short list.
3. Start the Actor. Open the run's default dataset to inspect, download, or export the returned records.

```json
{
  "property_ids": [
    194191988
  ]
}
```

Apify stores the run output in a dataset. You can download the dataset in JSON, CSV, Excel, XML, or other available formats from the run page.

## Output example

This illustrative record uses synthetic values. It shows the real output field names; optional source values may be null or absent.

```json
{
  "success": true,
  "status": "Example value",
  "message": "Example value",
  "property_id": 42,
  "listing_id": 42,
  "predicted_value": 42,
  "predicted_value_low": 42
}
```

## Input fields

| Field | Type | Required | Description |
| --- | --- | --- | --- |
| `property_id` | integer | No | Redfin property ID, usually the number after /home/ in a Redfin URL. Constraints: minimum 1. |
| `listing_id` | integer | No | Optional Redfin listing ID. Usually not needed; include it when you already have a listing-specific ID from Redfin search or property details. Constraints: minimum 1. |
| `url` | string | No | Optional Redfin property URL. The actor extracts the property ID from URLs containing /home/{property_id}. |
| `property_ids` | array of integer | No | Run multiple Redfin property valuations in one Apify run. When provided, top-level property_id, listing_id, and url fields are ignored. Constraints: maximum 50 items. |
| `properties` | array of object | No | Run multiple Redfin valuations with per-property listing IDs or URLs. When provided, this takes precedence over property_ids. Constraints: maximum 50 items. |

## Pricing

**Current live price:** $0.50 per 1,000 results.

The price is based on the latest live Apify pricing entry. Per-result charges depend on the number of billed results returned. If the price line lists subscription tiers, the rate shown for each tier applies to that Apify subscription level.

## FAQ

### Is it legal to scrape this data?

This Actor is intended for data that is publicly available from Redfin. You are responsible for following the source site's terms, privacy and copyright rules, and the laws that apply to your use of the data. Only collect information you have a lawful basis to use.

### What limits should I expect?

Use the field constraints and limits in the input table. The source controls which records are available, so a narrow query or unavailable page can return fewer results or none. Keep batch lists small when you need a quick first run.

### Can I run it through the API or connect it to other tools?

Yes. Send the same JSON input to `POST https://api.apify.com/v2/actors/thescrappa~redfin-valuation-scraper/runs`. The response includes `defaultDatasetId`; use it to fetch the run's dataset items. See the [Apify Run Actor API](https://docs.apify.com/api/v2/actors-runs-post) and [Actor runs guide](https://docs.apify.com/api/v2/actors-actor-runs). Apify integrations for [Make](https://docs.apify.com/integrations/make), [Zapier](https://docs.apify.com/integrations/zapier), and [n8n](https://docs.apify.com/integrations/n8n) can trigger runs and pass results to the next workflow step. Send dataset rows to Google Sheets through a Sheets step or a workflow integration.

### What happens if a request fails?

Apify reports input validation and source request errors in the run details. Review the error, correct the input, and retry after a temporary source problem. Depending on when a request stops, the dataset may be empty or contain results collected before the failure.

## Related Scrappa Actors

- [ImmobilienScout24 Location Autocomplete Scraper](https://apify.com/thescrappa/immobilienscout24-locations-scraper)
- [ImmobilienScout24 Price Insights Scraper](https://apify.com/thescrappa/immobilienscout24-price-insights-scraper)
- [ImmobilienScout24 Search Scraper for Real Estate](https://apify.com/thescrappa/immobilienscout24-search-scraper)
- [Immowelt Property Search Scraper for Real Estate](https://apify.com/thescrappa/immowelt-property-search-scraper)
- [Redfin Property Details Scraper for Real Estate](https://apify.com/thescrappa/redfin-property-details-scraper)
