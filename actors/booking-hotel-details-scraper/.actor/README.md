# Booking.com Hotel Details Scraper

Scrape Booking.com property-page hotel details through Scrappa. The actor accepts Booking.com hotel URLs or country/slug pairs, supports batches in one Apify run, and returns one dataset item per hotel detail result.

## What it extracts

- Hotel title and canonical Booking.com URL
- `hotel_schema`, `aggregate_rating`, `json_ld`, Open Graph, Twitter, H1, page type, redirect, and parsed metadata returned by Scrappa
- Request metadata for input type, URL, country, slug, batch index, success status, and per-item errors

## URL input

Use a direct Booking.com hotel URL when you already have the property page.

```json
{
  "url": "https://www.booking.com/hotel/fr/ritz-paris.html"
}
```

## Country and slug input

Use `country` plus `slug` when you have the Booking.com URL parts. The `.html` suffix on `slug` is optional.

```json
{
  "country": "fr",
  "slug": "ritz-paris"
}
```

## Batch input

Use `urls` or `hotels` to process multiple hotel pages in one actor run. The actor calls Scrappa once per hotel and pushes one dataset item per successful hotel detail response. Failed hotel requests are saved as uncharged error rows.

```json
{
  "hotels": [
    {
      "url": "https://www.booking.com/hotel/fr/ritz-paris.html"
    },
    {
      "country": "de",
      "slug": "sample.html"
    }
  ]
}
```

## Output

Dataset items contain the full Scrappa hotel detail payload plus normalized request fields:

```json
{
  "title": "Ritz Paris, Paris",
  "canonical_url": "https://www.booking.com/hotel/fr/ritz-paris.html",
  "hotel_schema": {
    "@context": "https://schema.org",
    "@type": "Hotel",
    "name": "Ritz Paris"
  },
  "aggregate_rating": {
    "@type": "AggregateRating",
    "ratingValue": "9.4",
    "reviewCount": "512"
  },
  "json_ld": [],
  "parsed": true,
  "request_index": 0,
  "request_input_type": "url",
  "request_url": "https://www.booking.com/hotel/fr/ritz-paris.html",
  "request_country": null,
  "request_slug": null,
  "request_success": true
}
```

For higher-volume Booking.com hotel detail extraction, use the Scrappa API directly at `https://scrappa.co`.
