# ImmobilienScout24 Price Insights Scraper

Compare German property markets with current apartment and house rent and purchase benchmarks per square meter. Use it for property-market benchmarking, rent-vs-buy research, and recurring city comparisons.

The Actor is a thin wrapper around Scrappa's `immobilienscout24-price-insights` endpoint. Scraping runs on Scrappa infrastructure. Batch up to 100 locations in one Actor run; each successfully resolved location produces one dataset item.

## Input

```json
{
  "locations": ["Berlin", "Munich", "Hamburg"]
}
```

`locations` also accepts a comma-separated string. The singular `location` field remains available for compatibility. Values are trimmed and deduplicated case-insensitively.

## Output

```json
{
  "location": "Berlin",
  "geocode": "1276003001",
  "currency": "EUR",
  "apartment_rent_per_m2": 12.72,
  "apartment_buy_per_m2": 4189.04,
  "house_rent_per_m2": 16.51,
  "house_buy_per_m2": 4394.87,
  "request_location": "Berlin",
  "request_index": 0
}
```

Unavailable or invalid locations are logged and skipped while the rest of the batch continues. Failed locations create no dataset item and no charge.

## Pricing

The Actor uses Apify pay-per-event pricing with `price-insight-result` at **$0.0005 per successful location snapshot** ($0.50 per 1,000). Only successfully saved snapshots are charged.

Need higher-volume recurring comparisons or direct integration? Use the [Scrappa API](https://scrappa.co) directly.
