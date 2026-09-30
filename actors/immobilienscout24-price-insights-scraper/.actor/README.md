# ImmobilienScout24 Price Insights Scraper

The ImmobilienScout24 Price Insights Scraper collects property estimates, price ranges, and market metrics from ImmobilienScout24. Provide the fields listed below; the actor saves source fields such as `location`, `geocode`, `currency`, and `apartment_rent_per_m2` to an Apify dataset.

## What data can you extract?

The dataset contains fields returned by ImmobilienScout24. The field names below match the Actor output schema. A source may leave optional values empty or omit fields when they are not available for a result.

| Field | Type | Description |
| --- | --- | --- |
| `location` | text | Resolved location returned for this result. |
| `geocode` | text | Geocode returned for this result. |
| `currency` | text | Currency returned for this result. |
| `apartment_rent_per_m2` | number | Apartment rent / m² returned for this result. |
| `apartment_buy_per_m2` | number | Apartment buy / m² returned for this result. |
| `house_rent_per_m2` | number | House rent / m² returned for this result. |
| `house_buy_per_m2` | number | House buy / m² returned for this result. |
| `request_location` | text | Requested location returned for this result. |
| `request_index` | number | Request index returned for this result. |

## Use cases

- Collect property estimates, price ranges, and market metrics for a target area or property search.
- Compare listing, price, and location fields across a set of properties.
- Prepare property research exports for spreadsheets or market reports.

## How to use

1. Open the **Input** tab and use the example JSON below.
2. Change the query, URL, identifier, or other fields you need. For multi-target work, use `locations` and start with a short list.
3. Start the Actor. Open the run's default dataset to inspect, download, or export the returned records.

```json
{
  "locations": [
    "Berlin"
  ]
}
```

Apify stores the run output in a dataset. You can download the dataset in JSON, CSV, Excel, XML, or other available formats from the run page.

## Output example

This illustrative record uses synthetic values. It shows the real output field names; optional source values may be null or absent.

```json
{
  "location": "Example location",
  "geocode": "Example value",
  "currency": "USD",
  "apartment_rent_per_m2": 42,
  "apartment_buy_per_m2": 42,
  "house_rent_per_m2": 42,
  "house_buy_per_m2": 42
}
```

## Input fields

| Field | Type | Required | Description |
| --- | --- | --- | --- |
| `locations` | array/string | No | German cities, districts, postal codes, or ImmobilienScout24 geocodes. Use an array or comma-separated text to batch locations in one run and reduce overhead. |
| `location` | string | No | Use this only for integrations that send one location. Ignored when locations is provided. |

## Pricing

**Current live price:** $0.50 per 1,000 results.

The price is based on the latest live Apify pricing entry. Per-result charges depend on the number of billed results returned. If the price line lists subscription tiers, the rate shown for each tier applies to that Apify subscription level.

## FAQ

### Is it legal to scrape this data?

This Actor is intended for data that is publicly available from ImmobilienScout24. You are responsible for following the source site's terms, privacy and copyright rules, and the laws that apply to your use of the data. Only collect information you have a lawful basis to use.

### What limits should I expect?

Use the field constraints and limits in the input table. The source controls which records are available, so a narrow query or unavailable page can return fewer results or none. Keep batch lists small when you need a quick first run.

### Can I run it through the API or connect it to other tools?

Yes. Send the same JSON input to `POST https://api.apify.com/v2/actors/thescrappa~immobilienscout24-price-insights-scraper/runs`. The response includes `defaultDatasetId`; use it to fetch the run's dataset items. See the [Apify Run Actor API](https://docs.apify.com/api/v2/actors-runs-post) and [Actor runs guide](https://docs.apify.com/api/v2/actors-actor-runs). Apify integrations for [Make](https://docs.apify.com/integrations/make), [Zapier](https://docs.apify.com/integrations/zapier), and [n8n](https://docs.apify.com/integrations/n8n) can trigger runs and pass results to the next workflow step. Send dataset rows to Google Sheets through a Sheets step or a workflow integration.

### What happens if a request fails?

Apify reports input validation and source request errors in the run details. Review the error, correct the input, and retry after a temporary source problem. Depending on when a request stops, the dataset may be empty or contain results collected before the failure.

## Related Scrappa Actors

- [ImmobilienScout24 Location Autocomplete Scraper](https://apify.com/thescrappa/immobilienscout24-locations-scraper)
- [ImmobilienScout24 Search Scraper for Real Estate](https://apify.com/thescrappa/immobilienscout24-search-scraper)
- [Immowelt Property Search Scraper for Real Estate](https://apify.com/thescrappa/immowelt-property-search-scraper)
- [Redfin Property Details Scraper for Real Estate](https://apify.com/thescrappa/redfin-property-details-scraper)
- [Redfin Property Search Scraper for Property Buyers](https://apify.com/thescrappa/redfin-property-search-scraper)
