# Implementation Summary: Stale Apify Notice Smoke Gate

Date: 2026-07-10

## Outcome

Completed one controlled production smoke run per approved Actor. All three runs passed the defined gate, so the targeted Apify `UNDER_MAINTENANCE` notices were cleared by updating only each Actor's `notice` field to `NONE`. No application source, builds, deployments, pricing, or secrets were changed.

| Actor | ID | Latest build (confirmed) | Smoke run | Result evidence | Effective paid pricing and accounting | Notice result |
| --- | --- | --- | --- | --- | --- | --- |
| Booking.com Search Scraper | `BehWN3LEvBxhEiJDF` | `NR534OiXB7Rb8wB3r` / `SUCCEEDED` | `vgKnULJIsxREWa7Z7` / `SUCCEEDED` | Default dataset readable with 26 rows. First-row fields include `name`, `url`, `currency`, `request_search_index`, `request_ss`, `request_checkin`, and `request_checkout`. Log: `Search 1 returned 26 Booking.com result(s)` and `Booking.com search completed successfully`; no failure/error lines. | `PAY_PER_EVENT`, active since `2026-06-03T10:06:39.118Z`; `booking-result` at USD `0.0002` each. Run reports `booking-result: 26`; run-level event charge amount is not separately returned by Apify. | `UNDER_MAINTENANCE` -> `NONE`; refreshed history has this run first. |
| Google Maps Advanced Search | `DT8bUdm2Vn4HjlyDo` | `agH9oDbqHRNZMKhIb` / `SUCCEEDED` | `JVEjCoJ01QMfgzL0v` / `SUCCEEDED` | Default dataset readable with one row, including `name`, `rating`, `review_count`, `full_address`, `phone_numbers`, `website`, `latitude`, and `longitude`. `OUTPUT` is readable with `items`. Log: `Found 1 results` and normal completion summary; no failure/error lines. | Live authoritative model is `PAY_PER_EVENT`, active since `2026-03-15T23:00:00.000Z` (superseding the stale repository inventory note). `search` is USD `0.005` and `result` is USD `0.0003`; run reports `search: 1`, `result: 1`. Apify did not return a separate event-charge amount. | `UNDER_MAINTENANCE` -> `NONE`; refreshed history has this run first. |
| YouTube Transcript Scraper | `ztc698cHC09lkCDYE` | `QCcV5FQUckuBydA4f` / `SUCCEEDED` | `2A6MAGndQAxjjBMWf` / `SUCCEEDED` | Default dataset readable with one row containing `videoId`, language metadata, `transcript`, `text`, and `segmentCount`; `segmentCount` is 61 and matches the transcript array length. `OUTPUT` is readable with the documented transcript fields. Log confirms successful fetch of 61 segments; no failure/error lines. | `PAY_PER_EVENT`, active since `2026-05-19T11:18:38.521Z`; primary `apify-default-dataset-item` event at USD `0.0003`. Run reports `apify-default-dataset-item: 1`; Apify did not return a separate event-charge amount. | `UNDER_MAINTENANCE` -> `NONE`; refreshed history has this run first. |

## Pre- and post-action checks

- Before the smoke runs, all three Actors were public, showed `UNDER_MAINTENANCE`, had no entries in the latest five-run response, had the expected successful latest build, and had active paid pricing in `pricingInfos`.
- The default-version `SCRAPPA_API_KEY` was present and marked secret for Booking `1.0`, Maps `1.0`, and Transcript `0.0`. Secret values were not retrieved or recorded.
- Each run had terminal `SUCCEEDED` status with no charge-limit termination. Each resulting default dataset was readable. Booking is dataset-first; Maps and Transcript both had readable documented `OUTPUT` records.
- Post-clear actor detail remained public and returned `notice: "NONE"` with no `notices` collection. Refreshed five-run history shows the corresponding successful smoke run first.
- Health audit (direct `node scripts/audit-apify-health.mjs --json`, because `pnpm` is unavailable in this container) recorded all three target Actors as `OK`, with no notice. Portfolio summary: 12 `OK`, 75 `NO_RUNS`, 0 failed latest runs, 0 failed latest builds, 0 notices, and 0 audit errors.
- Pricing audit (direct `node scripts/audit-apify-pricing.mjs --json`) recorded 88 public Actors with active paid pricing, 0 overdue active-pricing gaps, 0 missing paid pricing, 0 future-only paid pricing, and 0 errors.

## Scope compliance

- Exactly one minimal smoke run was started for each approved Actor.
- No Actor code, price, version, build, deployment, visibility, title, or secret was changed.
- The only live mutation was the approved targeted Actor notice clear after each Actor passed all gate conditions.
- No failure diagnostics were generated; no follow-up code fix is needed from these smoke runs.
