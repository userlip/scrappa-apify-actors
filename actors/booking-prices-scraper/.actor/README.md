# Booking.com Hotel Prices Scraper

Search Booking.com hotel price cards by destination and future stay dates. Collect property names, quoted prices, review scores, locations, and listing links for comparison.

## What data can you extract?

| Field | Type | Description |
| --- | --- | --- |
| `name` | String | Name displayed by the source for this record. |
| `url` | String | Public source or listing URL associated with this record. |
| `price` | String | Quoted price in the currency shown by the source. |
| `review_score` | String | Review Score value associated with this booking.com hotel prices record. |
| `location` | String | Location information reported for this source record. |
| `image` | String | Public image URL supplied by the source. |
| `input_ss` | String | Destination name submitted for the Booking.com price search. |
| `scraped_at` | String | UTC date and time when this record was collected. |

## Use cases

- Travel planners can compare hotel price cards across destination cities.
- Market analysts can track quoted lodging rates for future stays.
- Travel agencies can create shortlists from public property prices and review scores.

## How to use

1. Add a city or destination to each entry in `destinations`.
2. Set stay dates, guest counts, and currency, or rely on the relative date defaults.
3. Use `maxResults` to limit saved hotel price cards.

```json
{
  "destinations": [
    {
      "ss": "Berlin"
    }
  ],
  "group_adults": 2,
  "no_rooms": 1,
  "dest_type": "city",
  "lang": "en-us",
  "currency": "EUR",
  "maxResults": 10
}
```

## Input

| Field | Type | Required | Description |
| --- | --- | --- | --- |
| `destinations` | Array\<object\> | Yes | Cities, regions, or destination names to search for Booking.com hotel prices. |
| `destinations[].ss` | string | Yes per entry | Destination, e.g. "Paris" or "New York". |
| `destinations[].checkin` | string | No | Hotel check-in date in YYYY-MM-DD format. When omitted, check-in is set to 30 days from today. |
| `destinations[].checkout` | string | No | Hotel check-out date in YYYY-MM-DD format, after check-in. When omitted, check-out is set to two days after check-in. |
| `destinations[].group_adults` | integer | No | Number of adults \(1-30\). Defaults to Booking.com default. |
| `destinations[].group_children` | integer | No | Number of children \(0-20\). |
| `destinations[].no_rooms` | integer | No | Number of rooms \(1-30\). |
| `destinations[].dest_id` | integer | No | Booking.com destination id when known \(e.g. -1456928 for Paris\). Helps Booking render resolved city result pages. |
| `destinations[].dest_type` | string | No | Booking.com destination type when known: city, region, district, hotel, landmark, or airport. Accepted values: city, region, district, hotel, landmark, airport. |
| `destinations[].lang` | string | No | UI language hint \(e.g. en-us, de\). |
| `destinations[].currency` | string | No | 3-letter currency code \(e.g. EUR, USD\). |
| `checkin` | string | No | Hotel check-in date in YYYY-MM-DD format. When omitted, check-in is set to 30 days from today. |
| `checkout` | string | No | Hotel check-out date in YYYY-MM-DD format, after check-in. When omitted, check-out is set to two days after check-in. |
| `group_adults` | integer | No | Number of adults \(1-30\). Defaults to Booking.com default. |
| `group_children` | integer | No | Number of children \(0-20\). |
| `no_rooms` | integer | No | Number of rooms \(1-30\). |
| `dest_id` | integer | No | Booking.com destination id when known \(e.g. -1456928 for Paris\). Helps Booking render resolved city result pages. |
| `dest_type` | string | No | Booking.com destination type when known: city, region, district, hotel, landmark, or airport. Accepted values: city, region, district, hotel, landmark, airport. |
| `lang` | string | No | UI language hint \(e.g. en-us, de\). |
| `currency` | string | No | 3-letter currency code \(e.g. EUR, USD\). |
| `maxResults` | integer | No | Maximum dataset items to save across this entire run. |

## Output example

```json
{
  "name": "The Linden Berlin",
  "url": "https://www.booking.com/hotel/de/the-linden-berlin.html",
  "price": "€246",
  "review_score": "9.1",
  "location": "Mitte, Berlin",
  "image": "https://cf.bstatic.com/xdata/images/hotel/square60/berlin-linden.jpg",
  "input_ss": "Berlin",
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

Yes. Send a POST request to `https://api.apify.com/v2/acts/thescrappa~booking-prices-scraper/runs` with your Actor input, or use an Apify client library. Read the output from the run dataset.

### Can I connect the results to other tools?

Yes. Apify integrations and APIs can pass dataset results to Make, Zapier, n8n, Google Sheets, and other data workflows.

### What happens when one input fails?

The Actor logs a short source error and continues with the remaining entries. Transient rate limits and gateway errors are retried; if every entry fails, the run reports an error.

### Can I search a Booking.com destination ID?

Yes. Add `dest_id` and the matching `dest_type` to help resolve a destination when you have its Booking.com ID.

## Related Scrappa Actors

- [Booking.com Search Scraper](https://apify.com/thescrappa/booking-search-scraper)
- [Booking.com Rooms & Rates Scraper](https://apify.com/thescrappa/booking-rooms-scraper)
- [Booking.com Hotel Details Scraper](https://apify.com/thescrappa/booking-hotel-details-scraper)
