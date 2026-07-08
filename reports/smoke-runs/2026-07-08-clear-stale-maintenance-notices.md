# Clear stale Apify maintenance notices - 2026-07-08

## Scope

Targeted only the four public paid TheScrappa actors named in the cleanup task:

| Actor ID | Slug | Directory |
| --- | --- | --- |
| `IIPXRhbeyXH7ssOK6` | `google-flights-search-scraper` | `actors/google-flights-search-scraper` |
| `Kc3rfsV2Hif23mctw` | `google-hotels-search-scraper` | `actors/google-hotels-search-scraper` |
| `hoF0Qgm3S0wAfpY8y` | `google-trends-related-queries-scraper` | `actors/google-trends-related-queries-scraper` |
| `u5QoR4Um3MXwbdavk` | `kununu-reviews-scraper` | `actors/kununu-reviews-scraper` |

Direct actor detail at `2026-07-08T21:33Z` did not match the task baseline exactly: Google Flights and Google Trends Related Queries already returned `notice: "NONE"`. Google Hotels and Kununu still returned `UNDER_MAINTENANCE`, so only those two actor notice fields were changed.

## Pre-clear checks

All four actors were public, TheScrappa-owned, and actively paid before any metadata update.

| Actor ID | Slug | Notice before | Active pricing evidence |
| --- | --- | --- | --- |
| `IIPXRhbeyXH7ssOK6` | `google-flights-search-scraper` | `NONE` | `PAY_PER_EVENT`, `flight-result` at `$0.0002`, active from `2026-05-11T21:10:28.856Z` |
| `Kc3rfsV2Hif23mctw` | `google-hotels-search-scraper` | `UNDER_MAINTENANCE` | `PAY_PER_EVENT`, `apify-default-dataset-item` at `$0.0002`, active from `2026-05-26T08:23:20.330Z` |
| `hoF0Qgm3S0wAfpY8y` | `google-trends-related-queries-scraper` | `NONE` | `PAY_PER_EVENT`, `related-result` at `$0.0002`, active from `2026-06-07T08:00:00.000Z` |
| `u5QoR4Um3MXwbdavk` | `kununu-reviews-scraper` | `UNDER_MAINTENANCE` | `PAY_PER_EVENT`, `review-result` at `$0.00025`, active from `2026-05-29T22:26:00.000Z` |

Because the previous run evidence was stale relative to the July 8 cleanup date, fresh README-based smoke inputs were run for all four target actors.

| Actor ID | Slug | Fresh run ID | Status | Dataset ID | Dataset items | Build |
| --- | --- | --- | --- | --- | ---: | --- |
| `IIPXRhbeyXH7ssOK6` | `google-flights-search-scraper` | `2GtqcFlcw2GA7jXME` | `SUCCEEDED` | `JF1GxytOxeTBOqfcQ` | 112 | `bN2aTiXpjvvynl2eI` / `1.0.3` |
| `Kc3rfsV2Hif23mctw` | `google-hotels-search-scraper` | `RK4Ljd6LLEdQTPvRx` | `SUCCEEDED` | `kAMaRurCOGROYihrb` | 18 | `lbbXlKllL85dphblP` / `1.0.4` |
| `hoF0Qgm3S0wAfpY8y` | `google-trends-related-queries-scraper` | `fbpafS08mOEroUvzK` | `SUCCEEDED` | `YaZW3H2LfTtat2ifK` | 25 | `VGqaWARXwacoY3RdM` / `1.0.3` |
| `u5QoR4Um3MXwbdavk` | `kununu-reviews-scraper` | `aerwL0o0tDXVLSbpu` | `SUCCEEDED` | `vKxh8Mm0CBZq2Ttru` | 10 | `X58Z6Wcfd9fMtH2E4` / `1.0.13` |

## Metadata updates

The stale target notices were cleared with the Apify Actor update API:

```json
{"notice":"NONE"}
```

No source code, builds, pricing, secrets, or publication settings were changed.

| Actor ID | Slug | Notice before | Notice after | Modified at after |
| --- | --- | --- | --- | --- |
| `Kc3rfsV2Hif23mctw` | `google-hotels-search-scraper` | `UNDER_MAINTENANCE` | `NONE` | `2026-07-08T21:35:38.672Z` |
| `u5QoR4Um3MXwbdavk` | `kununu-reviews-scraper` | `UNDER_MAINTENANCE` | `NONE` | `2026-07-08T21:35:39.159Z` |

## Post-clear verification

Direct `GET /v2/acts/{actorId}` checks confirmed all four target actors now return `notice: "NONE"` and `notices: null`.

`APIFY_TOKEN=... node scripts/audit-apify-health.mjs --json` at `2026-07-08T21:36:21.079Z` reported:

| Metric | Count |
| --- | ---: |
| `ok` | 7 |
| `noRuns` | 76 |
| `failedLatestRuns` | 0 |
| `nonTerminalLatestRuns` | 0 |
| `failedLatestBuilds` | 0 |
| `recentFailedButLatestOk` | 0 |
| `notices` | 1 |
| `auditErrors` | 0 |

All four target actors were `OK` with latest run `SUCCEEDED` and `notice: null`.

The one remaining health-audit notice is scoped out of this task:

| Actor ID | Slug | Notice | Latest run |
| --- | --- | --- | --- |
| `BehWN3LEvBxhEiJDF` | `booking-search-scraper` | `UNDER_MAINTENANCE` | `YDwODp0Ukr6aeQshS` / `SUCCEEDED` |

`APIFY_TOKEN=... node scripts/audit-apify-pricing.mjs --json --include-active` at `2026-07-08T21:36:13.320Z` reported all `83` public actors with active paid pricing and zero pricing errors. The four target actors were all `ACTIVE_PAID_PRICING`.

Rendered marketplace-page text checks for the four target URLs returned HTTP `200` and did not contain `under maintenance`.

## Outcome

The four requested target actors are public, actively paid, freshly smoke-tested, and no longer listed as notice actors by the health audit. The only remaining portfolio notice is `booking-search-scraper`, which was intentionally left untouched because it was outside the approved four-actor scope.
