# Immowelt Property Search Scraper for Real Estate

The Immowelt Property Search Scraper for Real Estate collects property listings, prices, and locations from Immowelt. Provide the fields listed below; the actor saves source fields such as `title`, `price`, `price_formatted`, and `rooms` to an Apify dataset.

## What data can you extract?

The dataset contains fields returned by Immowelt. The field names below match the Actor output schema. A source may leave optional values empty or omit fields when they are not available for a result.

| Field | Type | Description |
| --- | --- | --- |
| `title` | text | Title returned for this result. |
| `price` | number | Price returned for this result. |
| `price_formatted` | text | Price Text returned for this result. |
| `rooms` | number | Rooms returned for this result. |
| `rooms_max` | number | Rooms Max returned for this result. |
| `size_m2` | number | Size m2 returned for this result. |
| `size_m2_max` | number | Size m2 Max returned for this result. |
| `address` | text | Address returned for this result. |
| `latitude` | number | Latitude returned for this result. |
| `longitude` | number | Longitude returned for this result. |
| `url` | link | Expose URL returned for this result. |
| `online_id` | text | Online ID returned for this result. |
| `id` | text | Listing ID returned for this result. |
| `image_url` | image | Image returned for this result. |
| `is_private` | boolean | Private Seller returned for this result. |
| `published` | date | Published returned for this result. |
| `request_location` | text | Request Location returned for this result. |
| `request_type` | text | Search Type returned for this result. |
| `request_page` | number | Page returned for this result. |
| `request_per_page` | number | Per Page returned for this result. |

## Use cases

- Collect property listings, prices, and locations for a target area or property search.
- Compare listing, price, and location fields across a set of properties.
- Prepare property research exports for spreadsheets or market reports.

## How to use

1. Open the **Input** tab and use the example JSON below.
2. Change the query, URL, identifier, or other fields you need. Set the lookup fields to match the query or identifier you want to collect.
3. Start the Actor. Open the run's default dataset to inspect, download, or export the returned records.

```json
{
  "location": "Berlin",
  "page": 1,
  "per_page": 20
}
```

Apify stores the run output in a dataset. You can download the dataset in JSON, CSV, Excel, XML, or other available formats from the run page.

## Output example

This illustrative record uses synthetic values. It shows the real output field names; optional source values may be null or absent.

```json
{
  "title": "Example result",
  "price": 129.99,
  "price_formatted": "129.99",
  "rooms": 42,
  "rooms_max": 42,
  "size_m2": 42,
  "size_m2_max": 42
}
```

## Input fields

| Field | Type | Required | Description |
| --- | --- | --- | --- |
| `location` | string | Yes | City, district, postal code, or location query to search on Immowelt. |
| `type` | string | No | Scrappa Immowelt search type. Constraints: allowed values: apartment-rent, apartment-buy, house-rent, house-buy. |
| `page` | integer | No | Results page to fetch. Constraints: minimum 1; maximum 10000. |
| `per_page` | integer | No | Number of listings to request for this page. Constraints: minimum 1; maximum 50. |

## Pricing

**Current live price:** $0.30 per 1,000 results.

The price is based on the latest live Apify pricing entry. Per-result charges depend on the number of billed results returned. If the price line lists subscription tiers, the rate shown for each tier applies to that Apify subscription level.

## FAQ

### Is it legal to scrape this data?

This Actor is intended for data that is publicly available from Immowelt. You are responsible for following the source site's terms, privacy and copyright rules, and the laws that apply to your use of the data. Only collect information you have a lawful basis to use.

### What limits should I expect?

Use the field constraints and limits in the input table. The source controls which records are available, so a narrow query or unavailable page can return fewer results or none. Keep batch lists small when you need a quick first run.

### Can I run it through the API or connect it to other tools?

Yes. Send the same JSON input to `POST https://api.apify.com/v2/actors/thescrappa~immowelt-property-search-scraper/runs`. The response includes `defaultDatasetId`; use it to fetch the run's dataset items. See the [Apify Run Actor API](https://docs.apify.com/api/v2/actors-runs-post) and [Actor runs guide](https://docs.apify.com/api/v2/actors-actor-runs). Apify integrations for [Make](https://docs.apify.com/integrations/make), [Zapier](https://docs.apify.com/integrations/zapier), and [n8n](https://docs.apify.com/integrations/n8n) can trigger runs and pass results to the next workflow step. Send dataset rows to Google Sheets through a Sheets step or a workflow integration.

### What happens if a request fails?

Apify reports input validation and source request errors in the run details. Review the error, correct the input, and retry after a temporary source problem. Depending on when a request stops, the dataset may be empty or contain results collected before the failure.

## Related Scrappa Actors

- [ImmobilienScout24 Location Autocomplete Scraper](https://apify.com/thescrappa/immobilienscout24-locations-scraper)
- [ImmobilienScout24 Price Insights Scraper](https://apify.com/thescrappa/immobilienscout24-price-insights-scraper)
- [ImmobilienScout24 Search Scraper for Real Estate](https://apify.com/thescrappa/immobilienscout24-search-scraper)
- [Redfin Property Details Scraper for Real Estate](https://apify.com/thescrappa/redfin-property-details-scraper)
- [Redfin Property Search Scraper for Property Buyers](https://apify.com/thescrappa/redfin-property-search-scraper)
