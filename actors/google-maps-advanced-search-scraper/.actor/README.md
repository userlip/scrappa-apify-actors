# Google Maps Advanced Search Scraper

Scrape Google Maps business listings from a precise map area using a search query, coordinates, and zoom level. This actor is built for local lead generation, competitor monitoring, territory research, and workflows where you need results tied to a specific latitude, longitude, and map zoom instead of a broad city-level search.

Use it when you need Google Maps results for a tight geographic area, a neighborhood, a sales territory, or a coordinate-defined map viewport.

## What It Does

- Searches Google Maps for businesses by keyword, category, or service.
- Targets results with optional `latitude` and `longitude` center coordinates.
- Controls local precision with Google Maps-style `zoom` levels from broad area to street-level focus.
- Supports result limits for quick samples or larger lead lists.
- Returns business names, ratings, reviews, addresses, websites, phones, coordinates, business types, hours, status, and sample photos when available.
- Supports language and region targeting with `hl` and `gl`.

## Common Use Cases

- Find "restaurants", "coffee shops", "dentists", "plumbers", or other businesses around exact coordinates.
- Scrape Google Maps leads inside a neighborhood, shopping district, downtown area, or delivery zone.
- Compare competitors around store locations, franchise territories, hotels, campuses, or event venues.
- Build coordinate-based local business datasets for sales prospecting, market mapping, and location intelligence.
- Run bounds-focused Google Maps research by choosing a center point and zoom level for each target area.

## Input

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `query` | string | Yes | Business, category, or service to search for, such as `coffee shops`, `restaurants`, or `dentists`. |
| `zoom` | integer | Yes | Map zoom level from `3` to `21`. Lower values cover a wider area; higher values target a smaller, more precise area. |
| `latitude` | number | No | Center latitude for the search area. If omitted, location may be resolved from the query. |
| `longitude` | number | No | Center longitude for the search area. If omitted, location may be resolved from the query. |
| `limit` | integer | No | Maximum number of results to return. |
| `hl` | string | No | Google language code, such as `en`, `de`, `es`, or `fr`. Defaults to `en`. |
| `gl` | string | No | Google region code, such as `us`, `de`, `fr`, or `uk`. |

## Example Input

```json
{
  "query": "coffee shops",
  "latitude": 40.758,
  "longitude": -73.9855,
  "zoom": 15,
  "limit": 50,
  "hl": "en",
  "gl": "us"
}
```

## Output

The actor saves matching businesses to the default dataset. Each result can include:

- `name`
- `business_id`
- `place_id`
- `rating`
- `review_count`
- `price_level`
- `website`
- `domain`
- `phone_numbers`
- `full_address`
- `district`
- `latitude`
- `longitude`
- `subtypes`
- `type`
- `short_description`
- `opening_hours`
- `current_status`
- `photos_sample`

## Example Output

```json
{
  "name": "Example Coffee",
  "business_id": "0x89c259...",
  "place_id": "ChIJ...",
  "rating": 4.6,
  "review_count": 842,
  "website": "https://example.com",
  "phone_numbers": ["+1 212-555-0100"],
  "full_address": "123 Example Ave, New York, NY 10036",
  "latitude": 40.7581,
  "longitude": -73.9856,
  "subtypes": ["Coffee shop"],
  "current_status": "Open"
}
```

## Tips For Coordinate And Bounds-Based Searches

- Use `latitude` and `longitude` when you need results around an exact point.
- Use higher zoom levels, such as `15` to `18`, for neighborhood or street-level searches.
- Use lower zoom levels, such as `10` to `14`, for city or metro-area discovery.
- For grid or bounds workflows, split your target region into center points and run the actor once per coordinate with the zoom level that matches your desired coverage.
- Include location words in `query` only when you want Google to interpret a named place; otherwise use coordinates for cleaner area targeting.

## Pricing

$0.30 per 1,000 results. No Google Maps API key required.
