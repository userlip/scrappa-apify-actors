# Immowelt Property Search Scraper

Search Immowelt listings by location and property type, then compare prices, rooms and floor area. Enter a city or area and choose the property type and page options you need.

## What data can you extract?

Listing prices and property details follow the current Immowelt page and can change over time.

| Field | Type | Description |
| --- | --- | --- |
| `title` | text | Title of the property listing, as shown by Immowelt; null when no title is published. |
| `price` | number | Listed price for this property listing, as a numeric amount in the listing currency; null when Immowelt provides no price. |
| `price_formatted` | text | Displayed price for this property listing, formatted as Immowelt displays it, including the currency when shown; null when unavailable. |
| `rooms` | number | Number of rooms shown by Immowelt, as a whole number; zero is possible, and null means no count was reported. |
| `rooms_max` | number | Rooms max shown for the property listing by Immowelt, in the format used by the source; null when it is omitted. |
| `size_m2` | number | Floor area for this property listing, in area in square meters; null when Immowelt does not supply the value. |
| `size_m2_max` | number | Size m2 max shown for the property listing by Immowelt, in the format used by the source; null when it is omitted. |
| `address` | text | Address shown for the property listing by Immowelt, in the format used by the source; null when it is omitted. |
| `latitude` | number | Latitude for this property listing on Immowelt, in decimal degrees; null when the source provides no coordinates. |
| `longitude` | number | Longitude for this property listing on Immowelt, in decimal degrees; null when the source provides no coordinates. |
| `url` | link | Source page url for this property listing on Immowelt; null when the source does not provide a URL. |
| `online_id` | text | online id for the property listing, assigned by Immowelt; null when the source does not expose it. |
| `id` | text | source ID for the property listing, assigned by Immowelt; null when the source does not expose it. |
| `image_url` | image | Image url for this property listing on Immowelt; null when the source does not provide a URL. |
| `is_private` | boolean | Whether the account is private; false is a reported value, while null means Immowelt provided no flag. |
| `published` | date | Published for this property listing shown by Immowelt, in YYYY-MM-DD when the source provides a calendar date; null if the source omits the date. |
| `request_location` | text | Location filter passed to Immowelt. This input value is copied into the output row; null when it was not supplied. |
| `request_type` | text | Requested result type passed to Immowelt. This input value is copied into the output row; null when it was not supplied. |
| `request_page` | number | Requested result page number passed to Immowelt; A whole-number page number. This input value is copied into the output row; null when it was not supplied. |
| `request_per_page` | number | Number of results per page passed to Immowelt; A whole-number result count. This input value is copied into the output row; null when it was not supplied. |

## Use cases

- Agents can compare asking prices, room counts and floor area in a target market.
- Researchers can review homes by location before building a market snapshot.
- Search teams can collect listing details for a property shortlist.

## How to use

1. Open the Actor’s **Input** tab and start with the JSON below.
2. Set `location` to the search term or source identifier you want to look up, then use the optional filters listed below.
3. Start the run and open its default dataset to inspect or download the rows.

```json
{
  "location": "Berlin",
  "page": 1,
  "per_page": 20
}
```

The run dataset can be downloaded as JSON, CSV, Excel or another format offered by Apify.

## Input

| Field | Type | Required | Description |
| --- | --- | --- | --- |
| `location` | string | Yes | City, district, postal code, or location query to search on Immowelt. |
| `type` | string | No | Scrappa Immowelt search type. Constraints: allowed values: apartment-rent, apartment-buy, house-rent, house-buy. |
| `page` | integer | No | Results page to fetch. Constraints: minimum 1; maximum 10000. |
| `per_page` | integer | No | Number of listings to request for this page. Constraints: minimum 1; maximum 50. |

## Output example

The record below is synthetic. Names and links are examples, and private contact fields are omitted.

```json
{
  "title": "Two-bedroom Craftsman near Green Lake",
  "price": 824000,
  "url": "https://listings.example.com/record/731-alder-way",
  "price_formatted": "$824,000",
  "rooms": 2,
  "rooms_max": 3,
  "size_m2": 74.5,
  "size_m2_max": 96
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

Yes. Send the same JSON input to `POST https://api.apify.com/v2/acts/thescrappa~immowelt-property-search-scraper/runs`, then read the run’s default dataset. Make, Zapier and n8n can start runs and pass dataset rows to the next step; Google Sheets can receive rows through those workflows or a dataset export.

### What happens when a request fails?

Check the run log for the source or input error, correct the input and retry after a temporary source issue. A run can contain rows saved before a later request failed.

### Which inputs control Immowelt property results?

Set a location and property type, then use supported price, room or page options in Input. Results depend on active public listings in that area.

## Related Scrappa Actors

- [ImmobilienScout24 Location Autocomplete Scraper](https://apify.com/thescrappa/immobilienscout24-locations-scraper)
- [ImmobilienScout24 Price Insights Scraper](https://apify.com/thescrappa/immobilienscout24-price-insights-scraper)
- [ImmobilienScout24 Search Scraper](https://apify.com/thescrappa/immobilienscout24-search-scraper)
- [Redfin Property Details Scraper](https://apify.com/thescrappa/redfin-property-details-scraper)
- [Redfin Property Search Scraper](https://apify.com/thescrappa/redfin-property-search-scraper)
