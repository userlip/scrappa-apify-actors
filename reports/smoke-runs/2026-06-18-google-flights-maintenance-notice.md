# 2026-06-18 Google Flights Maintenance Notice Clearance

Scope: clear the stale Apify `UNDER_MAINTENANCE` notice from public paid actor `google-flights-search-scraper` after confirming current run, build, pricing, and secret evidence remained healthy.

| Actor | Actor ID | Before notice | Evidence run | Latest build | Pricing | Secret | Final notice state |
| --- | --- | --- | --- | --- | --- | --- | --- |
| `google-flights-search-scraper` | `IIPXRhbeyXH7ssOK6` | `UNDER_MAINTENANCE` | `SQT5VpSG47cgBGq4B` succeeded at `2026-06-17T09:34:18.751Z` | `bN2aTiXpjvvynl2eI` succeeded, build `1.0.3` | Active `PAY_PER_EVENT`, `flight-result` at `$0.0002/result` since `2026-05-11T21:10:28.856Z` | `SCRAPPA_API_KEY` present as secret on version `1.0` | Cleared; Apify detail returned `notice: "NONE"` and health audit reports `notice: null`. |

Commands and API surfaces used:

- `GET /v2/acts/IIPXRhbeyXH7ssOK6` confirmed the actor was public, paid, and still marked `notice: "UNDER_MAINTENANCE"` before the change.
- `GET /v2/acts/IIPXRhbeyXH7ssOK6/runs?limit=5&desc=true` confirmed latest run `SQT5VpSG47cgBGq4B` was `SUCCEEDED`.
- `PUT /v2/actors/IIPXRhbeyXH7ssOK6` with body `{"notice":"NONE"}` cleared only the actor-level notice.
- `node scripts/audit-apify-health.mjs --json` confirmed `google-flights-search-scraper` remained `OK`, had no notice, and total notice actors decreased from `2` to `1`.
- `node scripts/audit-apify-pricing.mjs --json` confirmed no missing paid pricing across public actors; Google Flights direct detail still showed active paid `PAY_PER_EVENT` pricing.
- `node scripts/audit-apify-secrets.mjs --json` confirmed all `81` public actors had `SCRAPPA_API_KEY` present as a secret.

Post-change health audit summary at `2026-06-18T08:00:36.044Z`:

| Metric | Count |
| --- | ---: |
| OK actors | 29 |
| No-run actors | 50 |
| Failed latest runs | 1 |
| Failed latest builds | 0 |
| Notice actors | 1 |
| Audit errors | 0 |

Remaining notice actor:

| Actor | Actor ID | Notice | Status | Latest run |
| --- | --- | --- | --- | --- |
| `booking-search-scraper` | `BehWN3LEvBxhEiJDF` | `UNDER_MAINTENANCE` | `FAILED_LATEST_RUN` | `IfIJZb88gFIU1Thjw` |

Notes:

- The remaining `booking-search-scraper` notice was intentionally left in place because the latest run is failing.
- No actor source code was changed or deployed for this task.
