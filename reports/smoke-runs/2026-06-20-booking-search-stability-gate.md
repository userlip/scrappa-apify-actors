# Booking Search stability gate - 2026-06-20

Scope: run a narrow post-recovery stability gate for public paid actor `booking-search-scraper` (`BehWN3LEvBxhEiJDF`) before clearing its Apify `UNDER_MAINTENANCE` notice.

## Executive summary

- `booking-search-scraper` passed three consecutive fresh Apify runs with the dated Paris input requested for the gate.
- All three runs used build `NR534OiXB7Rb8wB3r`, finished `SUCCEEDED`, and wrote 25 Booking.com dataset items each.
- Run logs contained the actor success line and no Scrappa API 502/503 or uncaught exception evidence.
- After the gate passed, the actor-level notice was cleared with `{"notice":"NONE"}`.
- The follow-up health audit reports `booking-search-scraper` as `OK`, with recent statuses `SUCCEEDED, SUCCEEDED, SUCCEEDED, SUCCEEDED, SUCCEEDED`, and `noticeActors: []`.

## Gate input

```json
{
  "ss": "Paris",
  "checkin": "2026-07-01",
  "checkout": "2026-07-03",
  "group_adults": 2,
  "group_children": 0,
  "no_rooms": 1,
  "lang": "en-us",
  "currency": "EUR"
}
```

## Run evidence

| Gate run | Run ID | Status | Started at | Finished at | Build ID | Dataset ID | Dataset items | Log evidence |
| --- | --- | --- | --- | --- | --- | --- | ---: | --- |
| 1 | `wEsGnNBdvWfrz87CX` | `SUCCEEDED` | `2026-06-20T21:17:17.218Z` | `2026-06-20T21:17:26.141Z` | `NR534OiXB7Rb8wB3r` | `tm0TTcdPoMQ4BMf0F` | 25 | `Booking.com search completed successfully`; no 502/503 or uncaught exception text |
| 2 | `m8HpF9qfOg7LqZKNw` | `SUCCEEDED` | `2026-06-20T21:17:30.609Z` | `2026-06-20T21:17:38.166Z` | `NR534OiXB7Rb8wB3r` | `ZKh0cQk50NQIpCnDF` | 25 | `Booking.com search completed successfully`; no 502/503 or uncaught exception text |
| 3 | `YDwODp0Ukr6aeQshS` | `SUCCEEDED` | `2026-06-20T21:17:43.917Z` | `2026-06-20T21:17:47.050Z` | `NR534OiXB7Rb8wB3r` | `dNcIKhTzINnkawnk5` | 25 | `Booking.com search completed successfully`; no 502/503 or uncaught exception text |

Dataset spot checks returned usable Booking.com result rows for the requested Paris dates, including property name, Booking.com URL, image URL, price, currency, and request echo fields.

## Notice change

- Before: `GET /v2/acts/BehWN3LEvBxhEiJDF` returned `notice: "UNDER_MAINTENANCE"`.
- Change: `PUT /v2/actors/BehWN3LEvBxhEiJDF` with body `{"notice":"NONE"}`.
- After: Apify actor detail returned `notice: "NONE"` and `modifiedAt: "2026-06-20T21:18:19.533Z"`.

## Monetization and health

| Actor ID | Slug | Pricing status | Latest statuses | Latest build | Env var status | Final notice |
| --- | --- | --- | --- | --- | --- | --- |
| `BehWN3LEvBxhEiJDF` | `booking-search-scraper` | Paid `PAY_PER_EVENT`, `booking-result` at `0.0002` USD/result, started `2026-06-03T10:06:39.118Z` | `SUCCEEDED, SUCCEEDED, SUCCEEDED, SUCCEEDED, SUCCEEDED` | `NR534OiXB7Rb8wB3r` / build number `1.0.4`, `SUCCEEDED` | `SCRAPPA_API_KEY` present as a secret on version `1.0` | Cleared; health audit reports `notice: null` |

Pricing API note: actor detail still returned `pricingInfo: null` and `currentPricingInfo: null`, while `pricingInfos` contained the active past-started paid event pricing. The repository pricing audit reported `overdueMissingActivePricing: 0`, `missingPaidPricing: 0`, and did not list Booking Search as a pricing issue.

## Validation commands

- `GET /v2/acts/BehWN3LEvBxhEiJDF` for actor metadata, notice, pricing, and default run options.
- `GET /v2/acts/BehWN3LEvBxhEiJDF/versions/1.0` for `SCRAPPA_API_KEY` secret presence.
- `POST /v2/acts/BehWN3LEvBxhEiJDF/runs` three times with the gate input.
- `GET /v2/actor-runs/{runId}` for status/build/dataset IDs.
- `GET /v2/actor-runs/{runId}/log` for success/error log checks.
- `GET /v2/datasets/{datasetId}` and `GET /v2/datasets/{datasetId}/items?clean=true&limit=1` for dataset counts and sample row validation.
- `APIFY_TOKEN=... node scripts/audit-apify-health.mjs --json`.
- `APIFY_TOKEN=... node scripts/audit-apify-pricing.mjs --json`.

## Final audit result

`node scripts/audit-apify-health.mjs --json` at `2026-06-20T21:18:27.919Z` reported:

| Metric | Value |
| --- | ---: |
| Public TheScrappa actors | 82 |
| OK | 35 |
| No runs | 46 |
| Failed latest runs | 0 |
| Failed latest builds | 0 |
| Recent failed but latest OK | 1 |
| Notices | 0 |
| Audit errors | 0 |

`booking-search-scraper` final actor report:

- Latest run: `YDwODp0Ukr6aeQshS`, `SUCCEEDED`.
- Recent statuses: `SUCCEEDED, SUCCEEDED, SUCCEEDED, SUCCEEDED, SUCCEEDED`.
- Latest build: `NR534OiXB7Rb8wB3r`, `SUCCEEDED`.
- Notice: `null`.
- Status: `OK`.

Remaining non-OK health item is unrelated to this gate: `google-videos-scraper` (`kAdTwn5fkBCGKOQUq`) remains `RECENT_FAILED_BUT_LATEST_OK` with no notice.
