# Immowelt Property Search Scraper

Scrape Immowelt property listings by location and search type. The actor saves
one dataset item per listing and stores the complete Scrappa response in
key-value store key `OUTPUT`.

Recommended paid pricing: **$0.30 per 1,000 saved property listings** using the
`property-result` pay-per-event charge.

## Example input

```json
{
  "location": "Berlin",
  "type": "apartment-rent",
  "page": 1,
  "per_page": 20
}
```

## Output fields

Common top-level fields include:

- `title`
- `price`
- `price_formatted`
- `rooms`
- `rooms_max`
- `size_m2`
- `size_m2_max`
- `address`
- `latitude`
- `longitude`
- `url`
- `online_id`
- `id`
- `image_url`
- `is_private`
- `published`
- `request_location`
- `request_type`
- `request_page`
- `request_per_page`

Use `page`, `total_pages`, and `total_results` from the full `OUTPUT` object to continue pagination.
