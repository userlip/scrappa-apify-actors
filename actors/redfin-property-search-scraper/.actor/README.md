# Redfin Property Search Scraper

Scrape Redfin property search results by region, market, price, beds, baths,
property type, status, and page. The actor is a thin Apify wrapper around
Scrappa's Redfin API and saves one dataset item per property listing.

Recommended paid pricing: **$0.30 per 1,000 saved property listings** using the
`property-result` pay-per-event charge.

## Example input

```json
{
  "region_id": 16163,
  "region_type": 6,
  "market": "seattle",
  "num_homes": 50,
  "page": 1
}
```

## Batch input

Use `searches` to run multiple Redfin searches in one Apify run:

```json
{
  "searches": [
    { "region_id": 16163, "region_type": 6, "market": "seattle", "num_homes": 25 },
    { "region_id": 11203, "region_type": 6, "market": "socal", "num_homes": 25 }
  ]
}
```

## Output fields

Common top-level fields include:

- `property_id`
- `listing_id`
- `address`
- `city`
- `state`
- `zip`
- `price`
- `beds`
- `baths`
- `sqft`
- `lot_size`
- `year_built`
- `property_type`
- `property_type_label`
- `status`
- `latitude`
- `longitude`
- `url`
- `mls_number`
- `request_search_index`
- `request_region_id`
- `request_region_type`
- `request_market`
- `request_min_price`
- `request_max_price`
- `request_num_beds`
- `request_num_baths`
- `request_property_types`
- `request_status`
- `request_status_label`
- `request_sold_within_days`
- `request_num_homes`
- `request_page`
