# Kayak Cruises Scraper

Browse Kayak cruise sailings with ship names, departure ports, trip lengths, itineraries, fares, and regions. Request ocean or river product types in one run.

## What data can you extract?

| Field | Type | Description |
| --- | --- | --- |
| `id` | String | KAYAK identifier for this cruise sailing. |
| `name` | String | Sailing or cruise itinerary name shown by KAYAK. |
| `currency` | String | Three-letter currency code for the quoted cruise fare. |
| `currencySymbol` | String | Currency symbol displayed with the fare. |
| `ship` | String | Ship assigned to the sailing. |
| `departurePort` | String | Port where the cruise itinerary begins. |
| `nights` | Integer | Number of nights in the sailing itinerary. |
| `price` | Integer | Numeric cruise fare in the reported currency. |
| `regularPrice` | Integer | Regular or reference fare when supplied by KAYAK. |
| `startDateFormatted` | String | Formatted departure date displayed for the sailing. |
| `endDateFormatted` | String | Formatted return date displayed for the sailing. |
| `regionName` | String | Cruise region label associated with the itinerary. |
| `regionId` | String | KAYAK identifier for the cruise region. |
| `itinerary` | Array\<Object\> | Day-by-day port calls included in the sailing. |
| `score` | Integer | Numeric rating or score reported by the source. |
| `productType` | String | Cruise category such as ocean or river. |
| `input_product_type` | String | Cruise product category requested for the sailing search. |
| `scraped_at` | String | UTC date and time when this record was collected. |

## Use cases

- Cruise planners can compare sailings by ship, port, length, and fare.
- Travel analysts can monitor cruise inventory across regions.
- Travel agencies can build sailing shortlists for customer itineraries.

## How to use

1. Add `ocean` or `river` to each entry in `sailing_types`.
2. Set passenger count, minimum nights, page, and sort preferences.
3. Use `maxResults` to limit saved sailings.

```json
{
  "sailing_types": [
    {
      "product_type": "ocean"
    }
  ],
  "min_nights": 3,
  "adults": 2,
  "page": 1,
  "sort": "recommended",
  "sort_order": "asc",
  "maxResults": 10
}
```

## Input

| Field | Type | Required | Description |
| --- | --- | --- | --- |
| `sailing_types` | Array\<object\> | Yes | KAYAK cruise product types to retrieve, such as ocean or river sailings. |
| `sailing_types[].product_type` | string | Yes per entry | Cruise product category, such as ocean or river. Accepted values: ocean, river. |
| `sailing_types[].min_nights` | integer | No | Minimum number of nights for returned cruise sailings. |
| `sailing_types[].adults` | integer | No | Number of adult travelers included in the search. |
| `sailing_types[].children` | integer | No | Number of children traveling on the cruise search. |
| `sailing_types[].page` | integer | No | One-based result page to request. |
| `sailing_types[].sort` | string | No | Sort order for returned search results. Accepted values: recommended, price, duration, departuredate. |
| `sailing_types[].sort_order` | string | No | Direction used for the selected result sort. Accepted values: asc, desc. |
| `sailing_types[].include_deals` | boolean | No | Whether to include offers marked as cruise deals. |
| `sailing_types[].include_unavailable` | boolean | No | Whether to include sailings marked unavailable. |
| `min_nights` | integer | No | Minimum number of nights for returned cruise sailings. |
| `adults` | integer | No | Number of adult travelers included in the search. |
| `children` | integer | No | Number of children traveling on the cruise search. |
| `page` | integer | No | One-based result page to request. |
| `sort` | string | No | Sort order for returned search results. Accepted values: recommended, price, duration, departuredate. |
| `sort_order` | string | No | Direction used for the selected result sort. Accepted values: asc, desc. |
| `include_deals` | boolean | No | Whether to include offers marked as cruise deals. |
| `include_unavailable` | boolean | No | Whether to include sailings marked unavailable. |
| `maxResults` | integer | No | Maximum dataset items to save across this entire run. |

## Output example

```json
{
  "id": "sailing-2027-0418-11",
  "name": "Pacific Coast Discovery",
  "currency": "USD",
  "currencySymbol": "$",
  "ship": "Aurora Skye",
  "departurePort": "Los Angeles",
  "nights": 7,
  "price": 1249,
  "regularPrice": 1499,
  "startDateFormatted": "Apr 18, 2027",
  "endDateFormatted": "Apr 25, 2027",
  "regionName": "Mexican Riviera",
  "regionId": "mexican-riviera",
  "itinerary": [
    {
      "day": 1,
      "port": "Los Angeles"
    },
    {
      "day": 3,
      "port": "Cabo San Lucas"
    }
  ],
  "score": 94,
  "productType": "ocean",
  "input_product_type": "ocean",
  "scraped_at": "2026-10-01T12:00:00Z"
}
```

## Pricing

$0.30 per 1,000 results. Apify saves one dataset item per result and applies the Actor’s per-result price to saved items.

Set **maxResults** to cap the number of dataset items saved in one run.

## FAQ

### Is scraping this information legal?

Rules depend on the source, location, data type, and intended use. Check applicable laws and source terms, and make sure your workflow follows privacy and data protection requirements.

### What limits apply?

Submit up to 100 batch entries per run. Use **maxResults** to cap saved items. Results also depend on source availability and your Scrappa API plan.

### Can I start runs through the Apify API?

Yes. Send a POST request to `https://api.apify.com/v2/acts/thescrappa~kayak-cruises-scraper/runs` with your Actor input, or use an Apify client library. Read the output from the run dataset.

### Can I connect the results to other tools?

Yes. Apify integrations and APIs can pass dataset results to Make, Zapier, n8n, Google Sheets, and other data workflows.

### What happens when one input fails?

The Actor logs a short source error and continues with the remaining entries. Transient rate limits and gateway errors are retried; if every entry fails, the run reports an error.

### Can I restrict sailings to a departure port?

The available search filters do not include a departure-port input. Returned rows include the port when KAYAK supplies it.

## Related Scrappa Actors

- [Kayak Flights Scraper](https://apify.com/thescrappa/kayak-flights-scraper)
- [Kayak Hotels Search Scraper](https://apify.com/thescrappa/kayak-hotels-search-scraper)
- [Kayak Vacation Packages Scraper](https://apify.com/thescrappa/kayak-packages-scraper)
