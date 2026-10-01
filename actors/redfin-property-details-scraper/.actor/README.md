# Redfin Property Details Scraper

Review Redfin home details with address, estimated value, bedrooms and bathrooms. Look up one property by ID or URL, or submit a batch to review several homes together.

## What data can you extract?

Listing prices and estimates follow the current Redfin page and can change over time.

| Field | Type | Description |
| --- | --- | --- |
| `property_id` | number | property ID for the property listing, assigned by Redfin; null when the source does not expose it. |
| `address` | text | Address shown for the property listing by Redfin, in the format used by the source; null when it is omitted. |
| `city` | text | City shown for the property listing by Redfin; null when Redfin does not provide the value. |
| `state` | text | State or region shown for the property listing by Redfin; null when Redfin does not provide the value. |
| `zip` | text | Zip shown for the property listing by Redfin, in the format used by the source; null when it is omitted. |
| `country` | text | Country shown for the property listing by Redfin; null when Redfin does not provide the value. |
| `price` | number | Listed price for this property listing, as a numeric amount in the listing currency; null when Redfin provides no price. |
| `price_label` | text | Price label for this property listing, formatted as Redfin displays it, including the currency when shown; null when unavailable. |
| `beds` | number | Number of bedrooms shown by Redfin, as a whole number; zero is possible, and null means no count was reported. |
| `baths` | number | Number of bathrooms shown by Redfin, as a whole number; zero is possible, and null means no count was reported. |
| `sqft` | number | Interior area for this property listing, in area in square feet; null when Redfin does not supply the value. |
| `lot_size` | number | Lot area for this property listing, in area in square feet; null when Redfin does not supply the value. |
| `year_built` | number | Year built shown for the property listing by Redfin, in the format used by the source; null when it is omitted. |
| `property_type` | number | Property type shown for the property listing by Redfin, in the format used by the source; null when it is omitted. |
| `status` | number | Status reported for the property listing by Redfin; null when Redfin does not provide the value. |
| `status_label` | text | Status reported for the property listing by Redfin; null when Redfin does not provide the value. |
| `latitude` | number | Latitude for this property listing on Redfin, in decimal degrees; null when the source provides no coordinates. |
| `longitude` | number | Longitude for this property listing on Redfin, in decimal degrees; null when the source provides no coordinates. |
| `url` | link | Source page url for this property listing on Redfin; null when the source does not provide a URL. |
| `description` | text | Description text from Redfin for this property listing; null when the source has no text to show. |
| `photos` | array of objects | Place or property photos with image URL and caption where available from Redfin; an empty list when no entries are available. |
| `request_property_index` | number | Property result index passed to Redfin. This input value is copied into the output row; null when it was not supplied. |
| `request_property_id` | number | Property id passed to Redfin. This input value is copied into the output row; null when it was not supplied. |
| `request_input` | text | Submitted lookup value passed to Redfin. This input value is copied into the output row; null when it was not supplied. |
| `request_source` | text | Source selection passed to Redfin. This input value is copied into the output row; null when it was not supplied. |
| `success` | boolean | Whether the lookup completed successfully; false is a reported value, while null means Redfin provided no flag. |
| `error` | text | Diagnostic text for the Redfin lookup; null when the request completes without an error. |
| `status_code` | number | Http status code shown for the property listing by Redfin, in the format used by the source; null when it is omitted. |

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
  "url": "https://www.redfin.com/TN/Memphis/1549-Ely-St-38106/home/60791456",
  "property_ids": [
    60791456
  ]
}
```

The run dataset can be downloaded as JSON, CSV, Excel or another format offered by Apify.

## Input

| Field | Type | Required | Description |
| --- | --- | --- | --- |
| `property_id` | integer | No | Single Redfin property ID. Example: 60791456 from a Redfin URL ending in /home/60791456. Constraints: minimum 1. |
| `property_ids` | array of integer | No | Batch of Redfin property IDs to process in one Apify run. Constraints: maximum 100 items. |
| `url` | string | No | Single Redfin property URL containing /home/{property_id}. |
| `urls` | array of string | No | Batch of Redfin property URLs containing /home/{property_id}. Constraints: maximum 100 items. |

## Output example

The record below is synthetic. Names and links are examples, and private contact fields are omitted.

```json
{
  "photos": [
    {
      "url": "https://images.example.com/homes/green-lake-front.jpg",
      "caption": "Front exterior"
    }
  ],
  "description": "Renovated craftsman home with a covered porch, updated kitchen and fenced backyard.",
  "price": 824000,
  "url": "https://listings.example.com/record/731-alder-way",
  "property_id": 31245678,
  "address": "731 Alder Way, Seattle, WA 98103",
  "city": "Seattle",
  "state": "WA"
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

Yes. Send the same JSON input to `POST https://api.apify.com/v2/acts/thescrappa~redfin-property-details-scraper/runs`, then read the run’s default dataset. Make, Zapier and n8n can start runs and pass dataset rows to the next step; Google Sheets can receive rows through those workflows or a dataset export.

### What happens when a request fails?

Check the run log for the source or input error, correct the input and retry after a temporary source issue. A run can contain rows saved before a later request failed.

### Can Redfin Property Details accept a property URL?

Yes. Use `url` for one property or `urls` for a batch; `property_id` and `property_ids` are also supported. The record depends on the listing still being available.

## Related Scrappa Actors

- [ImmobilienScout24 Location Autocomplete Scraper](https://apify.com/thescrappa/immobilienscout24-locations-scraper)
- [ImmobilienScout24 Price Insights Scraper](https://apify.com/thescrappa/immobilienscout24-price-insights-scraper)
- [ImmobilienScout24 Search Scraper](https://apify.com/thescrappa/immobilienscout24-search-scraper)
- [Immowelt Property Search Scraper](https://apify.com/thescrappa/immowelt-property-search-scraper)
- [Redfin Property Search Scraper](https://apify.com/thescrappa/redfin-property-search-scraper)
