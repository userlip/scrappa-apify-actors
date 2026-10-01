# Google Hotels Search Scraper

Compare Google Hotels stays by property, guest rating, nightly rate and review count. Set future stay dates and guest counts to compare rates for the destination you choose.

## What data can you extract?

Hotel rates reflect the searched stay and displayed currency; availability and offers can change between searches.

| Field | Type | Description |
| --- | --- | --- |
| `name` | text | Name of the hotel listing, as shown by Google Hotels; null when no name is published. |
| `type` | text | Property type shown by Google Hotels, such as hotel or vacation rental; null when no type is shown. |
| `hotel_class` | text | Hotel class shown for the hotel listing by Google Hotels, in the format used by the source; null when it is omitted. |
| `overall_rating` | number | Guest rating on Google Hotels’ 1-to-5 star scale; null when no rating is shown. |
| `reviews` | number | Number of Google Hotels reviews for the property; null if no review total is shown. |
| `rate_per_night_lowest` | text | Lowest nightly rate formatted as Google Hotels displays it, including currency; null when no rate is listed. |
| `rate_per_night_extracted_lowest` | number | Lowest nightly rate parsed as a number in the searched currency; null when no rate is listed. |
| `total_rate_lowest` | text | Lowest stay total formatted as Google Hotels displays it, including currency; null when no total is listed. |
| `total_rate_extracted_lowest` | number | Lowest stay total parsed as a number in the searched currency; null when no total is listed. |
| `booking_link` | link | Booking link for this hotel listing on Google Hotels; null when the source does not provide a URL. |
| `property_token` | text | Google Hotels identifier for the property; when Google omits a token, this field falls back to the result’s `entity_id`. |
| `entity_id` | text | Google Hotels property ID for the hotel listing, assigned by Google Hotels; null when the source does not expose it. |
| `place_id` | text | Google Maps place ID for the hotel listing, assigned by Google Hotels; null when the source does not expose it. |
| `latitude` | number | Latitude for this hotel listing on Google Hotels, in decimal degrees; null when the source provides no coordinates. |
| `longitude` | number | Longitude for this hotel listing on Google Hotels, in decimal degrees; null when the source provides no coordinates. |
| `thumbnail` | image | Thumbnail url for this hotel listing on Google Hotels; null when the source does not provide a URL. |
| `price_sources_count` | number | Number of available price-sources shown by Google Hotels, as a whole number; zero is possible, and null means no count was reported. |
| `amenities_count` | number | Number of amenities shown by Google Hotels, as a whole number; zero is possible, and null means no count was reported. |
| `request_q` | text | Search phrase passed to Google Hotels. This input value is copied into the output row; null when it was not supplied. |
| `request_check_in_date` | date | Resolved check-in date sent to Google Hotels, formatted as `YYYY-MM-DD`; present on successful result rows. |
| `request_check_out_date` | date | Resolved check-out date sent to Google Hotels, formatted as `YYYY-MM-DD`; present on successful result rows. |
| `request_adults` | number | Adults passed to Google Hotels. This input value is copied into the output row; null when it was not supplied. |
| `request_children` | number | Children passed to Google Hotels. This input value is copied into the output row; null when it was not supplied. |
| `request_currency` | text | Three-letter currency code passed to Google Hotels; Use a three-letter code such as USD or EUR. This input value is copied into the output row; null when it was not supplied. |
| `request_gl` | text | Two-letter country or region code passed to Google Hotels; Use a two-letter country code such as us or de. This input value is copied into the output row; null when it was not supplied. |
| `request_hl` | text | Interface language code passed to Google Hotels; Use a language code such as en or de. This input value is copied into the output row; null when it was not supplied. |

## Use cases

- Travelers can compare fares or nightly rates before choosing a trip.
- Travel teams can check public options across routes, destinations and dates.
- Researchers can track prices and ratings in a travel market.

## How to use

1. Open the Actor’s **Input** tab and start with the JSON below.
2. Enter a hotel name, neighborhood or destination in `q`, then provide check-in and check-out dates for the stay.
3. Start the run and open its default dataset to inspect or download the rows.

```json
{
  "q": "Paris, France",
  "check_in_date": "tomorrow",
  "check_out_date": "day-after-tomorrow"
}
```

The run dataset can be downloaded as JSON, CSV, Excel or another format offered by Apify.

## Input

