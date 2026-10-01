# Kayak Flight Price Predictions Scraper

Retrieve KAYAK flight price prediction rows for airport routes with date ranges and minimum fares. Batch origin-destination pairs to compare route-level price patterns.

## What data can you extract?

| Field | Type | Description |
| --- | --- | --- |
| `startDate` | String | First date in the prediction period. |
| `endDate` | String | Last date in the prediction period. |
| `originAirport` | String | Airport code used as the route origin. |
| `destinationAirport` | String | Airport code used as the route destination. |
| `minPrice` | Integer | Minimum fare value reported for this date range. |
| `input_route` | Object | Origin and destination airport codes supplied for this route lookup. |
| `scraped_at` | String | UTC date and time when this record was collected. |

## Use cases

- Travel planners can compare predicted fare periods across routes.
- Fare analysts can review route-level minimum-price patterns.
- Travel agencies can prioritize routes for upcoming customer trips.

## How to use

1. Add origin and destination airports to each entry in `routes`.
2. Set `maxResults` to keep saved prediction rows within your desired size.
3. Compare each date range and minimum price across routes.

```json
{
  "routes": [
    {
      "origin": "ICT",
      "destination": "LAS"
    }
  ],
  "maxResults": 5
}
```

## Input

| Field | Type | Required | Description |
| --- | --- | --- | --- |
| `routes` | Array\<object\> | Yes | Origin and destination airport codes whose price ranges you want to compare. |
| `routes[].origin` | string | Yes per entry | Origin airport code for the route or destination search. |
| `routes[].destination` | string | Yes per entry | KAYAK destination code for the vacation-package search. |
| `destination` | string | No | KAYAK destination code for the vacation-package search. |
| `maxResults` | integer | No | Maximum dataset items to save across this entire run. |

## Output example

```json
{
  "startDate": "2026-10-15",
  "endDate": "2026-10-22",
  "originAirport": "ICT",
  "destinationAirport": "LAS",
  "minPrice": 189,
  "input_route": {
    "origin": "ICT",
    "destination": "LAS"
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

Yes. Send a POST request to `https://api.apify.com/v2/acts/thescrappa~kayak-flight-price-predictions-scraper/runs` with your Actor input, or use an Apify client library. Read the output from the run dataset.

### Can I connect the results to other tools?

Yes. Apify integrations and APIs can pass dataset results to Make, Zapier, n8n, Google Sheets, and other data workflows.

### What happens when one input fails?

The Actor logs a short source error and continues with the remaining entries. Transient rate limits and gateway errors are retried; if every entry fails, the run reports an error.

### Does a prediction guarantee the future ticket price?

No. KAYAK price predictions are estimates and can change as inventory and fares change.

## Related Scrappa Actors

- [Kayak Flights Scraper](https://apify.com/thescrappa/kayak-flights-scraper)
- [Google Flights Cheapest Dates Scraper](https://apify.com/thescrappa/google-flights-date-range-scraper)
- [Kayak Explore Cheapest Destinations Scraper](https://apify.com/thescrappa/kayak-explore-destinations-scraper)
