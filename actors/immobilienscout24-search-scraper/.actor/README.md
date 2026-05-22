# ImmobilienScout24 Search Scraper

Scrape ImmobilienScout24 property search listings by location, search type,
price, rooms, and floor area. The actor saves one dataset item per listing and
stores the trimmed Scrappa response in key-value store key `OUTPUT`.

Recommended paid pricing: **$0.30 per 1,000 saved property listings** using the
`property-result` pay-per-event charge.

## Example input

```json
{
  "location": "1276003001",
  "type": "apartment-rent",
  "price_max": 1500,
  "rooms_min": 2,
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
- `request_price_min`
- `request_price_max`
- `request_rooms_min`
- `request_rooms_max`
- `request_size_min`
- `request_size_max`
- `request_page`
- `request_per_page`

Use `page`, `total_pages`, and `total_results` from the full `OUTPUT` object to continue pagination.
