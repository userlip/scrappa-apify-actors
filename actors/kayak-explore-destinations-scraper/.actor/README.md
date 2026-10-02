# Kayak Explore Cheapest Destinations Scraper

Explore Kayak destinations from origin airports with city, country, departure and return windows, airline, stop count, and flight link. Choose a geographic search area.

## What data can you extract?

| Field | Type | Description |
| --- | --- | --- |
| `days` | Integer | Trip length in days for the suggested destination option. |
| `originAirportCode` | String | Airport code used as the departure point. |
| `dateDepart` | String | Suggested outbound travel date. |
| `dateReturn` | String | Suggested return date for the destination option. |
| `airline` | String | Airline name shown for the suggested flight. |
| `airlineCode` | String | Airline code associated with the suggestion. |
| `clickoutUrl` | String | KAYAK flight-search URL for this suggested trip. |
| `flightMaxStops` | Integer | Maximum stop count for the candidate flight. |
| `flightMaxDuration` | Integer | Maximum flight duration reported in minutes. |
| `city` | String | Destination city suggested by KAYAK. |
| `country` | String | Country containing the suggested destination. |
| `airport` | String | Airport name associated with the destination. |
| `imageUrl` | String | Destination image URL returned by KAYAK. |
| `input_origin` | String | Origin airport code submitted for destination exploration. |
| `scraped_at` | String | UTC date and time when this record was collected. |

## Use cases

- Flexible travelers can discover destinations within a selected map area.
- Travel planners can compare candidate cities from several origin airports.
- Fare analysts can review dates and airlines for exploratory trip searches.

## How to use

1. Add an origin airport code to each entry in `origins`.
2. Set departure and return windows or use the relative date defaults.
3. Enter northeast and southwest map bounds, then cap saved destinations with `maxResults`.

```json
{
  "origins": [
    {
      "origin": "ICT"
    }
  ],
  "north_east_lat": 70,
  "north_east_lng": -50,
  "south_west_lat": 15,
  "south_west_lng": -170,
  "stops": 1,
  "maxResults": 10
}
```

## Input

| Field | Type | Required | Description |
| --- | --- | --- | --- |
| `origins` | Array\<object\> | Yes | Origin airports whose lowest available destinations you want to explore. |
| `origins[].origin` | string | Yes per entry | Origin airport code for the route or destination search. |
| `origins[].departure_from` | string | No | First outbound date in the destination exploration window. When omitted, the departure window starts 30 days from today. |
| `origins[].departure_to` | string | No | Last outbound date in the destination exploration window. When omitted, the departure window ends seven days after it starts. |
| `origins[].return_from` | string | No | First possible return date in the destination exploration window. When omitted, the return window starts seven days after the first departure date. |
| `origins[].return_to` | string | No | Last possible return date in the destination exploration window. When omitted, the return window ends seven days after it starts. |
| `origins[].min_days` | integer | No | Minimum trip length in days for destination suggestions. |
| `origins[].max_days` | integer | No | Maximum trip length in days for destination suggestions. |
| `origins[].stops` | integer | No | Maximum flight stop count for destination suggestions. |
| `origins[].north_east_lat` | number | No | Northern latitude of the northeast corner of the destination map area. |
| `origins[].north_east_lng` | number | No | Eastern longitude of the northeast corner of the destination map area. |
| `origins[].south_west_lat` | number | No | Southern latitude of the southwest corner of the destination map area. |
| `origins[].south_west_lng` | number | No | Western longitude of the southwest corner of the destination map area. |
| `departure_from` | string | No | First outbound date in the destination exploration window. When omitted, the departure window starts 30 days from today. |
| `departure_to` | string | No | Last outbound date in the destination exploration window. When omitted, the departure window ends seven days after it starts. |
| `return_from` | string | No | First possible return date in the destination exploration window. When omitted, the return window starts seven days after the first departure date. |
| `return_to` | string | No | Last possible return date in the destination exploration window. When omitted, the return window ends seven days after it starts. |
| `min_days` | integer | No | Minimum trip length in days for destination suggestions. |
| `max_days` | integer | No | Maximum trip length in days for destination suggestions. |
| `stops` | integer | No | Maximum flight stop count for destination suggestions. |
| `north_east_lat` | number | Yes | Northern latitude of the northeast corner of the destination map area. |
| `north_east_lng` | number | Yes | Eastern longitude of the northeast corner of the destination map area. |
| `south_west_lat` | number | Yes | Southern latitude of the southwest corner of the destination map area. |
| `south_west_lng` | number | Yes | Western longitude of the southwest corner of the destination map area. |
| `maxResults` | integer | No | Maximum dataset items to save across this entire run. |

## Output example

```json
{
  "days": 7,
  "originAirportCode": "ICT",
  "dateDepart": "2026-10-31",
  "dateReturn": "2026-11-07",
  "airline": "United Airlines",
  "airlineCode": "UA",
  "clickoutUrl": "https://www.kayak.com/flights/ICT-LAS/2026-10-31/2026-11-07",
  "flightMaxStops": 1,
  "flightMaxDuration": 295,
  "city": "Las Vegas",
  "country": "United States",
  "airport": "Harry Reid International Airport",
  "imageUrl": "https://images.kayak.com/las-vegas-strip.jpg",
  "input_origin": "ICT",
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

Yes. Send a POST request to `https://api.apify.com/v2/acts/thescrappa~kayak-explore-destinations-scraper/runs` with your Actor input, or use an Apify client library. Read the output from the run dataset.

### Can I connect the results to other tools?

Yes. Apify integrations and APIs can pass dataset results to Make, Zapier, n8n, Google Sheets, and other data workflows.

### What happens when one input fails?

The Actor logs a short source error and continues with the remaining entries. Transient rate limits and gateway errors are retried; if every entry fails, the run reports an error.

### How do the map bounds work?

Set northeast and southwest latitude and longitude values to describe the geographic area where destinations should be considered.

## Related Scrappa Actors

- [Kayak Flights Scraper](https://apify.com/thescrappa/kayak-flights-scraper)
- [Google Flights Cheapest Dates Scraper](https://apify.com/thescrappa/google-flights-date-range-scraper)
- [Kayak Flight Price Predictions Scraper](https://apify.com/thescrappa/kayak-flight-price-predictions-scraper)
