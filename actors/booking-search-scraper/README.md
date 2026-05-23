# Booking.com Search Scraper

Scrape Booking.com search results through Scrappa. The actor returns one dataset item per property card and supports batching multiple destinations or date combinations in a single Apify run.

## What it extracts

- Property name, Booking.com URL, image, location, and price fields
- Review score, review label, and review count when Booking.com renders them
- Request context for destination, check-in/check-out dates, adults, children, rooms, language, currency, and batch search index

## Example input

This dated Paris request is the recommended smoke-test shape. Replace the dates with real future dates when you need a different stay window.

```json
{
  "ss": "Paris",
  "checkin": "2026-07-01",
  "checkout": "2026-07-03",
  "group_adults": 2,
  "group_children": 0,
  "no_rooms": 1,
  "lang": "en-us",
  "currency": "EUR"
}
```

## Batch input

Use `searches` to run multiple Booking.com searches in one actor run. The actor calls Scrappa once per search and still pushes one dataset item per returned property.

Each batch item should include real future dates for best result-card coverage.

```json
{
  "searches": [
    {
      "ss": "Paris",
      "checkin": "2026-07-01",
      "checkout": "2026-07-03",
      "group_adults": 2,
      "no_rooms": 1,
      "currency": "EUR"
    },
    {
      "ss": "Berlin",
      "checkin": "2026-07-10",
      "checkout": "2026-07-12",
      "group_adults": 1,
      "no_rooms": 1,
      "currency": "EUR"
    }
  ]
}
```

## Notes

Booking.com often needs `checkin` and `checkout` dates to render property cards. Destination-only searches may return fewer fields or no property results.

## Output

Dataset items contain the full Scrappa property card plus normalized top-level fields for common analysis:

```json
{
  "name": "Hotel Example Paris",
  "url": "https://www.booking.com/hotel/fr/example.html",
  "image": "https://cf.bstatic.com/example.jpg",
  "review_score": 8.7,
  "review_score_word": "Excellent",
  "review_count": 1240,
  "location": "1st arr., Paris",
  "price": "EUR 420",
  "currency": "EUR",
  "request_search_index": 0,
  "request_ss": "Paris",
  "request_checkin": "2026-07-01",
  "request_checkout": "2026-07-03"
}
```

For higher-volume Booking.com travel data use cases, use Scrappa directly at `https://scrappa.co`.
