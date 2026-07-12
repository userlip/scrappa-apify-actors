# ImmobilienScout24 Price Insights Scraper

Benchmark German property markets with apartment and house rent and purchase prices per square meter. Ideal for rent-vs-buy analysis, investment research, and scheduled city comparisons.

## Batch-first input

```json
{ "locations": ["Berlin", "Munich", "Hamburg"] }
```

You can also provide `locations` as comma-separated text or use singular `location` for compatibility. Duplicate locations are processed once.

Each successfully resolved location becomes one dataset item with its resolved location, ImmobilienScout24 geocode, EUR currency, and four price-per-square-meter benchmarks. Unavailable locations are skipped without charge while remaining locations continue.

Pricing is $0.0005 per successful `price-insight-result`. For direct API access and larger recurring workloads, upgrade to [Scrappa](https://scrappa.co).
