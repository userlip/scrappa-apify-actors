# Clear stale Apify maintenance notices - 2026-06-19

## Scope

Removed stale `UNDER_MAINTENANCE` notices from these recovered public paid TheScrappa actors:

| Actor ID | Slug | Directory |
| --- | --- | --- |
| `IIPXRhbeyXH7ssOK6` | `google-flights-search-scraper` | `actors/google-flights-search-scraper` |
| `Kc3rfsV2Hif23mctw` | `google-hotels-search-scraper` | `actors/google-hotels-search-scraper` |
| `u5QoR4Um3MXwbdavk` | `kununu-reviews-scraper` | `actors/kununu-reviews-scraper` |

`booking-search-scraper` was intentionally excluded. It still reports `UNDER_MAINTENANCE` and was used only as a rendered marketplace control.

## Apify metadata updates

Updates were made with the Apify Actor update API using a minimal payload:

```json
{"notice": null}
```

Apify documents this endpoint as updating only supplied actor properties; omitted properties are not changed.

| Actor ID | Slug | Notice before | Notice after | API modifiedAt after |
| --- | --- | --- | --- | --- |
| `IIPXRhbeyXH7ssOK6` | `google-flights-search-scraper` | `UNDER_MAINTENANCE` | `null` | `2026-06-19T14:09:19.872Z` |
| `Kc3rfsV2Hif23mctw` | `google-hotels-search-scraper` | `UNDER_MAINTENANCE` | `null` | `2026-06-19T14:09:29.293Z` |
| `u5QoR4Um3MXwbdavk` | `kununu-reviews-scraper` | `UNDER_MAINTENANCE` | `null` | `2026-06-19T14:09:30.097Z` |

After-state actor detail API checks also confirmed all three remain `isPublic: true`.

## Pre-removal health and monetization checks

| Actor ID | Slug | Pricing status | Env var status | Latest run | Latest build |
| --- | --- | --- | --- | --- | --- |
| `IIPXRhbeyXH7ssOK6` | `google-flights-search-scraper` | `PAY_PER_EVENT`, `flight-result` at `$0.0002` | `SCRAPPA_API_KEY` present | `SQT5VpSG47cgBGq4B` `SUCCEEDED`, dataset `JCRVsFIBhAYqhXtLP` | `bN2aTiXpjvvynl2eI` `SUCCEEDED` |
| `Kc3rfsV2Hif23mctw` | `google-hotels-search-scraper` | `PAY_PER_EVENT`, `apify-default-dataset-item` at `$0.0002` | `SCRAPPA_API_KEY` present | `JyKh0eEdbd0lgoPlN` `SUCCEEDED`, dataset `ffytznRh0gnRQHBho` | `lbbXlKllL85dphblP` `SUCCEEDED` |
| `u5QoR4Um3MXwbdavk` | `kununu-reviews-scraper` | `PAY_PER_EVENT`, `review-result` at `$0.00025` | `SCRAPPA_API_KEY` present | `AEGRatHxSnkQQyb9w` `SUCCEEDED`, dataset `kS8R7NLQ6k5d5bdTG` | `X58Z6Wcfd9fMtH2E4` `SUCCEEDED` |

Latest dataset samples:

| Actor ID | Slug | Latest dataset item count | Sample output evidence |
| --- | --- | ---: | --- |
| `IIPXRhbeyXH7ssOK6` | `google-flights-search-scraper` | 29 | keys include `airline_names`, `departure_airport`, `arrival_airport`, `price`, `booking_token` |
| `Kc3rfsV2Hif23mctw` | `google-hotels-search-scraper` | 20 | keys include `entity_id`, `description`, `amenities`, `booking_link`, `gps_coordinates` |
| `u5QoR4Um3MXwbdavk` | `kununu-reviews-scraper` | 10 | keys include `company_name`, `company_slug`, `date`, `rating`, `page_total_results` |

## Post-removal audits

Required commands were run from `/home/ploi/workspaces/scrappa-apify-actors-BuA7Jyio` with the configured Apify organization token.

```bash
node scripts/audit-apify-health.mjs --json
node scripts/audit-apify-pricing.mjs --json --include-active
```

Health audit checked at `2026-06-19T14:10:19.287Z`:

| Metric | Count |
| --- | ---: |
| `ok` | 33 |
| `noRuns` | 47 |
| `failedLatestRuns` | 0 |
| `nonTerminalLatestRuns` | 0 |
| `failedLatestBuilds` | 0 |
| `recentFailedButLatestOk` | 2 |
| `notices` | 2 |
| `excludedPublicActors` | 0 |
| `auditErrors` | 0 |

The three cleaned actors were reported as `OK`, with latest run `SUCCEEDED`, latest build `SUCCEEDED`, and `notice: null`.

Remaining health-audit notices:

| Slug | Notice |
| --- | --- |
| `booking-search-scraper` | `UNDER_MAINTENANCE` |
| `google-trends-related-queries-scraper` | `UNDER_MAINTENANCE` |

Pricing audit checked at `2026-06-19T14:10:10.186Z`:

| Metric | Count |
| --- | ---: |
| `activePaidPricing` | 80 |
| `overdueMissingActivePricing` | 0 |
| `missingPaidPricing` | 0 |
| `futureOnlyPaidPricing` | 2 |
| `errors` | 0 |

The three cleaned actors were all reported as `ACTIVE_PAID_PRICING`.

## Marketplace verification

Rendered Apify Store pages were checked with Playwright by evaluating visible `document.body.innerText` for `/under maintenance/i`.

| Page | Rendered maintenance text |
| --- | --- |
| `https://apify.com/thescrappa/google-flights-search-scraper` | absent |
| `https://apify.com/thescrappa/google-hotels-search-scraper` | absent |
| `https://apify.com/thescrappa/kununu-reviews-scraper` | absent |
| `https://apify.com/thescrappa/booking-search-scraper` | present, control actor intentionally excluded |

## Outcome

The stale marketplace warning was removed from exactly the three recovered green actors. No actor source code, pricing configuration, secrets, builds, or publication settings were changed.
