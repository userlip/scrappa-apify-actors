# Google Hotels Search Scraper

Scrape Google Hotels search results with dates, guests, price filters, star class, ratings, amenities, vacation rental filters, booking links, and pagination.

The actor saves one dataset item per property and stores the complete Scrappa response in key-value store key `OUTPUT`.

## Example input

Replace the date values with future dates before running the actor.

```json
{
  "q": "Paris, France",
  "check_in_date": "YYYY-MM-DD",
  "check_out_date": "YYYY-MM-DD",
  "adults": 2,
  "currency": "EUR",
  "gl": "fr",
  "hl": "en",
  "sort_by": "3",
  "hotel_class": "4",
  "rating": "8"
}
```

## Output fields

Common top-level fields include:

- `name`
- `hotel_class`
- `overall_rating`
- `reviews`
- `rate_per_night_lowest`
- `rate_per_night_extracted_lowest`
- `total_rate_lowest`
- `total_rate_extracted_lowest`
- `booking_link`
- `property_token`
- `entity_id`
- `place_id`
- `latitude`
- `longitude`
- `price_sources_count`
- `amenities_count`
- `request_q`
- `request_check_in_date`
- `request_check_out_date`

Use `next_page_token` from the full `OUTPUT.pagination` object to fetch additional result pages.
