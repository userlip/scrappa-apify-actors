# Kayak Car Rentals Scraper

Search Kayak car rentals by pickup location and travel dates. Collect provider names, vehicle descriptions, quoted prices, and booking links for comparison.

## What data can you extract?

| Field | Type | Description |
| --- | --- | --- |
| `resultId` | String | KAYAK identifier for this car-rental offer. |
| `providerCode` | String | Code identifying the rental provider. |
| `providerName` | String | Rental provider name displayed in the offer. |
| `companyName` | String | Car-hire company associated with the vehicle offer. |
| `title` | String | Vehicle or rental offer title shown by KAYAK. |
| `description` | String | Short vehicle or rental-condition description. |
| `price` | String | Localized price label for the offer. |
| `priceData` | Object | Numeric amount, currency, and period used for the quoted price. |
| `clickUrlTemplate` | String | KAYAK click-through URL template for the rental offer. |
| `isAd` | Boolean | True when KAYAK marks the result as an advertisement. |
| `rank` | Integer | Position of the offer in the returned search results. |
| `input_pickup` | String | Pickup location code submitted for the rental-car search. |
| `scraped_at` | String | UTC date and time when this record was collected. |

## Use cases

- Travel planners can compare rental-car options for airport pickups.
- Car-hire analysts can monitor quoted totals and providers across locations.
- Travel agencies can shortlist vehicles for future itineraries.

## How to use

1. Add an airport or city pickup code to each entry in `pickup_locations`.
2. Set pickup and drop-off dates or use the relative date defaults.
3. Choose pickup times, sorting, and total or daily price display.

```json
{
  "pickup_locations": [
    {
      "pickup": "LAS"
    }
  ],
  "pickup_hour": 10,
  "dropoff_hour": 10,
  "sort": "rank_a",
  "price_mode": "total",
  "maxResults": 10
}
```

## Input

| Field | Type | Required | Description |
| --- | --- | --- | --- |
| `pickup_locations` | Array\<object\> | Yes | Airport or city pickup codes for KAYAK rental-car searches. |
| `pickup_locations[].pickup` | string | Yes per entry | Airport or city code where the rental car is collected. |
| `pickup_locations[].pickup_date` | string | No | Rental car pickup date in YYYY-MM-DD format. When omitted, pickup is set to 30 days from today. |
| `pickup_locations[].dropoff_date` | string | No | Rental car drop-off date in YYYY-MM-DD format after pickup. When omitted, drop-off is set to four days after pickup. |
| `pickup_locations[].pickup_hour` | integer | No | Local hour when the rental car is collected. |
| `pickup_locations[].dropoff_hour` | integer | No | Local hour when the rental car is returned. |
| `pickup_locations[].sort` | string | No | Sort order for returned search results. Accepted values: rank\_a, price\_a. |
| `pickup_locations[].price_mode` | string | No | Whether quoted prices are shown as totals or per-night or per-person values. Accepted values: total, daily. |
| `pickup_date` | string | No | Rental car pickup date in YYYY-MM-DD format. When omitted, pickup is set to 30 days from today. |
| `dropoff_date` | string | No | Rental car drop-off date in YYYY-MM-DD format after pickup. When omitted, drop-off is set to four days after pickup. |
| `pickup_hour` | integer | No | Local hour when the rental car is collected. |
| `dropoff_hour` | integer | No | Local hour when the rental car is returned. |
| `sort` | string | No | Sort order for returned search results. Accepted values: rank\_a, price\_a. |
| `price_mode` | string | No | Whether quoted prices are shown as totals or per-night or per-person values. Accepted values: total, daily. |
| `maxResults` | integer | No | Maximum dataset items to save across this entire run. |

## Output example

```json
{
  "resultId": "car-280551",
  "providerCode": "hertz",
  "providerName": "Hertz",
  "companyName": "Hertz",
  "title": "Compact SUV",
  "description": "Compact SUV, unlimited mileage",
  "price": "$248",
  "priceData": {
    "amount": 248,
    "currency": "USD",
    "period": "total"
  },
  "clickUrlTemplate": "https://www.kayak.com/cars/LAS/2026-10-31/2026-11-04?provider=hertz",
  "isAd": false,
  "rank": 1,
  "input_pickup": "LAS",
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

Yes. Send a POST request to `https://api.apify.com/v2/acts/thescrappa~kayak-car-rentals-scraper/runs` with your Actor input, or use an Apify client library. Read the output from the run dataset.

### Can I connect the results to other tools?

Yes. Apify integrations and APIs can pass dataset results to Make, Zapier, n8n, Google Sheets, and other data workflows.

### What happens when one input fails?

The Actor logs a short source error and continues with the remaining entries. Transient rate limits and gateway errors are retried; if every entry fails, the run reports an error.

### Can pickup and drop-off times be specified?

Yes. Use `pickup_hour` and `dropoff_hour` with hour values supported by KAYAK.

## Related Scrappa Actors

- [Kayak Flights Scraper](https://apify.com/thescrappa/kayak-flights-scraper)
- [Kayak Hotels Search Scraper](https://apify.com/thescrappa/kayak-hotels-search-scraper)
- [Kayak Vacation Rentals Scraper](https://apify.com/thescrappa/kayak-vacation-rentals-scraper)
