# Kayak Vacation Packages Scraper

Search Kayak for flight-and-hotel packages with trip summaries, flight legs, lodging details, providers, prices, and booking links. Batch route pairs for future dates.

## What data can you extract?

| Field | Type | Description |
| --- | --- | --- |
| `resultId` | String | KAYAK identifier for the flight-and-hotel package result. |
| `localizedHeadline` | String | Localized package heading shown in the search results. |
| `localizedDescription` | String | Short summary of the bundled flight and lodging offer. |
| `price` | String | Localized package price label in the requested price mode. |
| `totalPrice` | String | Localized total price for the package offer. |
| `providerName` | String | Travel provider shown for the package booking. |
| `providerCode` | String | Code identifying the travel provider. |
| `clickUrl` | String | KAYAK or provider click-through URL for the package. |
| `flightLegs` | Array\<Object\> | Flight segments included in the package offer. |
| `hotelData` | Object | Hotel identity and stay length included in the bundle. |
| `isAd` | Boolean | True when KAYAK marks the package as an advertisement. |
| `input_package_search` | Object | Origin airport and destination supplied for this package search. |
| `scraped_at` | String | UTC date and time when this record was collected. |

## Use cases

- Travel planners can compare bundled flight and hotel offers.
- Travel agencies can shortlist packages for several airport pairs.
- Fare analysts can track bundle prices alongside standalone trips.

## How to use

1. Add a destination and origin airport to each entry in `package_searches`.
2. Set departure and return dates or use the relative date defaults.
3. Choose passenger count, duration, price mode, and sorting options.

```json
{
  "package_searches": [
    {
      "destination": "Z35107",
      "origin": "ICT"
    }
  ],
  "adults": 2,
  "duration": "4",
  "price_mode": "perperson",
  "sort": "rank_a",
  "maxResults": 10
}
```

## Input

| Field | Type | Required | Description |
| --- | --- | --- | --- |
| `package_searches` | Array\<object\> | Yes | Package destinations and origin airports to search together. |
| `package_searches[].destination` | string | Yes per entry | KAYAK destination code for the vacation-package search. |
| `package_searches[].origin` | string | Yes per entry | Origin airport code for the route or destination search. |
| `package_searches[].departure_date` | string | No | Outbound travel date in YYYY-MM-DD format. When omitted, departure is set to 30 days from today. |
| `package_searches[].return_date` | string | No | Return travel date in YYYY-MM-DD format after departure. When omitted, return is set to four days after departure. |
| `package_searches[].adults` | integer | No | Number of adult travelers included in the search. |
| `package_searches[].child_ages` | Array\<integer\> | No | Ages of children included in the stay, when applicable. |
| `package_searches[].duration` | string | No | Requested trip duration in days. |
| `package_searches[].price_mode` | string | No | Whether quoted prices are shown as totals or per-night or per-person values. Accepted values: perperson, total. |
| `package_searches[].sort` | string | No | Sort order for returned search results. Accepted values: rank\_a, price\_a. |
| `origin` | string | No | Origin airport code for the route or destination search. |
| `departure_date` | string | No | Outbound travel date in YYYY-MM-DD format. When omitted, departure is set to 30 days from today. |
| `return_date` | string | No | Return travel date in YYYY-MM-DD format after departure. When omitted, return is set to four days after departure. |
| `adults` | integer | No | Number of adult travelers included in the search. |
| `child_ages` | Array\<integer\> | No | Ages of children included in the stay, when applicable. |
| `duration` | string | No | Requested trip duration in days. |
| `price_mode` | string | No | Whether quoted prices are shown as totals or per-night or per-person values. Accepted values: perperson, total. |
| `sort` | string | No | Sort order for returned search results. Accepted values: rank\_a, price\_a. |
| `maxResults` | integer | No | Maximum dataset items to save across this entire run. |

## Output example

```json
{
  "resultId": "package-90117",
  "localizedHeadline": "Four nights in Las Vegas",
  "localizedDescription": "Flight and hotel bundle near the Strip",
  "price": "$389",
  "totalPrice": "$778",
  "providerName": "Expedia",
  "providerCode": "expedia",
  "clickUrl": "https://www.kayak.com/packages/ICT-LAS/2026-10-31/2026-11-04",
  "flightLegs": [
    {
      "originAirport": "ICT",
      "destinationAirport": "LAS",
      "airline": "Southwest Airlines"
    }
  ],
  "hotelData": {
    "name": "Desert Palm Resort",
    "nights": 4
  },
  "isAd": false,
  "input_package_search": {
    "destination": "Z35107",
    "origin": "ICT"
  },
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

Yes. Send a POST request to `https://api.apify.com/v2/acts/thescrappa~kayak-packages-scraper/runs` with your Actor input, or use an Apify client library. Read the output from the run dataset.

### Can I connect the results to other tools?

Yes. Apify integrations and APIs can pass dataset results to Make, Zapier, n8n, Google Sheets, and other data workflows.

### What happens when one input fails?

The Actor logs a short source error and continues with the remaining entries. Transient rate limits and gateway errors are retried; if every entry fails, the run reports an error.

### Is the displayed package price per person or for the full group?

That depends on `price_mode`. Choose `perperson` or `total` to request the corresponding KAYAK price display.

## Related Scrappa Actors

- [Kayak Flights Scraper](https://apify.com/thescrappa/kayak-flights-scraper)
- [Kayak Hotels Search Scraper](https://apify.com/thescrappa/kayak-hotels-search-scraper)
- [Kayak Vacation Rentals Scraper](https://apify.com/thescrappa/kayak-vacation-rentals-scraper)
