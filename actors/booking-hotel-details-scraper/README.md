# Booking.com Hotel Details Scraper

Review a Booking.com hotel page with its property name, address, guest rating and check-in details. Submit a hotel URL or country and slug, with batch input for multiple properties.

## What data can you extract?

Property details and nightly rates reflect the selected stay and the details Booking.com displays.

| Field | Type | Description |
| --- | --- | --- |
| `title` | text | Title of the hotel listing, as shown by Booking.com; null when no title is published. |
| `canonical_url` | link | Canonical page url for this hotel listing on Booking.com; null when the source does not provide a URL. |
| `hotel_schema` | object | Schema.org hotel details with name, street and city address fields, aggregateRating and check-in and check-out times; null when the page has no hotel schema. |
| `aggregate_rating` | object | Booking.com rating summary with ratingValue on the 10-point guest scale, reviewCount and bestRating; null when the page has no rating data. |
| `json_ld` | array of objects | Schema.org records with @type, name, address and aggregateRating fields; an empty list when the page has no structured hotel records. |
| `parsed` | boolean | Whether the page was parsed; false is a reported value, while null means Booking.com provided no flag. |
| `request_index` | number | Zero-based position of this request in the submitted Booking.com input batch; null for a single-item lookup. |
| `request_input_type` | text | Input type passed to Booking.com. This input value is copied into the output row; null when it was not supplied. |
| `request_url` | link | Source page url passed to Booking.com. This input value is copied into the output row; null when it was not supplied. |
| `request_country` | text | Country code or country name passed to Booking.com. This input value is copied into the output row; null when it was not supplied. |
| `request_slug` | text | Source hotel or shop slug passed to Booking.com. This input value is copied into the output row; null when it was not supplied. |
| `request_success` | boolean | Lookup success flag passed to Booking.com. This input value is copied into the output row; null when it was not supplied. |
| `error_message` | text | Diagnostic text for the Booking.com lookup; null when the request completes without an error. |

## Use cases

- Travelers can compare fares or nightly rates before choosing a trip.
- Travel teams can check public options across routes, destinations and dates.
- Researchers can track prices and ratings in a travel market.

## How to use

1. Open the Actor’s **Input** tab and start with the JSON below.
2. Put the supported targets in `urls` and use the identifier or URL format required by Booking.com.
3. Start the run and open its default dataset to inspect or download the rows.

```json
{
  "country": "fr",
  "slug": "ritz-paris",
  "urls": [
    "https://www.booking.com/hotel/fr/ritz-paris.html"
  ]
}
```

The run dataset can be downloaded as JSON, CSV, Excel or another format offered by Apify.

## Input

| Field | Type | Required | Description |
| --- | --- | --- | --- |
| `url` | string | No | Full Booking.com hotel URL for a single hotel detail request. If URL is provided, it takes precedence over Country and Slug. |
| `country` | string | No | Two-letter country code from the Booking.com hotel URL, such as fr, us, de, or gb. Use with Slug. |
| `slug` | string | No | Booking.com hotel slug, such as ritz-paris. The trailing .html is optional. |
| `urls` | array of string | No | Optional list of Booking.com hotel URLs to process in one actor run. Constraints: maximum 10 items. |
| `hotels` | array of object | No | Optional list of hotel request objects. Each item can include { "url" } or { "country", "slug" }. Constraints: maximum 10 items. |

## Output example

The record below is synthetic. Names and links are examples, and private contact fields are omitted.

```json
{
  "title": "Juniper House Hotel in Portland",
  "canonical_url": "https://www.booking.com/hotel/us/juniper-house.html",
  "hotel_schema": {
    "@type": "Hotel",
    "name": "Juniper House Hotel",
    "address": {
      "streetAddress": "18 Pine Avenue",
      "addressLocality": "Portland",
      "addressRegion": "OR",
      "addressCountry": "US"
    },
    "aggregateRating": {
      "ratingValue": "8.8",
      "reviewCount": 184
    },
    "checkinTime": "15:00",
    "checkoutTime": "11:00"
  },
  "aggregate_rating": {
    "ratingValue": "8.8",
    "reviewCount": 184,
    "bestRating": "10"
  },
  "json_ld": [
    {
      "@type": "Hotel",
      "name": "Juniper House Hotel",
      "aggregateRating": {
        "ratingValue": "8.8",
        "reviewCount": 184
      }
    }
  ],
  "parsed": true
}
```

## Pricing

**Current live price:** $0.30 per 1,000 results.

Each saved hotel record counts as one result.

## FAQ

### Is it legal to collect public information?

This Actor is for information visible on the public source pages it reads. You are responsible for checking the source terms, privacy and copyright rules, and the laws that apply to your use.

### How many records will a run return?

The result count depends on the input limits, pagination settings and what the source makes available. A small query or unavailable page can return fewer records, including none.

### Can I call it through the API or connect it to other tools?

Yes. Send the same JSON input to `POST https://api.apify.com/v2/acts/thescrappa~booking-hotel-details-scraper/runs`, then read the run’s default dataset. Make, Zapier and n8n can start runs and pass dataset rows to the next step; Google Sheets can receive rows through those workflows or a dataset export.

### What happens when a request fails?

Check the run log for the source or input error, correct the input and retry after a temporary source issue. A run can contain rows saved before a later request failed.

### Can Booking.com Hotel Details take a full hotel URL?

Yes. Submit one URL in `url` or a list in `urls`. You can also provide a country code and hotel slug; the page must be accessible to retrieve its public schema.

## Related Scrappa Actors

- [Booking.com Search Scraper](https://apify.com/thescrappa/booking-search-scraper)
- [Google Flights Scraper](https://apify.com/thescrappa/google-flights-search-scraper)
- [Google Hotels Autocomplete Scraper](https://apify.com/thescrappa/google-hotels-autocomplete-scraper)
- [Google Hotels Search Scraper](https://apify.com/thescrappa/google-hotels-search-scraper)
- [Google Maps Directions Scraper](https://apify.com/thescrappa/google-maps-directions-scraper)
