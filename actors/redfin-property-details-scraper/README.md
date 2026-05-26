# Redfin Property Details Scraper

Scrape detailed Redfin property records by property ID or Redfin URL. The actor
is a thin Apify wrapper around Scrappa's Redfin Property API and saves one
dataset item per property, while all scraping work stays on Scrappa
infrastructure.

Recommended paid pricing: **$0.30 per 1,000 saved property detail results**
using the `property-result` pay-per-event charge.

## Example input

```json
{
  "property_id": 60791456
}
```

## URL input

```json
{
  "url": "https://www.redfin.com/TN/Memphis/1549-Ely-St-38106/home/60791456"
}
```

## Batch input

Use `property_ids` or `urls` to fetch multiple property records in one Apify run:

```json
{
  "property_ids": [60791456, 194191988],
  "urls": [
    "https://www.redfin.com/TN/Memphis/1549-Ely-St-38106/home/60791456"
  ]
}
```

Duplicate property IDs are processed once per run.

## Output fields

Common top-level fields include:

- `property_id`
- `address`
- `city`
- `state`
- `zip`
- `country`
- `price`
- `price_label`
- `beds`
- `baths`
- `sqft`
- `lot_size`
- `year_built`
- `property_type`
- `status`
- `status_label`
- `latitude`
- `longitude`
- `url`
- `photos`
- `description`
- `request_property_index`
- `request_property_id`
- `request_input`
- `request_source`

For single-property runs, `OUTPUT` contains the property detail item. For batch
runs, `OUTPUT` contains a short summary and the dataset remains the primary
result channel.
