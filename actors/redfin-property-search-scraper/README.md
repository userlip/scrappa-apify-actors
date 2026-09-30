# Redfin Property Search Scraper for Property Buyers

The Redfin Property Search Scraper for Property Buyers collects property listings, prices, and locations from Redfin. Provide a search phrase or a short list of phrases; the actor saves source fields such as `property_id`, `listing_id`, `address`, and `city` to an Apify dataset.

## What data can you extract?

The dataset contains fields returned by Redfin. The field names below match the Actor output schema. A source may leave optional values empty or omit fields when they are not available for a result.

| Field | Type | Description |
| --- | --- | --- |
| `property_id` | number | Property ID returned for this result. |
| `listing_id` | number | Listing ID returned for this result. |
| `address` | text | Address returned for this result. |
| `city` | text | City returned for this result. |
| `state` | text | State returned for this result. |
| `zip` | text | ZIP returned for this result. |
| `price` | number | Price returned for this result. |
| `beds` | number | Beds returned for this result. |
| `baths` | number | Baths returned for this result. |
| `sqft` | number | Square Feet returned for this result. |
| `lot_size` | number | Lot Size returned for this result. |
| `year_built` | number | Year Built returned for this result. |
| `property_type` | number | Property Type ID returned for this result. |
| `property_type_label` | text | Property Type returned for this result. |
| `status` | text | Status returned for this result. |
| `latitude` | number | Latitude returned for this result. |
| `longitude` | number | Longitude returned for this result. |
| `url` | link | Redfin URL returned for this result. |
| `mls_number` | text | MLS Number returned for this result. |
| `request_search_index` | number | Search Index returned for this result. |
| `request_region_id` | number | Request Region ID returned for this result. |
| `request_region_type` | number | Request Region Type returned for this result. |
| `request_market` | text | Request Market returned for this result. |
| `request_min_price` | number | Min Price returned for this result. |
| `request_max_price` | number | Max Price returned for this result. |
| `request_num_beds` | number | Min Beds returned for this result. |
| `request_num_baths` | number | Min Baths returned for this result. |
| `request_property_types` | text | Property Types returned for this result. |
| `request_status` | number | Request Status ID returned for this result. |
| `request_status_label` | text | Request Status returned for this result. |
| `request_sold_within_days` | number | Sold Within Days returned for this result. |
| `request_num_homes` | number | Requested Homes returned for this result. |
| `request_page` | number | Page returned for this result. |

## Use cases

- Collect property listings, prices, and locations for a target area or property search.
- Compare listing, price, and location fields across a set of properties.
- Prepare property research exports for spreadsheets or market reports.

## How to use

1. Open the **Input** tab and use the example JSON below.
2. Change the query, URL, identifier, or other fields you need. For multi-target work, use `searches` and start with a short list.
3. Start the Actor. Open the run's default dataset to inspect, download, or export the returned records.

```json
{
  "property_types": "1,2,3",
  "num_homes": 5,
  "page": 1,
  "searches": [
    {
      "region_id": 16163,
      "region_type": 6,
      "market": "seattle",
      "property_types": "1,2,3",
      "num_homes": 5,
      "page": 1
    }
  ]
}
```

Apify stores the run output in a dataset. You can download the dataset in JSON, CSV, Excel, XML, or other available formats from the run page.

## Output example

This illustrative record uses synthetic values. It shows the real output field names; optional source values may be null or absent.

```json
{
  "property_id": 42,
  "listing_id": 42,
  "address": "Example location",
  "city": "New York",
  "state": "Example value",
  "zip": "Example value",
  "price": 129.99
}
```

## Input fields

| Field | Type | Required | Description |
| --- | --- | --- | --- |
| `region_id` | integer | No | Region ID Constraints: minimum 1. |
| `region_type` | string | No | Redfin region type: 1=neighborhood, 2=ZIP, 4=postal code, 5=county, 6=city. Constraints: allowed values: 1, 2, 4, 5, 6. |
| `market` | string | No | Redfin market identifier from the locations endpoint, such as seattle, socal, dc, or nyc. |
| `min_price` | integer | No | Minimum listing price. Constraints: minimum 0. |
| `max_price` | integer | No | Maximum listing price. Constraints: minimum 0. |
| `num_beds` | integer | No | Minimum number of bedrooms. Constraints: minimum 0; maximum 10. |
| `num_baths` | number | No | Minimum number of bathrooms. Constraints: minimum 0; maximum 10. |
| `property_types` | string | No | Comma-separated Redfin property type IDs from 1-8. Leave blank for all types. |
| `status` | string | No | Listing status: 1=active, 9=all, 130=pending, 131=active plus pending. Constraints: allowed values: 1, 9, 130, 131. |
| `sold_within_days` | integer | No | Include properties sold within this many days. Required for some sold-listing workflows. Constraints: minimum 1; maximum 365. |
| `num_homes` | integer | No | Number of listings to return for each search. Constraints: minimum 1; maximum 450. |
| `page` | integer | No | Results page to fetch. Constraints: minimum 1. |
| `searches` | array of object | No | Run multiple Redfin region searches in one Apify run. When provided, top-level search fields are ignored. Constraints: maximum 25 items. |

## Pricing

**Current live price:** $0.30 per 1,000 results.

The price is based on the latest live Apify pricing entry. Per-result charges depend on the number of billed results returned. If the price line lists subscription tiers, the rate shown for each tier applies to that Apify subscription level.

## FAQ

### Is it legal to scrape this data?

This Actor is intended for data that is publicly available from Redfin. You are responsible for following the source site's terms, privacy and copyright rules, and the laws that apply to your use of the data. Only collect information you have a lawful basis to use.

### What limits should I expect?

Use the field constraints and limits in the input table. The source controls which records are available, so a narrow query or unavailable page can return fewer results or none. Keep batch lists small when you need a quick first run.

### Can I run it through the API or connect it to other tools?

Yes. Send the same JSON input to `POST https://api.apify.com/v2/actors/thescrappa~redfin-property-search-scraper/runs`. The response includes `defaultDatasetId`; use it to fetch the run's dataset items. See the [Apify Run Actor API](https://docs.apify.com/api/v2/actors-runs-post) and [Actor runs guide](https://docs.apify.com/api/v2/actors-actor-runs). Apify integrations for [Make](https://docs.apify.com/integrations/make), [Zapier](https://docs.apify.com/integrations/zapier), and [n8n](https://docs.apify.com/integrations/n8n) can trigger runs and pass results to the next workflow step. Send dataset rows to Google Sheets through a Sheets step or a workflow integration.

### What happens if a request fails?

Apify reports input validation and source request errors in the run details. Review the error, correct the input, and retry after a temporary source problem. Depending on when a request stops, the dataset may be empty or contain results collected before the failure.

## Related Scrappa Actors

- [ImmobilienScout24 Location Autocomplete Scraper](https://apify.com/thescrappa/immobilienscout24-locations-scraper)
- [ImmobilienScout24 Price Insights Scraper](https://apify.com/thescrappa/immobilienscout24-price-insights-scraper)
- [ImmobilienScout24 Search Scraper for Real Estate](https://apify.com/thescrappa/immobilienscout24-search-scraper)
- [Immowelt Property Search Scraper for Real Estate](https://apify.com/thescrappa/immowelt-property-search-scraper)
- [Redfin Property Details Scraper for Real Estate](https://apify.com/thescrappa/redfin-property-details-scraper)
