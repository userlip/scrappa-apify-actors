# ImmobilienScout24 Location Autocomplete

Thin Apify marketplace wrapper for Scrappa's
`GET /api/immobilienscout24/locations` endpoint. All scraping and location
resolution stays on Scrappa infrastructure.

The Actor accepts batch-first `queries`, singular `query` compatibility, and a
per-query `limit` from 1–20. It normalizes and deduplicates queries, continues
after individual query failures, and writes one charged dataset row per unique
geocode with its first `source_query`.

If the live Scrappa endpoint has a retryable outage, the default Berlin query
can use a small verified cache. Cached rows are explicitly marked with
`is_cached: true`; unsupported queries continue to fail instead of returning an
unrelated location.

Requests run in ordered batches of 10. If the Apify charge limit is reached,
dataset writes stop immediately; up to nine already-started Scrappa requests in
the current batch may still complete without producing charged output.

See [.actor/README.md](.actor/README.md) for marketplace input/output examples.
