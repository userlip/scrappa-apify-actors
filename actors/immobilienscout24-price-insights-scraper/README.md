# ImmobilienScout24 Price Insights Scraper

Compare ImmobilienScout24 rent and sale price estimates per square meter for apartments and houses. Enter one or more city or district names to compare rent and sale estimates.

## What data can you extract?

These are area-level rent and sale estimates per square meter, not individual listing prices.

| Field | Type | Description |
| --- | --- | --- |
| `location` | text | Location shown for the property listing by ImmobilienScout24, in the format used by the source; null when it is omitted. |
| `geocode` | text | ImmobilienScout24 location code used to identify the city or district estimate; null when the response omits it. |
| `currency` | text | Currency code for this property listing, formatted as ImmobilienScout24 displays it, including the currency when shown; null when unavailable. |
| `apartment_rent_per_m2` | number | Estimated apartment rent per square meter in the source currency; null when no local estimate is available. |
| `apartment_buy_per_m2` | number | Estimated apartment sale price per square meter in the source currency; null when no local estimate is available. |
| `house_rent_per_m2` | number | Estimated house rent per square meter in the source currency; null when no local estimate is available. |
| `house_buy_per_m2` | number | Estimated house sale price per square meter in the source currency; null when no local estimate is available. |
| `request_location` | text | Location filter passed to ImmobilienScout24. This input value is copied into the output row; null when it was not supplied. |
| `request_index` | number | Zero-based position of this request in the submitted ImmobilienScout24 input batch; null for a single-item lookup. |

## Use cases

- Agents can compare asking prices, room counts and floor area in a target market.
- Researchers can review homes by location before building a market snapshot.
- Search teams can collect listing details for a property shortlist.

## How to use

1. Open the Actor’s **Input** tab and start with the JSON below.
2. Set `locations` to the search term or source identifier you want to look up, then use the optional filters listed below.
3. Start the run and open its default dataset to inspect or download the rows.

```json
{
  "locations": [
    "Berlin"
  ]
}
```

The run dataset can be downloaded as JSON, CSV, Excel or another format offered by Apify.

## Input

| Field | Type | Required | Description |
| --- | --- | --- | --- |
| `locations` | array/string | No | German cities, districts, postal codes, or ImmobilienScout24 geocodes. Use an array or comma-separated text to batch locations in one run and reduce overhead. |
| `location` | string | No | Use this only for integrations that send one location. Ignored when locations is provided. |

## Output example

The record below is synthetic. Names and links are examples, and private contact fields are omitted.

```json
{
  "location": "Seattle, WA",
  "geocode": "1276006",
  "currency": "USD",
  "apartment_rent_per_m2": 14.8,
  "apartment_buy_per_m2": 6240,
  "house_rent_per_m2": 12.6,
  "house_buy_per_m2": 5180
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

Yes. Send the same JSON input to `POST https://api.apify.com/v2/acts/thescrappa~immobilienscout24-price-insights-scraper/runs`, then read the run’s default dataset. Make, Zapier and n8n can start runs and pass dataset rows to the next step; Google Sheets can receive rows through those workflows or a dataset export.

### What happens when a request fails?

Check the run log for the source or input error, correct the input and retry after a temporary source issue. A run can contain rows saved before a later request failed.

### Can ImmobilienScout24 Price Insights compare several locations?

Yes. Provide a city or district name in each supported location entry. The returned estimates are source market values per square meter, when available.

## Related Scrappa Actors

- [ImmobilienScout24 Location Autocomplete Scraper](https://apify.com/thescrappa/immobilienscout24-locations-scraper)
- [ImmobilienScout24 Search Scraper](https://apify.com/thescrappa/immobilienscout24-search-scraper)
- [Immowelt Property Search Scraper](https://apify.com/thescrappa/immowelt-property-search-scraper)
- [Redfin Property Details Scraper](https://apify.com/thescrappa/redfin-property-details-scraper)
- [Redfin Property Search Scraper](https://apify.com/thescrappa/redfin-property-search-scraper)
