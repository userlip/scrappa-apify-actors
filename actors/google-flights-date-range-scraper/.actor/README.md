# Google Flights Cheapest Dates Scraper

Compare Google Flights’ cheapest starting option for each route and date range, with price, airline, stops, duration, and itinerary legs. Dates default to a future search window.

## What data can you extract?

| Field | Type | Description |
| --- | --- | --- |
| `price` | Integer | Quoted price in the currency shown by the source. |
| `currency` | String | Three-letter currency code for the quoted amount. |
| `total_duration_minutes` | Integer | Total Duration Minutes value associated with this google flights cheapest dates record. |
| `stops` | Integer | Stops value associated with this google flights cheapest dates record. |
| `airline_name` | String | Airline Name value associated with this google flights cheapest dates record. |
| `departure_date` | String | Departure Date value associated with this google flights cheapest dates record. |
| `return_date` | String | Return Date value associated with this google flights cheapest dates record. |
| `legs` | Array\<Object\> | Legs value associated with this google flights cheapest dates record. |
| `input_route` | Object | Origin and destination airport codes supplied for this route lookup. |
| `scraped_at` | String | UTC date and time when this record was collected. |

## Use cases

- Travel planners can compare lowest starting fares across airport pairs.
- Fare analysts can watch route prices before choosing departure windows.
- Travel agencies can collect first-look itinerary options for client routes.

## How to use

1. Add origin and destination airport codes to each entry in `routes`.
2. Set start and end dates or use the relative date defaults.
3. Choose passenger and cabin settings, then export the itinerary rows.

```json
{
  "routes": [
    {
      "origin": "JFK",
      "destination": "LAX"
    }
  ],
  "adults": 1,
  "cabin_class": "economy",
  "maxResults": 10
}
```

## Input

| Field | Type | Required | Description |
| --- | --- | --- | --- |
| `routes` | Array\<object\> | Yes | Origin and destination airport codes to compare for the same date range. |
| `routes[].origin` | string | Yes per entry | Origin airport code for the route or destination search. |
| `routes[].destination` | string | Yes per entry | KAYAK destination code for the vacation-package search. |
| `routes[].adults` | integer | No | Number of adult travelers included in the search. |
| `routes[].children` | integer | No | Number of children traveling on the cruise search. |
| `routes[].infants_in_seat` | integer | No | Number of infants in seat \(0-9, default: 0\) |
| `routes[].infants_on_lap` | integer | No | Number of infants on lap \(0-9, default: 0\) |
| `routes[].cabin_class` | string | No | Cabin class: economy, premium\_economy, business, or first \(default: economy\) Accepted values: economy, premium\_economy, business, first. |
| `routes[].exclude_basic` | boolean | No | Exclude basic economy fares when searching in economy class \(default: false\) |
| `routes[].max_stops` | string | No | Maximum stops: any, nonstop, one\_or\_fewer, or two\_or\_fewer \(default: any\) Accepted values: any, nonstop, one\_or\_fewer, two\_or\_fewer. |
| `routes[].hl` | string | No | Language code \(e.g., en, de, en-US\) |
| `routes[].gl` | string | No | Country/region code \(e.g., us, de, gb\) |
| `routes[].currency` | string | No | Currency code for prices \(e.g., USD, EUR, GBP\) |
| `routes[].outbound_times` | string | No | Outbound time window as comma-separated hours, e.g. "6,18" for departures between 06:00 and 18:00. Two further values narrow the arrival window. |
| `routes[].bags` | integer | No | Carry-on bag count. Only 0 is currently supported. Accepted values: 0. Allowed values: 0. |
| `routes[].api_version` | integer | No | Value accepted by the Google Flights Cheapest Dates Scraper search. Accepted values: 2. Allowed values: 2. |
| `routes[].from_date` | string | No | Start of date range in Y-m-d format When omitted, the start date is set to 30 days from today. |
| `routes[].to_date` | string | No | End of date range in Y-m-d format When omitted, the end date is set to seven days after the start date. |
| `routes[].trip_duration` | integer | No | Duration of trip in days \(1-30, default: 7\) |
| `destination` | string | No | KAYAK destination code for the vacation-package search. |
| `adults` | integer | No | Number of adult travelers included in the search. |
| `children` | integer | No | Number of children traveling on the cruise search. |
| `infants_in_seat` | integer | No | Number of infants in seat \(0-9, default: 0\) |
| `infants_on_lap` | integer | No | Number of infants on lap \(0-9, default: 0\) |
| `cabin_class` | string | No | Cabin class: economy, premium\_economy, business, or first \(default: economy\) Accepted values: economy, premium\_economy, business, first. |
| `exclude_basic` | boolean | No | Exclude basic economy fares when searching in economy class \(default: false\) |
| `max_stops` | string | No | Maximum stops: any, nonstop, one\_or\_fewer, or two\_or\_fewer \(default: any\) Accepted values: any, nonstop, one\_or\_fewer, two\_or\_fewer. |
| `hl` | string | No | Language code \(e.g., en, de, en-US\) |
| `gl` | string | No | Country/region code \(e.g., us, de, gb\) |
| `currency` | string | No | Currency code for prices \(e.g., USD, EUR, GBP\) |
| `outbound_times` | string | No | Outbound time window as comma-separated hours, e.g. "6,18" for departures between 06:00 and 18:00. Two further values narrow the arrival window. |
| `bags` | integer | No | Carry-on bag count. Only 0 is currently supported. Accepted values: 0. Allowed values: 0. |
| `api_version` | integer | No | Value accepted by the Google Flights Cheapest Dates Scraper search. Accepted values: 2. Allowed values: 2. |
| `from_date` | string | No | Start of date range in Y-m-d format When omitted, the start date is set to 30 days from today. |
| `to_date` | string | No | End of date range in Y-m-d format When omitted, the end date is set to seven days after the start date. |
| `trip_duration` | integer | No | Duration of trip in days \(1-30, default: 7\) |
| `maxResults` | integer | No | Maximum dataset items to save across this entire run. |

## Output example

```json
{
  "price": 245,
  "currency": "USD",
  "total_duration_minutes": 356,
  "stops": 0,
  "airline_name": "JetBlue",
  "departure_date": "2026-10-31",
  "return_date": "2026-11-07",
  "legs": [
    {
      "departure_airport": "JFK",
      "arrival_airport": "LAX",
      "departure_time": "2026-10-31T08:15:00-04:00",
      "arrival_time": "2026-10-31T11:11:00-07:00",
      "airline_name": "JetBlue",
      "flight_number": "B6123"
    }
  ],
  "input_route": {
    "origin": "JFK",
    "destination": "LAX"
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

Yes. Send a POST request to `https://api.apify.com/v2/acts/thescrappa~google-flights-date-range-scraper/runs` with your Actor input, or use an Apify client library. Read the output from the run dataset.

### Can I connect the results to other tools?

Yes. Apify integrations and APIs can pass dataset results to Make, Zapier, n8n, Google Sheets, and other data workflows.

### What happens when one input fails?

The Actor logs a short source error and continues with the remaining entries. Transient rate limits and gateway errors are retried; if every entry fails, the run reports an error.

### Does the Actor return every date in the selected range?

This version returns the cheapest starting option for the start date. Request the next departure date separately to continue a date-range search.

## Related Scrappa Actors

- [Google Flights Scraper](https://apify.com/thescrappa/google-flights-search-scraper)
- [Kayak Flights Scraper](https://apify.com/thescrappa/kayak-flights-scraper)
- [Kayak Flight Price Predictions Scraper](https://apify.com/thescrappa/kayak-flight-price-predictions-scraper)
