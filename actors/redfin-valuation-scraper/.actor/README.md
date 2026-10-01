# Redfin Valuation Scraper

Check Redfin’s estimated home value and range alongside comparable property details. Provide a property ID or URL to get Redfin’s estimate and comparables when the home has coverage.

## What data can you extract?

The value range is an estimate from Redfin and does not represent an appraisal or guaranteed sale price.

| Field | Type | Description |
| --- | --- | --- |
| `success` | boolean | Whether the lookup completed successfully; false is a reported value, while null means Redfin provided no flag. |
| `status` | text | Status reported for the property listing by Redfin; null when Redfin does not provide the value. |
| `message` | text | Diagnostic text for the Redfin lookup; null when the request completes without an error. |
| `property_id` | number | property ID for the property listing, assigned by Redfin; null when the source does not expose it. |
| `listing_id` | number | listing ID for the property listing, assigned by Redfin; null when the source does not expose it. |
| `predicted_value` | number | Redfin’s estimated home value in US dollars; null when no estimate is available. |
| `predicted_value_low` | number | Low end of Redfin’s estimated home value range, in US dollars; null when the range is unavailable. |
| `predicted_value_high` | number | High end of Redfin’s estimated home value range, in US dollars; null when the range is unavailable. |
| `last_sold_price` | number | Last sold price for this property listing, as a numeric amount in the record currency; null when Redfin provides no price. |
| `last_sold_date` | date | Date the property last sold shown by Redfin, in YYYY-MM-DD when the source provides a calendar date; null if the source omits the date. |
| `beds` | number | Number of bedrooms shown by Redfin, as a whole number; zero is possible, and null means no count was reported. |
| `baths` | number | Number of bathrooms shown by Redfin, as a whole number; zero is possible, and null means no count was reported. |
| `sqft` | number | Interior area for this property listing, in area in square feet; null when Redfin does not supply the value. |
| `lot_size` | number | Lot area for this property listing, in area in square feet; null when Redfin does not supply the value. |
| `year_built` | number | Year built shown for the property listing by Redfin, in the format used by the source; null when it is omitted. |
| `comparables_count` | number | Number of comparable-homes shown by Redfin, as a whole number; zero is possible, and null means no count was reported. |
| `comparables` | text | Nearby homes with value and listing details from Redfin; null when the source provides no details. |
| `request_index` | number | Zero-based position of this request in the submitted Redfin input batch; null for a single-item lookup. |
| `request_property_id` | number | Property id passed to Redfin. This input value is copied into the output row; null when it was not supplied. |
| `request_listing_id` | number | Listing id passed to Redfin. This input value is copied into the output row; null when it was not supplied. |
| `request_url` | link | Source page url passed to Redfin. This input value is copied into the output row; null when it was not supplied. |

## Use cases

- Agents can compare asking prices, room counts and floor area in a target market.
- Researchers can review homes by location before building a market snapshot.
- Search teams can collect listing details for a property shortlist.

## How to use

1. Open the Actor’s **Input** tab and start with the JSON below.
2. Put the supported targets in `property_ids` and use the identifier or URL format required by Redfin.
3. Start the run and open its default dataset to inspect or download the rows.

```json
{
  "property_ids": [
    194191988
  ]
}
```

The run dataset can be downloaded as JSON, CSV, Excel or another format offered by Apify.

## Input

| Field | Type | Required | Description |
| --- | --- | --- | --- |
| `property_id` | integer | No | Redfin property ID, usually the number after /home/ in a Redfin URL. Constraints: minimum 1. |
| `listing_id` | integer | No | Optional Redfin listing ID. Usually not needed; include it when you already have a listing-specific ID from Redfin search or property details. Constraints: minimum 1. |
| `url` | string | No | Optional Redfin property URL. The actor extracts the property ID from URLs containing /home/{property_id}. |
| `property_ids` | array of integer | No | Run multiple Redfin property valuations in one Apify run. When provided, top-level property_id, listing_id, and url fields are ignored. Constraints: maximum 50 items. |
| `properties` | array of object | No | Run multiple Redfin valuations with per-property listing IDs or URLs. When provided, this takes precedence over property_ids. Constraints: maximum 50 items. |

## Output example

The record below is synthetic. Names and links are examples, and private contact fields are omitted.

```json
{
  "predicted_value": 875000,
  "status": "active",
  "property_id": 31245678,
  "listing_id": 84012657,
  "predicted_value_low": 840000,
  "predicted_value_high": 910000,
  "last_sold_price": 128.5,
  "last_sold_date": "2026-08-14"
}
```

## Pricing

**Current live price:** $0.50 per 1,000 results.

Each saved property record counts as one result.

## FAQ

### Is it legal to collect public information?

This Actor is for information visible on the public source pages it reads. You are responsible for checking the source terms, privacy and copyright rules, and the laws that apply to your use.

### How many records will a run return?

The result count depends on the input limits, pagination settings and what the source makes available. A small query or unavailable page can return fewer records, including none.

### Can I call it through the API or connect it to other tools?

Yes. Send the same JSON input to `POST https://api.apify.com/v2/acts/thescrappa~redfin-valuation-scraper/runs`, then read the run’s default dataset. Make, Zapier and n8n can start runs and pass dataset rows to the next step; Google Sheets can receive rows through those workflows or a dataset export.

### What happens when a request fails?

Check the run log for the source or input error, correct the input and retry after a temporary source issue. A run can contain rows saved before a later request failed.

### Can Redfin Valuation use a listing ID instead of a property ID?

Yes. Provide a supported `property_id`, `listing_id` or URL. A valuation and comparable homes are returned only when Redfin has data for that property.

## Related Scrappa Actors

- [ImmobilienScout24 Location Autocomplete Scraper](https://apify.com/thescrappa/immobilienscout24-locations-scraper)
- [ImmobilienScout24 Price Insights Scraper](https://apify.com/thescrappa/immobilienscout24-price-insights-scraper)
- [ImmobilienScout24 Search Scraper](https://apify.com/thescrappa/immobilienscout24-search-scraper)
- [Immowelt Property Search Scraper](https://apify.com/thescrappa/immowelt-property-search-scraper)
- [Redfin Property Details Scraper](https://apify.com/thescrappa/redfin-property-details-scraper)
