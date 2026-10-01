# Kayak Vacation Rentals Scraper

Search Kayak vacation rentals by property path and location ID with names, ratings, amenities, provider prices, and links. Omitted dates default to a future two-night visit.

## What data can you extract?

| Field | Type | Description |
| --- | --- | --- |
| `resultId` | String | KAYAK identifier for the vacation-rental search result. |
| `resultType` | String | ResultType value associated with this kayak vacation rentals record. |
| `localizedHotelName` | String | Rental or property name shown in the selected KAYAK locale. |
| `stars` | Integer | Stars value associated with this kayak vacation rentals record. |
| `hid` | Integer | Hid value associated with this kayak vacation rentals record. |
| `geolocation` | Object | Rental coordinates and localized city returned by KAYAK. |
| `rating` | Object | Guest score, rating label, and review total for the property. |
| `detailsUrl` | String | DetailsUrl value associated with this kayak vacation rentals record. |
| `url` | String | Public source or listing URL associated with this record. |
| `providers` | Array\<Object\> | Booking providers and quoted total prices for this rental. |
| `amenities` | Array\<Object\> | Amenity codes and labels reported for the property. |
| `input_rental_search` | Object | KAYAK rental path and location ID supplied for this search. |
| `scraped_at` | String | UTC date and time when this record was collected. |

## Use cases

- Travel planners can compare vacation rental options for a destination.
- Rental analysts can track property ratings, amenities, and quoted totals.
- Property managers can benchmark public rental inventory by market.

## How to use

1. Add a KAYAK rental path and matching location ID to each entry in `rentals`.
2. Set stay dates or use the relative date defaults.
3. Limit saved listings with `maxResults` and export the property rows.

```json
{
  "rentals": [
    {
      "path": "/Seattle-Vacation-Rentals.7054.rental.ksp",
      "location_id": "7054"
    }
  ],
  "location_type": "city",
  "adults": 2,
  "rooms": 1,
  "sort": "rank_a",
  "maxResults": 10
}
```

## Input

| Field | Type | Required | Description |
| --- | --- | --- | --- |
| `rentals` | Array\<object\> | Yes | KAYAK vacation-rental paths and their location IDs. |
| `rentals[].path` | string | Yes per entry | KAYAK property path copied from a hotel or rental search URL. |
| `rentals[].location_id` | string | Yes per entry | KAYAK location identifier for the city, airport, or property search. |
| `rentals[].location_type` | string | No | Type of KAYAK location identifier, such as a city, airport, or hotel. Accepted values: city. |
| `rentals[].checkin` | string | No | Hotel check-in date in YYYY-MM-DD format. When omitted, check-in is set to 30 days from today. |
| `rentals[].checkout` | string | No | Hotel check-out date in YYYY-MM-DD format, after check-in. When omitted, check-out is set to two days after check-in. |
| `rentals[].adults` | integer | No | Number of adult travelers included in the search. |
| `rentals[].rooms` | integer | No | Number of hotel rooms requested. |
| `rentals[].child_age` | integer | No | Age of one child in the hotel search, from 0 to 17. |
| `rentals[].sort` | string | No | Sort order for returned search results. Accepted values: rank\_a, price\_a, review\_a. |
| `location_id` | string | No | KAYAK location identifier for the city, airport, or property search. |
| `location_type` | string | No | Type of KAYAK location identifier, such as a city, airport, or hotel. Accepted values: city. |
| `checkin` | string | No | Hotel check-in date in YYYY-MM-DD format. When omitted, check-in is set to 30 days from today. |
| `checkout` | string | No | Hotel check-out date in YYYY-MM-DD format, after check-in. When omitted, check-out is set to two days after check-in. |
| `adults` | integer | No | Number of adult travelers included in the search. |
| `rooms` | integer | No | Number of hotel rooms requested. |
| `child_age` | integer | No | Age of one child in the hotel search, from 0 to 17. |
| `sort` | string | No | Sort order for returned search results. Accepted values: rank\_a, price\_a, review\_a. |
| `maxResults` | integer | No | Maximum dataset items to save across this entire run. |

## Output example

```json
{
  "resultId": "rental-7054-221",
  "resultType": "vacation-rental",
  "localizedHotelName": "Cedar House Seattle",
  "stars": 4,
  "hid": 8051221,
  "geolocation": {
    "point": {
      "lat": 47.608,
      "lng": -122.336
    },
    "localizedCity": "Seattle"
  },
  "rating": {
    "score": 9.1,
    "reviewCount": 126,
    "localizedRatingCategory": "Wonderful"
  },
  "detailsUrl": "https://www.kayak.com/Seattle-Vacation-Rentals.7054.rental.ksp",
  "url": "https://www.kayak.com/homes/cedar-house-seattle",
  "providers": [
    {
      "localizedProviderName": "Vrbo",
      "providerCode": "vrbo",
      "totalPrice": {
        "price": 642,
        "currency": "USD"
      }
    }
  ],
  "amenities": [
    {
      "code": "kitchen",
      "localizedName": "Kitchen"
    },
    {
      "code": "wifi",
      "localizedName": "Wi-Fi"
    }
  ],
  "input_rental_search": {
    "path": "/Seattle-Vacation-Rentals.7054.rental.ksp",
    "location_id": "7054"
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

Yes. Send a POST request to `https://api.apify.com/v2/acts/thescrappa~kayak-vacation-rentals-scraper/runs` with your Actor input, or use an Apify client library. Read the output from the run dataset.

### Can I connect the results to other tools?

Yes. Apify integrations and APIs can pass dataset results to Make, Zapier, n8n, Google Sheets, and other data workflows.

### What happens when one input fails?

The Actor logs a short source error and continues with the remaining entries. Transient rate limits and gateway errors are retried; if every entry fails, the run reports an error.

### Where do I find a rental location ID?

Copy the location code from a KAYAK vacation rental search URL or reuse the ID returned for that market.

## Related Scrappa Actors

- [Kayak Hotels Search Scraper](https://apify.com/thescrappa/kayak-hotels-search-scraper)
- [Kayak Hotel Rates Scraper](https://apify.com/thescrappa/kayak-hotel-rates-scraper)
- [Kayak Flights Scraper](https://apify.com/thescrappa/kayak-flights-scraper)
