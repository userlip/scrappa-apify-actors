# Redfin Property Details Scraper for Real Estate

The Redfin Property Details Scraper for Real Estate collects public record details and identifying fields from Redfin. Provide one or more public URLs; the actor saves source fields such as `property_id`, `address`, `city`, and `state` to an Apify dataset.

## What data can you extract?

The dataset contains fields returned by Redfin. The field names below match the Actor output schema. A source may leave optional values empty or omit fields when they are not available for a result.

| Field | Type | Description |
| --- | --- | --- |
| `property_id` | number | Property ID returned for this result. |
| `address` | text | Address returned for this result. |
| `city` | text | City returned for this result. |
| `state` | text | State returned for this result. |
| `zip` | text | ZIP returned for this result. |
| `country` | text | Country returned for this result. |
| `price` | number | Price returned for this result. |
| `price_label` | text | Price Label returned for this result. |
| `beds` | number | Beds returned for this result. |
| `baths` | number | Baths returned for this result. |
| `sqft` | number | Square Feet returned for this result. |
| `lot_size` | number | Lot Size returned for this result. |
| `year_built` | number | Year Built returned for this result. |
| `property_type` | number | Property Type ID returned for this result. |
| `status` | number | Status ID returned for this result. |
| `status_label` | text | Status returned for this result. |
| `latitude` | number | Latitude returned for this result. |
| `longitude` | number | Longitude returned for this result. |
| `url` | link | Redfin URL returned for this result. |
| `description` | text | Description returned for this result. |
| `photos` | object | Photos returned for this result. |
| `request_property_index` | number | Request Index returned for this result. |
| `request_property_id` | number | Requested Property ID returned for this result. |
| `request_input` | text | Request Input returned for this result. |
| `request_source` | text | Request Source returned for this result. |
| `success` | boolean | Success returned for this result. |
| `error` | text | Error returned for this result. |
| `status_code` | number | Status Code returned for this result. |

## Use cases

- Collect public record details and identifying fields for a target area or property search.
- Compare listing, price, and location fields across a set of properties.
- Prepare property research exports for spreadsheets or market reports.

## How to use

1. Open the **Input** tab and use the example JSON below.
2. Change the query, URL, identifier, or other fields you need. For multi-target work, use `property_ids` and start with a short list.
3. Start the Actor. Open the run's default dataset to inspect, download, or export the returned records.

```json
{
  "url": "https://www.redfin.com/TN/Memphis/1549-Ely-St-38106/home/60791456",
  "property_ids": [
    60791456
  ]
}
```

Apify stores the run output in a dataset. You can download the dataset in JSON, CSV, Excel, XML, or other available formats from the run page.

## Output example

This illustrative record uses synthetic values. It shows the real output field names; optional source values may be null or absent.

```json
{
  "property_id": 42,
  "address": "Example location",
  "city": "New York",
  "state": "Example value",
  "zip": "Example value",
  "country": "42",
  "price": 129.99
}
```

## Input fields

| Field | Type | Required | Description |
| --- | --- | --- | --- |
| `property_id` | integer | No | Single Redfin property ID. Example: 60791456 from a Redfin URL ending in /home/60791456. Constraints: minimum 1. |
| `property_ids` | array of integer | No | Batch of Redfin property IDs to process in one Apify run. Constraints: maximum 100 items. |
| `url` | string | No | Single Redfin property URL containing /home/{property_id}. |
| `urls` | array of string | No | Batch of Redfin property URLs containing /home/{property_id}. Constraints: maximum 100 items. |

## Pricing

**Current live price:** $0.30 per 1,000 results.

The price is based on the latest live Apify pricing entry. Per-result charges depend on the number of billed results returned. If the price line lists subscription tiers, the rate shown for each tier applies to that Apify subscription level.

## FAQ

### Is it legal to scrape this data?

This Actor is intended for data that is publicly available from Redfin. You are responsible for following the source site's terms, privacy and copyright rules, and the laws that apply to your use of the data. Only collect information you have a lawful basis to use.

### What limits should I expect?

Use the field constraints and limits in the input table. The source controls which records are available, so a narrow query or unavailable page can return fewer results or none. Keep batch lists small when you need a quick first run.

### Can I run it through the API or connect it to other tools?

Yes. Send the same JSON input to `POST https://api.apify.com/v2/actors/thescrappa~redfin-property-details-scraper/runs`. The response includes `defaultDatasetId`; use it to fetch the run's dataset items. See the [Apify Run Actor API](https://docs.apify.com/api/v2/actors-runs-post) and [Actor runs guide](https://docs.apify.com/api/v2/actors-actor-runs). Apify integrations for [Make](https://docs.apify.com/integrations/make), [Zapier](https://docs.apify.com/integrations/zapier), and [n8n](https://docs.apify.com/integrations/n8n) can trigger runs and pass results to the next workflow step. Send dataset rows to Google Sheets through a Sheets step or a workflow integration.

### What happens if a request fails?

Apify reports input validation and source request errors in the run details. Review the error, correct the input, and retry after a temporary source problem. Depending on when a request stops, the dataset may be empty or contain results collected before the failure.

## Related Scrappa Actors

- [ImmobilienScout24 Location Autocomplete Scraper](https://apify.com/thescrappa/immobilienscout24-locations-scraper)
- [ImmobilienScout24 Price Insights Scraper](https://apify.com/thescrappa/immobilienscout24-price-insights-scraper)
- [ImmobilienScout24 Search Scraper for Real Estate](https://apify.com/thescrappa/immobilienscout24-search-scraper)
- [Immowelt Property Search Scraper for Real Estate](https://apify.com/thescrappa/immowelt-property-search-scraper)
- [Redfin Property Search Scraper for Property Buyers](https://apify.com/thescrappa/redfin-property-search-scraper)