| Field | Type | Required | Description |
| --- | --- | --- | --- |
| `q` | string | Yes | Location, landmark, area, or hotel query to search in Google Hotels. |
| `check_in_date` | string | Yes | Check-in date in YYYY-MM-DD format, or use today, tomorrow, or day-after-tomorrow. Must resolve to today or a future date. |
| `check_out_date` | string | Yes | Check-out date in YYYY-MM-DD format, or use tomorrow or day-after-tomorrow. Must resolve after the check-in date. |
| `adults` | integer | No | Number of adults. Constraints: minimum 1; maximum 10. |
| `children` | integer | No | Number of children. If greater than 0, provide matching Children Ages. Constraints: minimum 0; maximum 6. |
| `children_ages` | array of integer | No | Ages for each child, 1-17. The number of ages must match Children. |
| `currency` | string | No | Three-letter currency code such as USD, EUR, or GBP. |
| `gl` | string | No | Two-letter Google country code such as us, fr, gb, or de. |
| `hl` | string | No | Two-letter language code such as en, fr, de, or es. |
| `sort_by` | string | No | Sort hotel results by lowest price, highest rating, or most reviewed. Constraints: allowed values: 3, 8, 13. |
| `min_price` | integer | No | Minimum nightly price. Constraints: minimum 0. |
| `max_price` | integer | No | Maximum nightly price. Must be greater than Minimum Price and within 5000. Constraints: minimum 1; maximum 5000. |
| `hotel_class` | string | No | Filter results by 2-star, 3-star, 4-star, or 5-star hotel class. Constraints: allowed values: 2, 3, 4, 5. |
| `rating` | string | No | Filter results by a minimum guest rating of 3.5+, 4.0+, or 4.5+. Constraints: allowed values: 7, 8, 9. |
| `free_cancellation` | boolean | No | Return only properties with free cancellation. Cannot be combined with price, class, rating, amenities, property type, or brand filters. |
| `amenities` | array of integer | No | Google Hotels amenity IDs. |
| `vacation_rentals` | boolean | No | Search vacation rentals instead of hotels. |
| `eco_certified` | boolean | No | Return only eco-certified hotels. Cannot be combined with other non-boolean filters. |
| `special_offers` | boolean | No | Return only properties with special offers. Cannot be combined with other non-boolean filters. |
| `property_types` | array of integer | No | Google Hotels property type IDs. |
| `brands` | array of integer | No | Google Hotels brand IDs. |
| `bedrooms` | integer | No | Minimum bedrooms for vacation rental searches. Constraints: minimum 1; maximum 20. |
| `bathrooms` | integer | No | Minimum bathrooms for vacation rental searches. Constraints: minimum 1; maximum 20. |
| `next_page_token` | string | No | Pagination token returned by a previous Google Hotels search response. |
| `property_token` | string | No | Specific Google Hotels property token, often returned by Google Hotels autocomplete. |

## Output example

The record below is synthetic. Names and links are examples, and private contact fields are omitted.

```json
{
  "name": "Juniper House Hotel",
  "type": "Hotel",
  "hotel_class": "4-star hotel",
  "overall_rating": 4.6,
  "reviews": 184,
  "rate_per_night_lowest": "$184",
  "rate_per_night_extracted_lowest": 184,
  "total_rate_lowest": "$552"
}
```

## Pricing

**Current live price:** $0.20 per 1,000 results.

Each saved hotel record counts as one result.

## FAQ

### Is it legal to collect public information?

This Actor is for information visible on the public source pages it reads. You are responsible for checking the source terms, privacy and copyright rules, and the laws that apply to your use.

### How many records will a run return?

The result count depends on the input limits, pagination settings and what the source makes available. A small query or unavailable page can return fewer records, including none.

### Can I call it through the API or connect it to other tools?

Yes. Send the same JSON input to `POST https://api.apify.com/v2/acts/thescrappa~google-hotels-search-scraper/runs`, then read the run’s default dataset. Make, Zapier and n8n can start runs and pass dataset rows to the next step; Google Sheets can receive rows through those workflows or a dataset export.

### What happens when a request fails?

Check the run log for the source or input error, correct the input and retry after a temporary source issue. A run can contain rows saved before a later request failed.

### Which dates should I use for Google Hotels?

Set check-in and check-out to future dates and include guest or room counts when needed.

## Related Scrappa Actors

- [Booking.com Hotel Details Scraper](https://apify.com/thescrappa/booking-hotel-details-scraper)
- [Booking.com Search Scraper](https://apify.com/thescrappa/booking-search-scraper)
- [Google Flights Scraper](https://apify.com/thescrappa/google-flights-search-scraper)
- [Google Hotels Autocomplete Scraper](https://apify.com/thescrappa/google-hotels-autocomplete-scraper)
- [Google Maps Directions Scraper](https://apify.com/thescrappa/google-maps-directions-scraper)
