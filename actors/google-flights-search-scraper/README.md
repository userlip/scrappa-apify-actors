# Google Flights Scraper

Compare Google Flights itineraries by fare, airlines, stops and total travel time. Compare routes using a future date and the one-way or round-trip option.

## What data can you extract?

Itineraries and fares reflect the selected route and dates; availability can change after a search.

| Field | Type | Description |
| --- | --- | --- |
| `position` | number | Result position in the Google Flights flight itinerary list, as a whole number; null when the source does not supply one. |
| `trip_type` | text | Trip type shown for the flight itinerary by Google Flights; null when Google Flights does not provide the value. |
| `price` | number | Displayed fare for this Google Flights itinerary, in the searched currency; null when no fare is shown. |
| `currency` | text | Currency code for this flight itinerary, formatted as Google Flights displays it, including the currency when shown; null when unavailable. |
| `total_duration_minutes` | number | Duration of this flight itinerary, in minutes; null when Google Flights provides no timing information. |
| `stops` | number | Flight stop count shown for the flight itinerary by Google Flights, in the format used by the source; null when it is omitted. |
| `airline_names` | array of text | Airline names serving this itinerary from Google Flights; an empty list when no entries are available. |
| `flight_numbers` | array of text | Flight numbers for the itinerary segments from Google Flights; an empty list when no entries are available. |
| `departure_airport` | text | Departure airport shown for the flight itinerary by Google Flights, in the format used by the source; null when it is omitted. |
| `arrival_airport` | text | Arrival airport shown for the flight itinerary by Google Flights, in the format used by the source; null when it is omitted. |
| `departure_time` | date | Departure time shown for the flight itinerary by Google Flights, in the format used by the source; null when it is omitted. |
| `arrival_time` | date | Arrival time shown for the flight itinerary by Google Flights, in the format used by the source; null when it is omitted. |
| `booking_token` | text | Booking token shown for the flight itinerary by Google Flights, in the format used by the source; null when it is omitted. |
| `legs` | array of objects | Flight segments with airports, airline, flight number and departure and arrival times from Google Flights; an empty list when no entries are available. |
| `outbound_legs` | array of objects | Outbound flight segments with airports, airline, flight number and departure and arrival times from Google Flights; an empty list when no entries are available. |
| `return_legs` | array of objects | Return flight segments with airports, airline, flight number and departure and arrival times from Google Flights; an empty list when no entries are available. |
| `request_origin` | text | Route origin passed to Google Flights. This input value is copied into the output row; null when it was not supplied. |
| `request_destination` | text | Route destination passed to Google Flights. This input value is copied into the output row; null when it was not supplied. |
| `request_departure_date` | date | Departure date passed to Google Flights; Use YYYY-MM-DD or a supported relative date. This input value is copied into the output row; null when it was not supplied. |
| `request_return_date` | date | Return date passed to Google Flights; Use YYYY-MM-DD or a supported relative date. This input value is copied into the output row; null when it was not supplied. |
| `request_cabin_class` | text | Flight cabin class passed to Google Flights. This input value is copied into the output row; null when it was not supplied. |
| `request_max_stops` | text | Maximum flight stop count passed to Google Flights; A whole-number stop count. This input value is copied into the output row; null when it was not supplied. |
| `request_sort_by` | text | Result sort order passed to Google Flights. This input value is copied into the output row; null when it was not supplied. |

## Use cases

- Travelers can compare fares or nightly rates before choosing a trip.
- Travel teams can check public options across routes, destinations and dates.
- Researchers can track prices and ratings in a travel market.

## How to use

1. Open the Actor’s **Input** tab and start with the JSON below.
2. Set the origin, destination and departure date; add a return date only for a round trip.
3. Start the run and open its default dataset to inspect or download the rows.

```json
{
  "origin": "JFK",
  "destination": "LAX",
  "departure_date": "45 days",
  "return_date": "52 days"
}
```

The run dataset can be downloaded as JSON, CSV, Excel or another format offered by Apify.

## Input

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

## Output example

The record below is synthetic. Names and links are examples, and private contact fields are omitted.

```json
{
  "trip_type": "one_way",
  "price": 438,
  "currency": "USD",
  "total_duration_minutes": 375,
  "stops": 0,
  "airline_names": [
    "Alaska Airlines"
  ],
  "legs": [
    {
      "departure_airport": "JFK",
      "arrival_airport": "LAX",
      "departure_time": "2026-11-15T08:10:00-05:00",
      "arrival_time": "2026-11-15T11:25:00-08:00",
      "airline": "Alaska Airlines",
      "flight_number": "AS 214",
      "duration_minutes": 375
    }
  ],
  "flight_numbers": [
    "AS 214"
  ]
}
```

## Pricing

**Current live price:** $0.20 per 1,000 results.

Each saved flight offer counts as one result.

## FAQ

### Is it legal to collect public information?

This Actor is for information visible on the public source pages it reads. You are responsible for checking the source terms, privacy and copyright rules, and the laws that apply to your use.

### How many records will a run return?

The result count depends on the input limits, pagination settings and what the source makes available. A small query or unavailable page can return fewer records, including none.

### Can I call it through the API or connect it to other tools?

Yes. Send the same JSON input to `POST https://api.apify.com/v2/acts/thescrappa~google-flights-search-scraper/runs`, then read the run’s default dataset. Make, Zapier and n8n can start runs and pass dataset rows to the next step; Google Sheets can receive rows through those workflows or a dataset export.

### What happens when a request fails?

Check the run log for the source or input error, correct the input and retry after a temporary source issue. A run can contain rows saved before a later request failed.

### How do I request a one-way Google Flights itinerary?

Set the trip type to one-way and provide a route and future departure date. Add a return date for a round trip.

## Related Scrappa Actors

- [Booking.com Hotel Details Scraper](https://apify.com/thescrappa/booking-hotel-details-scraper)
- [Booking.com Search Scraper](https://apify.com/thescrappa/booking-search-scraper)
- [Google Hotels Autocomplete Scraper](https://apify.com/thescrappa/google-hotels-autocomplete-scraper)
- [Google Hotels Search Scraper](https://apify.com/thescrappa/google-hotels-search-scraper)
- [Google Maps Directions Scraper](https://apify.com/thescrappa/google-maps-directions-scraper)
