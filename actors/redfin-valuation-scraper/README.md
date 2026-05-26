# Redfin Valuation Scraper

Get Redfin home value estimates, low/high value ranges, last sale context,
property specs, and comparable sales by Redfin property ID. The actor is a thin
Apify wrapper around Scrappa's Redfin valuation API and saves one dataset item
per successful valuation.

Recommended paid pricing: **$0.50 per 1,000 saved valuation results** using the
`valuation-result` pay-per-event charge.

This actor is Redfin-backed through Scrappa. It does not scrape or claim Zillow
valuation coverage.

## Example input

```json
{
  "property_id": 194191988
}
```

## Batch input

Use `property_ids` to value multiple properties in one Apify run:

```json
{
  "property_ids": [194191988, 123456789]
}
```

Use `properties` when each property needs its own optional listing ID or when
you want the actor to extract IDs from Redfin URLs:

```json
{
  "properties": [
    { "property_id": 194191988 },
    { "property_id": 123456789, "listing_id": 207388793 },
    { "url": "https://www.redfin.com/home/194191988" }
  ]
}
```

## Output fields

Common top-level fields include:

- `property_id`
- `listing_id`
- `predicted_value`
- `predicted_value_low`
- `predicted_value_high`
- `last_sold_price`
- `last_sold_date`
- `beds`
- `baths`
- `sqft`
- `lot_size`
- `year_built`
- `comparables_count`
- `comparables`
- `request_index`
- `request_property_id`
- `request_listing_id`
- `request_url`
