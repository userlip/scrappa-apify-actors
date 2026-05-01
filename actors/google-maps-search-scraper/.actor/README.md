# Google Maps Search

Search for businesses on Google Maps at scale. This actor returns Scrappa Google Maps search records with business identity, contact, location, rating, category, photo, status, and opening-hours fields.

## Features

- Search Google Maps by natural-language query.
- Return each business as a separate Apify dataset item.
- Include rich fields such as `full_address`, coordinates, `business_id`, `place_id`, `phone_numbers`, `photos_sample`, and `opening_hours` when Google exposes them.
- Save the complete Scrappa API response to the `OUTPUT` key-value store record.
- Use cached Scrappa responses by default for faster repeat runs and lower cost.
- Retry through advanced search with a configurable zoom level after transient simple-search upstream failures.

## Input

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `query` | string | Yes | Search query, such as `restaurants in NYC`, `coffee shops`, or `plumber near Austin`. |
| `hl` | string | No | Two-letter language code, optionally with a two-letter region, such as `en`, `de`, or `en-US`. Defaults to `en`. |
| `gl` | string | No | Region code in ISO 3166-1 alpha-2 format, such as `us`, `de`, or `uk`. |
| `debug` | boolean | No | Enable Scrappa debug output for troubleshooting. Useful only for accounts with debug access. Defaults to `false`. |
| `use_cache` | boolean | No | Use cached data when available. Defaults to `true`. |
| `maximum_cache_age` | integer | No | Maximum allowed cache age in seconds. Defaults to `3600`. Set to `0` to request fresh data when cache is enabled. |
| `fallback_zoom` | integer | No | Zoom level used when retrying via advanced search after a transient simple-search failure. Defaults to `13`. |

## Example Input

```json
{
  "query": "pizza restaurants in Manhattan",
  "hl": "en",
  "gl": "us",
  "use_cache": true,
  "maximum_cache_age": 3600,
  "fallback_zoom": 13
}
```

## Output

The actor pushes every item from the Scrappa `items` response array into the default dataset. Fields vary by business, but records can include:

- `name`
- `price_level`
- `price_level_text`
- `review_count`
- `rating`
- `website`
- `domain`
- `latitude`
- `longitude`
- `business_id`
- `subtypes`
- `district`
- `full_address`
- `address`
- `timezone`
- `short_description`
- `full_description`
- `owner_id`
- `owner_name`
- `owner_link`
- `order_link`
- `google_mid`
- `type`
- `phone_numbers`
- `phone`
- `place_id`
- `photos_sample`
- `opening_hours`
- `current_status`

`address` and `phone` are dataset-friendly aliases added by the actor from `full_address` and `phone_numbers`. The original Scrappa fields are preserved.

## Example Dataset Item

```json
{
  "name": "Example Pizza",
  "type": "Pizza restaurant",
  "rating": 4.5,
  "review_count": 1250,
  "full_address": "123 Main St, New York, NY 10001",
  "address": "123 Main St, New York, NY 10001",
  "latitude": 40.7501,
  "longitude": -73.997,
  "phone_numbers": ["+1 212-555-0100"],
  "phone": "+1 212-555-0100",
  "website": "https://example.com",
  "business_id": "0x89c259af336b3341:0x1234567890abcdef",
  "place_id": "ChIJExamplePlaceId",
  "timezone": "America/New_York",
  "current_status": "Open"
}
```

## Nested Fields

`photos_sample` contains sample photo objects when available, including fields such as `photo_id`, `photo_url`, `photo_url_large`, `video_thumbnail_url`, `latitude`, `longitude`, and `type`.

`opening_hours` contains day-level hours objects when available, including fields such as `day`, `hours`, `date`, and `special_day`.

## Pricing

$0.30 per 1,000 results. No Google Maps API key required.
