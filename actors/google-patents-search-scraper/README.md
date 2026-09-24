# Google Patents Search Scraper

Search Google Patents through Scrappa and export structured patent results to an Apify dataset. Use it for prior-art research, IP monitoring, patent landscaping, assignee tracking, inventor research, competitive intelligence, and date-filtered patent discovery.

## Pricing

This actor is intended for paid per-result monetization on Apify's default dataset-item event (`apify-default-dataset-item`). Recommended marketplace pricing is **$0.20 per 1,000 dataset items** so users pay for successful patent results, not empty runs.

## Input

```json
{
  "q": "wireless charging vehicle battery",
  "page": 1,
  "num": 10,
  "sort": "new",
  "country": "US,EP",
  "status": "GRANT",
  "type": "PATENT",
  "after": "filing:20200101",
  "assignee": "Tesla,Toyota"
}
```

### Fields

- `q` - Required Google Patents query.
- `page` - One-based result page, 1 to 100.
- `num` - Results per page, 1 to 100.
- `sort` - `new` or `old`; omit for relevance.
- `country` - Comma-separated country codes such as `US,EP,WO`.
- `language` - Google Patents language filter such as `ENGLISH`.
- `status` - `GRANT` or `APPLICATION`.
- `type` - `PATENT` or `DESIGN`.
- `before` / `after` - Date filters in `filing:YYYYMMDD` or `publication:YYYYMMDD` format.
- `inventor` - Comma-separated inventor names.
- `assignee` - Comma-separated assignee or company names.

## Output

Each dataset item is one patent result with flattened fields for easier exports:

```json
{
  "rank": 1,
  "title": "Wireless charging system",
  "patent_id": "patent/US1234567B1/en",
  "publication_number": "US1234567B1",
  "patent_page": "https://patents.google.com/patent/US1234567B1",
  "assignee": "Example Inc.",
  "inventor": "Jane Doe",
  "priority_date": "2020-01-01",
  "filing_date": "2021-01-01",
  "grant_date": "2024-01-01",
  "publication_date": "2022-01-01",
  "pdf": "https://patentimages.storage.googleapis.com/...",
  "family_countries": "US,EP",
  "family_status_count": 2,
  "request_q": "wireless charging vehicle battery",
  "request_country": "US,EP",
  "request_status": "GRANT"
}
```

The full Scrappa response is also stored in the key-value store as `OUTPUT`.

Before writing results, the actor reads the run's PPE prices, charged event counts, and `maxTotalChargeUsd`, then saves only the affordable prefix of result rows. Apify's default dataset item event charges each row written to the default dataset. If the budget is reached, `OUTPUT` contains only the saved patent prefix and the run receives a charge-limit status message. Scrappa requests have a 60-second deadline and retry up to three times for timeouts and transient HTTP statuses (`408`, `429`, `500`, `502`, `503`, `504`).

## Development

This actor uses Rust 1.90. Run its focused test suite from this directory:

```bash
cargo test --locked
```

## Run locally

The actor reads `INPUT` from its default key-value store, calls Scrappa, and writes dataset items and `OUTPUT` through the Apify API:

```bash
APIFY_TOKEN=... \
ACTOR_RUN_ID=... \
ACTOR_DEFAULT_KEY_VALUE_STORE_ID=... \
ACTOR_DEFAULT_DATASET_ID=... \
SCRAPPA_API_KEY=... \
cargo run --locked
```

For local mocks, `APIFY_API_PUBLIC_BASE_URL` and `SCRAPPA_API_BASE_URL` can override the production API bases.

The actor preserves Scrappa's original patent result fields and adds the flattened aliases shown above, so new upstream fields may appear in exports without an actor code change.

## Notes

- Google Patents filtering is exacting. If a run returns no results, loosen the assignee, inventor, country, or date filters first.
- For recurring IP monitoring, schedule the actor with a stable query and sort by `new`.
- For higher-volume direct API usage, use Scrappa's Google Patents API.
