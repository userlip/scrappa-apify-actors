# Kayak Flights Scraper

Search Kayak one-way or round-trip flights and collect flight options, airlines, providers, and routes.

## Data you get

- **resultId**: Kayak flight result identifier.
- **type**: Flight itinerary type.
- **shareableUrl**: Link to the flight search result.
- **legs**: Flight legs, including route and timing.
- **totalProviders**: Number of providers offering the itinerary.

## Use cases

- Flight price monitoring
- Route comparison
- Travel planning research

## How to use

Add one or more entries to **flights**. Each entry maps its **origin** value to the Scrappa **origin** input. Shared endpoint options can be set at the top level.

```json
{
  "flights": [
    {
      "origin": "JFK"
    }
  ],
  "destination": "LAX",
  "departure_date": "2026-11-14",
  "tripType": "one-way",
  "maxResults": 20,
  "maxPages": 2
}
```

## Output example

This synthetic example shows the response fields and the input value attached to each result.

```json
{
  "resultId": "synthetic-flight-1",
  "type": "ONE_WAY",
  "shareableId": "example-route",
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
  "input_origin": "JFK",
  "scraped_at": "2026-01-01T00:00:00Z"
}
```

## Pricing

$0.30 per 1,000 results. The Actor writes one dataset item for each result.

This Actor supports pagination and stops at the configured **maxPages** or **maxResults** limit.

## FAQ

### Is scraping this data legal?

Scraping rules depend on the source, the data, and how you use it. Review the applicable laws, source terms, and privacy requirements for your use case. You are responsible for your collection and use of the data.

### Are there request limits?

You can submit up to 100 batch entries per run. Set **maxResults** to cap saved rows and **maxPages** to bound pagination for each entry. Scrappa API limits and source availability also apply.

### Can I use the output with integrations or the API?

Yes. Read results from the Apify dataset, use the Apify API or client libraries, or connect the dataset to your existing data workflow. Each row includes **input_origin** and **scraped_at** for traceability.

## Related Actors

- [Google Flights Search Scraper](https://apify.com/thescrappa/google-flights-search-scraper)
- [Booking Search Scraper](https://apify.com/thescrappa/booking-search-scraper)

## Search terms

`Kayak Flights Scraper`, `resultId`, `type`, `shareableUrl`, `/kayak/flights/one-way API`
