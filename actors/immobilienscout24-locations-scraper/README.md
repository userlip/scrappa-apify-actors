# ImmobilienScout24 Location Autocomplete

Thin Apify marketplace wrapper for Scrappa's
`GET /api/immobilienscout24/locations` endpoint. All scraping and location
resolution stays on Scrappa infrastructure.

The Actor accepts batch-first `queries`, singular `query` compatibility, and a
per-query `limit` from 1–20. It normalizes and deduplicates queries, continues
after individual query failures, and writes one charged dataset row per unique
geocode with its first `source_query`.

See [.actor/README.md](.actor/README.md) for marketplace input/output examples.
