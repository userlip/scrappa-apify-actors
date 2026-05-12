# Google Flights Search Scraper

Search Google Flights one-way and round-trip fares for travel pricing, route monitoring, flight comparison, and airfare research workflows. The actor wraps Scrappa's `/api/flights/one-way` and `/api/flights/round-trip` endpoints and writes one Apify dataset item per returned flight result.

## What you get

- One-way and round-trip Google Flights search results
- Price, currency, total duration, stop count, airlines, flight numbers, airport codes, departure and arrival times
- Full leg details for every itinerary
- Booking tokens when returned by Google Flights
- Search metadata and the full Scrappa response in key-value store record `OUTPUT`
- Optional baggage enrichment for the cheapest returned flight

## Input

```json
{
  "trip_type": "round_trip",
  "origin": "JFK",
  "destination": "LAX",
  "departure_date": "2026-09-15",
  "return_date": "2026-09-22",
  "adults": 1,
  "cabin_class": "economy",
  "max_stops": "one_or_fewer",
  "sort_by": "cheapest",
  "currency": "USD",
  "hl": "en",
  "gl": "us"
}
```

For one-way searches, set `trip_type` to `one_way` and omit `return_date`.

## Output

Each dataset item is one flight option:

```json
{
  "position": 1,
  "trip_type": "round_trip",
  "price": 326,
  "currency": "USD",
  "total_duration_minutes": 374,
  "stops": 0,
  "airline_names": ["Delta"],
  "flight_numbers": ["DL123"],
  "departure_airport": "JFK",
  "arrival_airport": "LAX",
  "departure_time": "2026-09-15T08:00:00",
  "arrival_time": "2026-09-15T11:14:00",
  "booking_token": "Cg0I...",
  "legs": [],
  "request_origin": "JFK",
  "request_destination": "LAX",
  "request_departure_date": "2026-09-15",
  "request_return_date": "2026-09-22"
}
```

## Pricing

This actor is intended for paid pay-per-event monetization with the `flight-result` event, so users pay only for saved flight result rows. If Apify requires a different marketplace model during publication, use the closest paid usage-aligned model and verify pricing through the Apify API before making the actor public.

For higher-volume Google Flights data extraction or direct API access, use Scrappa's Google Flights API at `https://scrappa.co/api/flights/one-way` and `https://scrappa.co/api/flights/round-trip`.
