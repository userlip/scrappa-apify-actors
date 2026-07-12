# Testing Report: ImmobilienScout24 Price Insights Scraper

Date: 2026-07-13

## Result

TESTING_PASSED

The focused test suites, schema checks, deployment build, paid pricing, secret configuration, Berlin/Munich batch smoke run, mixed-failure smoke run, and dataset/charge parity all passed for the new Actor.

## Local verification

| Check | Result |
| --- | --- |
| `npm test` from `actors/immobilienscout24-price-insights-scraper` | 26 passed, 0 failed |
| `npm run test:dev` | 26 passed, 0 failed |
| `npm run typecheck` | Passed |
| `jq empty .actor/actor.json .actor/input_schema.json` | Passed |
| `npx apify-cli validate-schema` | Input and dataset schemas passed |
| `npm run test:audit-health` | 17 passed, 0 failed |
| `npm run test:audit-secrets` | 19 passed, 0 failed |
| `npm run test:audit-pricing` | 14 passed, 0 failed |
| `git diff --check main...HEAD` | Passed |

The tests cover array, CSV, and singular input normalization; trimming and case-insensitive deduplication; the 100-location limit; bounded concurrency; endpoint and retry parameters; complete response mapping; partial failures; and charge-confirmed dataset writes.

## Deployment and configuration

Actor: `gw1ZWMNQMBu0dGUnz` (`immobilienscout24-price-insights-scraper`)

- Build `1.0.9`, ID `BC4VS1JHWLSqLyDls`: `SUCCEEDED`.
- Default run profile: 128 MB memory, 300-second timeout.
- Version `1.0` has `SCRAPPA_API_KEY` configured as a secret.
- Input schema on the deployed build accepts array-or-comma-separated `locations` and singular `location` compatibility input.
- Active pricing is `PAY_PER_EVENT` at `$0.0005` for the primary `price-insight-result` event.

The first source push warned that this container had no local secret and removed the remote version’s secret metadata. The existing Scrappa Apify key was restored from the configured Scrappa dashboard key and the Actor was rebuilt; the final configuration above was re-verified before smoke testing.

## Live smoke verification

### Berlin and Munich batch

Run: `PKs7fmAHPsPpPVhsh`

Dataset: `8LDNdmOH7v5df1MZt`

- Input: `{"locations":["Berlin","Munich"]}`
- Terminal status: `SUCCEEDED`.
- Dataset: exactly 2 rows, both complete EUR snapshots with all four benchmarks.
- Resolved locations: Berlin and München.
- `chargedEventCounts.price-insight-result`: `2`.
- Dataset writes: `2`.
- Default key-value store contained only `INPUT`; no per-item `OUTPUT` records were written.

### Mixed valid/invalid batch

Run: `h4gumcKUWI1YRBABi`

Dataset: `d7efboksQTr8Mw5M8`

- Input: one valid Berlin location and one deliberately invalid location.
- Terminal status: `SUCCEEDED`.
- Status message: `Saved 1 of 2 requested location snapshot(s); 1 failed.`
- Dataset: exactly 1 Berlin row.
- Invalid location: logged as `Bad Request`, omitted from the dataset, and not charged.
- `chargedEventCounts.price-insight-result`: `1`.
- Dataset writes: `1`.

The live results therefore confirm one dataset item and one charge per successful location, with partial failures isolated and uncharged.

## Portfolio audit context

- Secret audit: 92 actors checked, 92 secret-present, 0 missing, 0 non-secret, 0 errors.
- Pricing audit: 92 actors checked, 92 active paid pricing, 0 overdue, 0 missing, 0 future-only, 0 errors.
- Health audit: the new Actor is `OK` with five recent `SUCCEEDED` runs and a successful latest build. The portfolio-wide command remains non-zero because the unrelated `tiktok-challenge-posts-scraper` (`CVaJEgPjl3jWKbm71`) has a failed latest run; this does not affect the tested Actor.

No source code changes were made during testing. The deployment and secret restoration were operational release actions for the reviewed branch.
