# Google Hotels Autocomplete Scraper

Find hotel and destination suggestions on Google Hotels before searching for a stay. Use a city, district or hotel name to get a property suggestion that can refine a later stay search.

## What data can you extract?

Suggestions and destination tokens follow the hotel or place name entered; not every suggestion has a property token.

| Field | Type | Description |
| --- | --- | --- |
| `position` | number | Result position in the Google Hotels source record list, as a whole number; null when the source does not supply one. |
| `value` | text | Place or hotel name matched by Google Hotels autocomplete; null when the suggestion has no display label. |
| `autocomplete_suggestion` | text | Full hotel or destination phrase suggested for the input query; null when Google Hotels provides no expanded phrase. |
| `type` | text | Google Hotels suggestion category, such as location or accommodation; null when no category is supplied. |
| `property_token` | text | Token Google Hotels uses to identify a property or destination suggestion; null when the match has no token. |
| `thumbnail` | image | Thumbnail url for this source record on Google Hotels; null when the source does not provide a URL. |
| `scrappa_google_hotels_link` | link | Scrappa google hotels link for this source record on Google Hotels; null when the source does not provide a URL. |
| `source_query` | text | Place or hotel name submitted for this autocomplete lookup. |
| `request_gl` | text | Two-letter country or region code passed to Google Hotels; Use a two-letter country code such as us or de. This input value is copied into the output row; null when it was not supplied. |
| `request_hl` | text | Interface language code passed to Google Hotels; Use a language code such as en or de. This input value is copied into the output row; null when it was not supplied. |
| `request_currency` | text | Three-letter currency code passed to Google Hotels; Use a three-letter code such as USD or EUR. This input value is copied into the output row; null when it was not supplied. |
| `request_type` | text | Requested result type passed to Google Hotels. This input value is copied into the output row; null when it was not supplied. |
| `response_time_ms` | number | Response time for this Google Hotels lookup, measured in milliseconds; null when no timing value was recorded. |

## Use cases

- Travelers can find recognized destination or hotel names before starting a stay search.
- Travel teams can help users match a typed city or property name to Google Hotels suggestions.
- Researchers can compare destination and property suggestions across markets.

## How to use

1. Open the Actor’s **Input** tab and start with the JSON below.
2. Put the supported targets in `queries` and use the identifier or URL format required by Google Hotels.
3. Start the run and open its default dataset to inspect or download the rows.

```json
{
  "queries": [
    "Berlin"
  ]
}
```

The run dataset can be downloaded as JSON, CSV, Excel or another format offered by Apify.

## Input

| Field | Type | Required | Description |
| --- | --- | --- | --- |
| `queries` | array of string | No | Destination, landmark, area, or hotel-name prefixes. You can also provide a comma-separated string through the API. Constraints: minimum 1 items; maximum 100 items. |
| `q` | string | No | Compatibility alias for one query. Queries and q are combined and deduplicated when both are provided. |
| `gl` | string | No | Two-letter Google country code, such as de, us, gb, or fr. |
| `hl` | string | No | Two-letter language code, such as en, de, es, or fr. |
| `currency` | string | No | Three-letter currency code used in generated hotel-search links. |
| `type` | string | No | Return locations, hotels/accommodations, or both. Constraints: allowed values: location, hotel, all. |

## Output example

The record below is synthetic. Names and links are examples, and private contact fields are omitted.

```json
{
  "value": "Juniper House Hotel, Portland",
  "autocomplete_suggestion": "Juniper House Hotel, Portland",
  "type": "accommodation",
  "property_token": "Cg9qLW1vY2stcHJvcGVydHk",
  "thumbnail": "https://images.example.com/hotels/juniper-house.jpg",
  "scrappa_google_hotels_link": "https://hotels.example.com/search?property=juniper-house",
  "source_query": "Portland, OR",
  "response_time_ms": 348
}
```

## Pricing

**Current live price:** $0.25 per 1,000 results.

Each saved autocomplete suggestion row counts as one result.

## FAQ

### Is it legal to collect public information?

This Actor is for information visible on the public source pages it reads. You are responsible for checking the source terms, privacy and copyright rules, and the laws that apply to your use.

### How many records will a run return?

The number of rows depends on the suggestions Google Hotels returns for each query. The Actor does not expose pagination controls.

### Can I call it through the API or connect it to other tools?

Yes. Send the same JSON input to `POST https://api.apify.com/v2/acts/thescrappa~google-hotels-autocomplete-scraper/runs`, then read the run’s default dataset. Make, Zapier and n8n can start runs and pass dataset rows to the next step; Google Sheets can receive rows through those workflows or a dataset export.

### What happens when a request fails?

Check the run log for the source or input error, correct the input and retry after a temporary source issue. A run can contain rows saved before a later request failed.

### How do I use a Google Hotels suggestion in a stay search?

Enter a destination or hotel name in `q`. The returned suggestion can include a property token that Google Hotels Search accepts for a more specific lookup.

## Related Scrappa Actors

- [Booking.com Hotel Details Scraper](https://apify.com/thescrappa/booking-hotel-details-scraper)
- [Booking.com Search Scraper](https://apify.com/thescrappa/booking-search-scraper)
- [Google Flights Scraper](https://apify.com/thescrappa/google-flights-search-scraper)
- [Google Hotels Search Scraper](https://apify.com/thescrappa/google-hotels-search-scraper)
- [Google Maps Directions Scraper](https://apify.com/thescrappa/google-maps-directions-scraper)
