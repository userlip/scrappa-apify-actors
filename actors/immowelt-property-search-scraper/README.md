# Immowelt Property Search Scraper

Scrape Immowelt property search results through Scrappa. The actor returns one dataset item per property listing and writes the full Scrappa response to the default key-value store as `OUTPUT`.

## Monetization

This actor is built for paid pay-per-event pricing with the `property-result` event. Recommended launch pricing is **$0.30 per 1,000 saved property listings** so users pay only for listing rows written to the dataset.

## What it extracts

- Listing title, Immowelt listing ID, online ID, expose URL, image URL, and published timestamp
- Price, formatted price text, room counts, and floor area in square meters
- Address, latitude, longitude, and private-seller flag
- Request context for location, property type, page, and page size

## Example input

```json
{
  "location": "Berlin",
  "property_type": "apartment",
  "page": 1,
  "limit": 20
}
```

## Pagination

The Scrappa response includes `page`, `total_pages`, and `total_results`. Run the actor again with the same `location` and `property_type` plus the next `page` value to continue through the result set.

## Output

Dataset items contain the full property object plus normalized top-level fields for common analysis:

```json
{
  "id": "estate_6c21f567-4a32-4955-b755-550cab4d635e",
  "online_id": "2paau5t",
  "title": "Moderne 3-Zimmerwohnung in begehrter Lage Berlin-Mitte",
  "price": 2170.82,
  "price_formatted": "2.171 EUR (Kaltmiete)",
  "rooms": 3.5,
  "size_m2": 113.81,
  "address": "Mitte, 10117, Berlin",
  "latitude": 52.511009,
  "longitude": 13.402116,
  "url": "https://www.immowelt.de/expose/2paau5t",
  "image_url": "https://ms.immowelt.org/example/original.jpg",
  "is_private": false,
  "published": "2026-05-11T19:02:59.797Z",
  "request_location": "Berlin",
  "request_property_type": "apartment",
  "request_page": 1,
  "request_limit": 20
}
```

For higher-volume Immowelt or real estate market research use cases, use Scrappa directly at `https://scrappa.co`.
