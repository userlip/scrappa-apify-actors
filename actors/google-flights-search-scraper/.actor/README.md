# Google Flights Search Scraper for Travel Planning

The Google Flights Search Scraper for Travel Planning collects flight options, prices, and schedules from Google Flights. Provide the fields listed below; the actor saves source fields such as `position`, `trip_type`, `price`, and `currency` to an Apify dataset.

## What data can you extract?

The dataset contains fields returned by Google Flights. The field names below match the Actor output schema. A source may leave optional values empty or omit fields when they are not available for a result.

| Field | Type | Description |
| --- | --- | --- |
| `position` | number | Position returned for this result. |
| `trip_type` | text | Trip Type returned for this result. |
| `price` | number | Price returned for this result. |
| `currency` | text | Currency returned for this result. |
| `total_duration_minutes` | number | Duration returned for this result. |
| `stops` | number | Stops returned for this result. |
| `airline_names` | object | Airlines returned for this result. |
| `flight_numbers` | object | Flight Numbers returned for this result. |
| `departure_airport` | text | From returned for this result. |
| `arrival_airport` | text | To returned for this result. |
| `departure_time` | date | Departure returned for this result. |
| `arrival_time` | date | Arrival returned for this result. |
| `booking_token` | text | Booking Token returned for this result. |
| `legs` | object | Legs returned for this result. |
| `outbound_legs` | object | Outbound Legs returned for this result. |
| `return_legs` | object | Return Legs returned for this result. |
| `request_origin` | text | Requested Origin returned for this result. |
| `request_destination` | text | Requested Destination returned for this result. |
| `request_departure_date` | date | Departure Date returned for this result. |
| `request_return_date` | date | Return Date returned for this result. |
| `request_cabin_class` | text | Cabin returned for this result. |
| `request_max_stops` | text | Max Stops returned for this result. |
| `request_sort_by` | text | Sort returned for this result. |

## Use cases

- Collect flight options, prices, and schedules for a destination, route, or travel date.
- Compare returned options and details before planning a trip.
- Export travel records to a spreadsheet or booking research workflow.

## How to use

1. Open the **Input** tab and use the example JSON below.
2. Change the query, URL, identifier, or other fields you need. Set the lookup fields to match the query or identifier you want to collect.
3. Start the Actor. Open the run's default dataset to inspect, download, or export the returned records.

```json
{
  "origin": "JFK",
  "destination": "LAX",
  "departure_date": "45 days",
  "return_date": "52 days"
}
```

Apify stores the run output in a dataset. You can download the dataset in JSON, CSV, Excel, XML, or other available formats from the run page.

## Output example

This illustrative record uses synthetic values. It shows the real output field names; optional source values may be null or absent.

```json
{
  "position": 42,
  "trip_type": "Example value",
  "price": 129.99,
  "currency": "USD",
  "total_duration_minutes": 42,
  "stops": 42,
  "airline_names": {}
}
```

## Input fields

