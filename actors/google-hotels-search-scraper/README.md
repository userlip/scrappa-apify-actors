# Google Hotels Search Scraper

Scrape Google Hotels search results through Scrappa. The actor returns one dataset item per hotel or vacation rental property and writes the full Scrappa response to the default key-value store as `OUTPUT`.

## What it extracts

- Hotel name, class, rating, review count, thumbnail, booking link, and property token
- Nightly and total rates, including extracted numeric price fields when available
- GPS coordinates, place IDs, entity IDs, price source count, and amenity count
- Request context for destination, check-in/check-out dates, guests, currency, country, language, pagination token, and property token

## Example input

```json
{
  "q": "Paris, France",
  "check_in_date": "2026-06-15",
  "check_out_date": "2026-06-18",
  "adults": 2,
  "currency": "EUR",
  "gl": "fr",
  "hl": "en",
  "sort_by": 3,
  "hotel_class": 4,
  "rating": 8
}
```

## Pagination

If the Scrappa response includes `pagination.next_page_token`, run the actor again with the same search parameters plus `next_page_token`.

## Filters

Google Hotels supports several filter families, but some cannot be combined upstream. This actor validates those constraints before calling Scrappa:

- `free_cancellation`, `eco_certified`, and `special_offers` cannot be combined with price, class, rating, amenity, property type, or brand filters.
- `bedrooms` and `bathrooms` are only available when `vacation_rentals` is `true`.
- `hotel_class` and `brands` are not available for vacation rental searches.

## Output

Dataset items contain the full property object plus normalized top-level fields for common analysis:

```json
{
  "name": "Hotel Le Bristol Paris",
  "hotel_class": "5-star hotel",
  "overall_rating": 4.8,
  "reviews": 2150,
  "rate_per_night_lowest": "€850",
  "rate_per_night_extracted_lowest": 850,
  "total_rate_lowest": "€1,700",
  "total_rate_extracted_lowest": 1700,
  "booking_link": "https://www.booking.com/hotel/fr/example.html",
  "property_token": "CgoIyNaqqL33x5ovEAE",
  "latitude": 48.8711,
  "longitude": 2.3146,
  "request_q": "Paris, France",
  "request_check_in_date": "2026-06-15",
  "request_check_out_date": "2026-06-18"
}
```

For higher-volume Google Hotels or travel market research use cases, use Scrappa directly at `https://scrappa.co`.
