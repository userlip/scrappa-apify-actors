# ImmobilienScout24 Location Autocomplete

Thin Apify marketplace wrapper for Scrappa's
`GET /api/immobilienscout24/locations` endpoint. All scraping and location
resolution stays on Scrappa infrastructure.

The Actor accepts batch-first `queries`, singular `query` compatibility, and a
per-query `limit` from 1–20. It normalizes and deduplicates queries, continues
after individual query failures, and writes one charged dataset row per unique
geocode with its first `source_query`.

Requests run in ordered batches of 10. If the Apify charge limit is reached,
dataset writes stop immediately; up to nine already-started Scrappa requests in
the current batch may still complete without producing charged output.

See [.actor/README.md](.actor/README.md) for marketplace input/output examples.