| Field | Type | Required | Description |
| --- | --- | --- | --- |
| `trip_type` | string | No | Search one-way or round-trip Google Flights results. Constraints: allowed values: one_way, round_trip. |
| `origin` | string | Yes | Three-letter IATA origin airport code. |
| `destination` | string | Yes | Three-letter IATA destination airport code. |
| `departure_date` | string | Yes | Departure date in YYYY-MM-DD format or relative to the run date, such as 45 days. |
| `return_date` | string | No | Return date in YYYY-MM-DD format or relative to the run date. Required when Trip Type is Round-trip. |
| `adults` | integer | No | Number of adult passengers. Constraints: minimum 1; maximum 9. |
| `children` | integer | No | Number of child passengers. Constraints: minimum 0; maximum 9. |
| `infants_in_seat` | integer | No | Number of infant passengers with their own seat. Constraints: minimum 0; maximum 9. |
| `infants_on_lap` | integer | No | Number of infant passengers seated on lap. Constraints: minimum 0; maximum 9. |
| `cabin_class` | string | No | Cabin class filter. Constraints: allowed values: economy, premium_economy, business, first. |
| `exclude_basic` | boolean | No | Exclude basic economy fares where Google supports that filter. |
| `max_stops` | string | No | Maximum stops filter. Constraints: allowed values: any, nonstop, one_or_fewer, two_or_fewer. |
| `sort_by` | string | No | Google Flights sort order. Constraints: allowed values: top_flights, cheapest, departure_time, arrival_time, duration. |
| `airlines` | string | No | Optional comma-separated 2-character IATA airline codes, such as AA,DL,UA. |
| `include_baggage` | boolean | No | Fetch baggage details for the cheapest returned flight. |
| `currency` | string | No | Three-letter currency code for prices. |
| `hl` | string | No | Google Flights language code, such as en, de, es, or en-us. |
| `gl` | string | No | Two-letter country code, such as us, gb, de, or ca. |
| `departure_time_min` | integer | No | Earliest departure hour, from 0 to 23. Constraints: minimum 0; maximum 23. |
| `departure_time_max` | integer | No | Latest departure hour, from 0 to 23. Constraints: minimum 0; maximum 23. |
| `arrival_time_min` | integer | No | Earliest arrival hour, from 0 to 23. Constraints: minimum 0; maximum 23. |
| `arrival_time_max` | integer | No | Latest arrival hour, from 0 to 23. Constraints: minimum 0; maximum 23. |
| `max_duration_minutes` | integer | No | Maximum total flight duration in minutes. Constraints: minimum 1. |
| `max_price` | integer | No | Maximum price in the selected currency. Constraints: minimum 1. |

## Pricing

**Current live price:** $0.20 per 1,000 results.

The price is based on the latest live Apify pricing entry. Per-result charges depend on the number of billed results returned. If the price line lists subscription tiers, the rate shown for each tier applies to that Apify subscription level.

## FAQ

### Is it legal to scrape this data?

This Actor is intended for data that is publicly available from Google Flights. You are responsible for following the source site's terms, privacy and copyright rules, and the laws that apply to your use of the data. Only collect information you have a lawful basis to use.

### What limits should I expect?

Use the field constraints and limits in the input table. The source controls which records are available, so a narrow query or unavailable page can return fewer results or none. Keep batch lists small when you need a quick first run.

### Can I run it through the API or connect it to other tools?

Yes. Send the same JSON input to `POST https://api.apify.com/v2/actors/thescrappa~google-flights-search-scraper/runs`. The response includes `defaultDatasetId`; use it to fetch the run's dataset items. See the [Apify Run Actor API](https://docs.apify.com/api/v2/actors-runs-post) and [Actor runs guide](https://docs.apify.com/api/v2/actors-actor-runs). Apify integrations for [Make](https://docs.apify.com/integrations/make), [Zapier](https://docs.apify.com/integrations/zapier), and [n8n](https://docs.apify.com/integrations/n8n) can trigger runs and pass results to the next workflow step. Send dataset rows to Google Sheets through a Sheets step or a workflow integration.

### What happens if a request fails?

Apify reports input validation and source request errors in the run details. Review the error, correct the input, and retry after a temporary source problem. Depending on when a request stops, the dataset may be empty or contain results collected before the failure.

## Related Scrappa Actors

- [Booking.com Hotel Details Scraper for Travel](https://apify.com/thescrappa/booking-hotel-details-scraper)
- [Booking.com Search Scraper for Travel Planning](https://apify.com/thescrappa/booking-search-scraper)
- [Google Hotels Autocomplete Scraper for Stays](https://apify.com/thescrappa/google-hotels-autocomplete-scraper)
- [Google Hotels Search Scraper for Travel Planning](https://apify.com/thescrappa/google-hotels-search-scraper)
- [Google Maps Directions Scraper for Travel Planning](https://apify.com/thescrappa/google-maps-directions-scraper)
