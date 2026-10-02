# Kayak Hotels Search Scraper

Search Kayak for hotel listings with names, star levels, review scores, amenities, provider prices, and property links. Batch location IDs for future stays.

## What data can you extract?

| Field | Type | Description |
| --- | --- | --- |
| `resultId` | String | KAYAK identifier for this hotel search result. |
| `resultType` | String | ResultType value associated with this kayak hotels search record. |
| `localizedHotelName` | String | Hotel name shown in the selected KAYAK locale. |
| `stars` | Integer | Stars value associated with this kayak hotels search record. |
| `hid` | Integer | KAYAK hotel identifier used to request property rates or reviews. |
| `geolocation` | Object | Property coordinates and localized city returned by KAYAK. |
| `rating` | Object | Guest score, rating label, and review total displayed for the property. |
| `detailsUrl` | String | DetailsUrl value associated with this kayak hotels search record. |
| `url` | String | Public source or listing URL associated with this record. |
| `providers` | Array\<Object\> | Booking providers and quoted total prices shown for this hotel. |
| `amenities` | Array\<Object\> | Amenity codes and labels listed for the property. |
| `input_location_id` | String | KAYAK location identifier submitted for this hotel search. |
| `scraped_at` | String | UTC date and time when this record was collected. |

## Use cases

- Travel planners can compare hotel choices and booking providers by destination.
- Hospitality analysts can benchmark ratings, star levels, and amenities.
- Travel agencies can build property shortlists for upcoming trips.

## How to use

1. Add a KAYAK location ID to each entry in `locations`.
2. Set stay dates and guest options or use the relative date defaults.
3. Use `maxResults` to cap saved property offers.

```json
{
  "locations": [
    {
      "location_id": "15830"
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
| `locations` | Array\<object\> | Yes | City, airport, or property location IDs to search for hotel offers. |
| `locations[].location_id` | string | Yes per entry | KAYAK location identifier for the city, airport, or property search. |
| `locations[].location_type` | string | No | Type of KAYAK location identifier, such as a city, airport, or hotel. Accepted values: city, airport, hotel. |
| `locations[].checkin` | string | No | Hotel check-in date in YYYY-MM-DD format. When omitted, check-in is set to 30 days from today. |
| `locations[].checkout` | string | No | Hotel check-out date in YYYY-MM-DD format, after check-in. When omitted, check-out is set to two days after check-in. |
| `locations[].adults` | integer | No | Number of adult travelers included in the search. |
| `locations[].rooms` | integer | No | Number of hotel rooms requested. |
| `locations[].child_age` | integer | No | Age of one child in the hotel search, from 0 to 17. |
| `locations[].sort` | string | No | Sort order for returned search results. Accepted values: rank\_a, price\_a, review\_a. |
| `location_type` | string | No | Type of KAYAK location identifier, such as a city, airport, or hotel. Accepted values: city, airport, hotel. |
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
  "resultId": "hotel-15830-1042",
  "resultType": "hotel",
  "localizedHotelName": "Park MGM Las Vegas",
  "stars": 4,
  "hid": 15297,
  "geolocation": {
    "point": {
      "lat": 36.103,
      "lng": -115.169
    },
    "localizedCity": "Las Vegas"
  },
  "rating": {
    "score": 8.7,
    "reviewCount": 8420,
    "localizedRatingCategory": "Excellent"
  },
  "detailsUrl": "https://www.kayak.com/Las-Vegas-Hotels-Park-MGM-Las-Vegas.15297.ksp",
  "url": "https://www.kayak.com/hotels/park-mgm-las-vegas",
  "providers": [
    {
      "localizedProviderName": "Booking.com",
      "providerCode": "booking",
      "totalPrice": {
        "price": 318,
        "currency": "USD"
      }
    }
  ],
  "amenities": [
    {
      "code": "pool",
      "localizedName": "Pool"
    },
    {
      "code": "wifi",
      "localizedName": "Wi-Fi"
    }
  ],
  "input_location_id": "15830",
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

Yes. Send a POST request to `https://api.apify.com/v2/acts/thescrappa~kayak-hotels-search-scraper/runs` with your Actor input, or use an Apify client library. Read the output from the run dataset.

### Can I connect the results to other tools?

Yes. Apify integrations and APIs can pass dataset results to Make, Zapier, n8n, Google Sheets, and other data workflows.

### What happens when one input fails?

The Actor logs a short source error and continues with the remaining entries. Transient rate limits and gateway errors are retried; if every entry fails, the run reports an error.

### What is a KAYAK location ID?

It is the location code used by KAYAK for a city, airport, or property. Copy it from a supported search URL or a previous result.

## Related Scrappa Actors

- [Kayak Flights Scraper](https://apify.com/thescrappa/kayak-flights-scraper)
- [Kayak Hotel Rates Scraper](https://apify.com/thescrappa/kayak-hotel-rates-scraper)
- [Kayak Hotel Details Scraper](https://apify.com/thescrappa/kayak-hotel-details-scraper)
