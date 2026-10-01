# Redfin Property Search Scraper

Search Redfin homes by market and filters, then compare addresses, prices and room counts. Set a Redfin market or region, then narrow the search by price, rooms or property status.

## What data can you extract?

Listing prices and estimates follow the current Redfin page and can change over time.

| Field | Type | Description |
| --- | --- | --- |
| `property_id` | number | property ID for the property listing, assigned by Redfin; null when the source does not expose it. |
| `listing_id` | number | listing ID for the property listing, assigned by Redfin; null when the source does not expose it. |
| `address` | text | Address shown for the property listing by Redfin, in the format used by the source; null when it is omitted. |
| `city` | text | City shown for the property listing by Redfin; null when Redfin does not provide the value. |
| `state` | text | State or region shown for the property listing by Redfin; null when Redfin does not provide the value. |
| `zip` | text | Zip shown for the property listing by Redfin, in the format used by the source; null when it is omitted. |
| `price` | number | Listed price for this property listing, as a numeric amount in the listing currency; null when Redfin provides no price. |
| `beds` | number | Number of bedrooms shown by Redfin, as a whole number; zero is possible, and null means no count was reported. |
| `baths` | number | Number of bathrooms shown by Redfin, as a whole number; zero is possible, and null means no count was reported. |
| `sqft` | number | Interior area for this property listing, in area in square feet; null when Redfin does not supply the value. |
| `lot_size` | number | Lot area for this property listing, in area in square feet; null when Redfin does not supply the value. |
| `year_built` | number | Year built shown for the property listing by Redfin, in the format used by the source; null when it is omitted. |
| `property_type` | number | Property type shown for the property listing by Redfin, in the format used by the source; null when it is omitted. |
| `property_type_label` | text | Category assigned to the property listing by Redfin; null when Redfin does not provide the value. |
| `status` | text | Status reported for the property listing by Redfin; null when Redfin does not provide the value. |
| `latitude` | number | Latitude for this property listing on Redfin, in decimal degrees; null when the source provides no coordinates. |
| `longitude` | number | Longitude for this property listing on Redfin, in decimal degrees; null when the source provides no coordinates. |
| `url` | link | Source page url for this property listing on Redfin; null when the source does not provide a URL. |
| `mls_number` | text | Mls number shown for the property listing by Redfin, in the format used by the source; null when it is omitted. |
| `request_search_index` | number | Search result index passed to Redfin. This input value is copied into the output row; null when it was not supplied. |
| `request_region_id` | number | Source region id passed to Redfin. This input value is copied into the output row; null when it was not supplied. |
| `request_region_type` | number | Region type passed to Redfin. This input value is copied into the output row; null when it was not supplied. |
| `request_market` | text | Market passed to Redfin. This input value is copied into the output row; null when it was not supplied. |
| `request_min_price` | number | Minimum price filter passed to Redfin. This input value is copied into the output row; null when it was not supplied. |
| `request_max_price` | number | Maximum price filter passed to Redfin. This input value is copied into the output row; null when it was not supplied. |
| `request_num_beds` | number | Minimum bedroom count passed to Redfin. This input value is copied into the output row; null when it was not supplied. |
| `request_num_baths` | number | Minimum bathroom count passed to Redfin. This input value is copied into the output row; null when it was not supplied. |
| `request_property_types` | text | Property type ids passed to Redfin. This input value is copied into the output row; null when it was not supplied. |
| `request_status` | number | Status filter passed to Redfin. This input value is copied into the output row; null when it was not supplied. |
| `request_status_label` | text | Status label passed to Redfin. This input value is copied into the output row; null when it was not supplied. |
| `request_sold_within_days` | number | Sale lookback in days passed to Redfin; A whole number of days. This input value is copied into the output row; null when it was not supplied. |
| `request_num_homes` | number | Maximum home count passed to Redfin. This input value is copied into the output row; null when it was not supplied. |
| `request_page` | number | Requested result page number passed to Redfin; A whole-number page number. This input value is copied into the output row; null when it was not supplied. |

## Use cases

- Agents can compare asking prices, room counts and floor area in a target market.
- Researchers can review homes by location before building a market snapshot.
- Search teams can collect listing details for a property shortlist.

## How to use

1. Open the Actor’s **Input** tab and start with the JSON below.
2. Put the supported targets in `searches` and use the identifier or URL format required by Redfin.
3. Start the run and open its default dataset to inspect or download the rows.

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

The run dataset can be downloaded as JSON, CSV, Excel or another format offered by Apify.

## Input

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

## Output example

The record below is synthetic. Names and links are examples, and private contact fields are omitted.

```json
{
  "price": 824000,
  "url": "https://listings.example.com/record/731-alder-way",
  "property_id": 31245678,
  "listing_id": 84012657,
  "address": "731 Alder Way, Seattle, WA 98103",
  "city": "Seattle",
  "state": "WA",
  "zip": "98101"
}
```

## Pricing

**Current live price:** $0.30 per 1,000 results.

Each saved property record counts as one result.

## FAQ

### Is it legal to collect public information?

This Actor is for information visible on the public source pages it reads. You are responsible for checking the source terms, privacy and copyright rules, and the laws that apply to your use.

### How many records will a run return?

The result count depends on the input limits, pagination settings and what the source makes available. A small query or unavailable page can return fewer records, including none.

### Can I call it through the API or connect it to other tools?

Yes. Send the same JSON input to `POST https://api.apify.com/v2/acts/thescrappa~redfin-property-search-scraper/runs`, then read the run’s default dataset. Make, Zapier and n8n can start runs and pass dataset rows to the next step; Google Sheets can receive rows through those workflows or a dataset export.

### What happens when a request fails?

Check the run log for the source or input error, correct the input and retry after a temporary source issue. A run can contain rows saved before a later request failed.

### How do I limit a Redfin search to a market?

Choose the supported `market` or provide `region_id` and `region_type`. You can then narrow listings with price, room, type and status filters.

## Related Scrappa Actors

- [ImmobilienScout24 Location Autocomplete Scraper](https://apify.com/thescrappa/immobilienscout24-locations-scraper)
- [ImmobilienScout24 Price Insights Scraper](https://apify.com/thescrappa/immobilienscout24-price-insights-scraper)
- [ImmobilienScout24 Search Scraper](https://apify.com/thescrappa/immobilienscout24-search-scraper)
- [Immowelt Property Search Scraper](https://apify.com/thescrappa/immowelt-property-search-scraper)
- [Redfin Property Details Scraper](https://apify.com/thescrappa/redfin-property-details-scraper)
