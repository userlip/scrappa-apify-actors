# Kayak Flights Scraper

Search Kayak for airport routes and collect itinerary IDs, flight legs, timing, provider counts, and shareable result links. Choose one-way or round-trip searches.

## What data can you extract?

| Field | Type | Description |
| --- | --- | --- |
| `resultId` | String | Identifier for the flight itinerary in this Kayak result set. |
| `type` | String | Kayak trip type for the itinerary, such as one-way or round trip. |
| `shareableId` | String | Kayak identifier used to reopen the saved itinerary search. |
| `shareableUrl` | String | URL that opens this itinerary or its search results on Kayak. |
| `legs` | Array\<object\> | Flight segments with airports, departure and arrival times, and duration. |
| `totalProviders` | Integer | Number of booking providers offering this itinerary. |
| `totalBookingOptions` | Integer | Number of distinct booking choices reported for the itinerary. |
| `input_route` | Object | Origin, destination, and any dates submitted for this route search. |
| `scraped_at` | String | UTC date and time when this itinerary was collected. |

## Use cases

- Travel planners can compare itineraries across multiple airport routes.
- Travel agencies can check providers, flight times, and booking choices.
- Fare analysts can track options for one-way and round-trip searches.

## How to use

1. Add one route object with origin and destination to `flights` per search.
2. Choose one-way or round-trip; omitted dates are set relative to today.
3. Set passenger and cabin options, then cap pages and results.

```json
{
  "flights": [
    {
      "origin": "JFK",
      "destination": "LAX"
    }
  ],
  "tripType": "one-way",
  "maxResults": 20,
  "maxPages": 2
}
```

## Input

| Field | Type | Required | Description |
| --- | --- | --- | --- |
| `flights` | Array\<object\> | Yes | Flight routes with an origin and destination for each search. |
| `flights[].origin` | string | Yes per entry | 3-letter IATA airport or metro code. |
| `flights[].destination` | string | Yes per entry | 3-letter IATA airport or metro code. |
| `flights[].departure_date` | string | No | Departure date in YYYY-MM-DD format. When omitted, departure is set to 30 days from today. |
| `flights[].adults` | integer | No | Adult passengers, 1-9. |
| `flights[].children` | integer | No | Child passengers, 0-8. |
| `flights[].infants_in_seat` | integer | No | Infants with a seat, 0-8. |
| `flights[].infants_on_lap` | integer | No | Lap infants, 0-8. |
| `flights[].cabin` | string | No | economy, premium\_economy, business, or first. |
| `flights[].sort` | string | No | best, price, or duration. |
| `flights[].page` | integer | No | Results page, 1-20. |
| `flights[].currency` | string | No | 3-letter ISO currency code. |
| `flights[].locale` | string | No | Language locale such as en-US. |
| `flights[].include_price_prediction` | boolean | No | Include available price insight data. |
| `flights[].include_display_metadata` | boolean | No | Include available display and offer metadata. |
| `flights[].return_date` | string | Conditional | Return date in YYYY-MM-DD format. Required when tripType is round-trip. Required for Round trip searches. When omitted for a round trip, return is set to 4 days after departure. |
| `destination` | string | No | 3-letter IATA airport or metro code. |
| `departure_date` | string | No | Departure date in YYYY-MM-DD format. When omitted, departure is set to 30 days from today. |
| `adults` | integer | No | Adult passengers, 1-9. |
| `children` | integer | No | Child passengers, 0-8. |
| `infants_in_seat` | integer | No | Infants with a seat, 0-8. |
| `infants_on_lap` | integer | No | Lap infants, 0-8. |
| `cabin` | string | No | economy, premium\_economy, business, or first. |
| `sort` | string | No | best, price, or duration. |
| `page` | integer | No | Results page, 1-20. |
| `currency` | string | No | 3-letter ISO currency code. |
| `locale` | string | No | Language locale such as en-US. |
| `include_price_prediction` | boolean | No | Include available price insight data. |
| `include_display_metadata` | boolean | No | Include available display and offer metadata. |
| `return_date` | string | Conditional | Return date in YYYY-MM-DD format. Required when tripType is round-trip. Required for Round trip searches. When omitted for a round trip, return is set to 4 days after departure. |
| `tripType` | string | No | Choose a one-way or round-trip search. Round-trip routes use a return date. |
| `maxResults` | integer | No | Maximum dataset items to save across this entire run. |
| `maxPages` | integer | No | Maximum pages to request for each batch entry. |

## Output example

```json
{
  "resultId": "FL-JFK-LAX-20261114-01",
  "type": "ONE_WAY",
  "shareableId": "8f31b7",
  "shareableUrl": "https://www.kayak.com/flights/JFK-LAX/2026-11-14",
  "legs": [
    {
      "id": "leg-1",
      "departure": "2026-11-14T08:00:00Z",
      "arrival": "2026-11-14T11:00:00Z",
      "duration": 360
    }
  ],
  "totalProviders": 2,
  "totalBookingOptions": 3,
  "input_route": {
    "origin": "JFK",
    "destination": "LAX"
  },
  "scraped_at": "2026-10-01T12:00:00Z"
}
```

## Pricing

$0.30 per 1,000 results. Apify saves one dataset item per result and applies the Actor’s per-result price to saved items.

Pagination follows the source response. Set **maxPages** per batch entry and **maxResults** across the run.

## FAQ

### Is scraping this information legal?

Rules depend on the source, location, data type, and intended use. Check applicable laws and source terms, and make sure your workflow follows privacy and data protection requirements.

### What limits apply?

Submit up to 100 batch entries per run. Use **maxResults** to cap saved items and **maxPages** to limit pages for each entry. Results also depend on source availability and your Scrappa API plan.

### Can I start runs through the Apify API?

Yes. Send a POST request to `https://api.apify.com/v2/acts/thescrappa~kayak-flights-scraper/runs` with your Actor input, or use an Apify client library. Read the output from the run dataset.

### Can I connect the results to other tools?

Yes. Apify integrations and APIs can pass dataset results to Make, Zapier, n8n, Google Sheets, and other data workflows.

### What happens when one input fails?

The Actor logs a short source error and continues with the remaining entries. Transient rate limits and gateway errors are retried; if every entry fails, the run reports an error.

### How are travel dates chosen when omitted?

Departure defaults to 30 days from the run date. A round-trip return defaults to four days later; dates can be overridden per route or at the top level.

## Related Scrappa Actors

- [Google Flights Scraper](https://apify.com/thescrappa/google-flights-search-scraper)
- [Booking.com Search Scraper](https://apify.com/thescrappa/booking-search-scraper)
- [Booking.com Hotel Details Scraper](https://apify.com/thescrappa/booking-hotel-details-scraper)
