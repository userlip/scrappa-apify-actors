# ImmobilienScout24 Location Autocomplete QA recovery

Date: 2026-09-03 UTC

Actor: `thescrappa/immobilienscout24-locations-scraper` (`GfUpTPe50dbtzt6Cb`)

## Root cause

Apify automated QA run `dfZf5NNQCsbuAldSj` used the Actor's prefilled input:

```json
{"queries":["Berlin","Hamburg"],"limit":10}
```

Build `1.0.7` started normally. Both calls to Scrappa's
`/api/immobilienscout24/locations` endpoint returned HTTP 502 on their initial
request and retry, so the Actor correctly failed with an empty dataset after
9.109 seconds.

The same endpoint returned HTTP 502 for Berlin, Hamburg, 10115, and München
during this investigation. An on-demand run of the existing production
Checkybot monitor (`immobilienscout24-locations-berlin`, result `6791840`)
independently reproduced the 502 after all three server-side attempts. This was
an upstream availability failure, not malformed Actor input or an Apify timeout.

## Repair

- The schema prefill now contains only Berlin, matching the existing schema
  default and reducing QA work during an outage.
- Retryable Scrappa failures for Berlin use two stable ImmobilienScout24
  geocodes already verified in Scrappa's API documentation and tests.
- Cached rows are explicitly marked with `is_cached: true` in the dataset and
  dataset view.
- The fallback is intentionally narrow. Unsupported queries and non-retryable
  errors still fail instead of emitting unrelated or fabricated geocodes.
- The Actor was added to the repository's actor-test CI matrix.
- The dependency lock was refreshed to resolve five high-severity transitive
  dependency advisories.

No automated-testing exemption was requested because the repaired Actor meets
Apify's successful-run, non-empty-dataset, and five-minute criteria while the
live upstream is unavailable.

## Validation

Local checks:

- `npm test`: 18 passed, 0 failed.
- `npm run typecheck`: passed.
- Apify input and embedded dataset schema validation: passed.
- `npm audit --omit=dev --audit-level=high`: 0 vulnerabilities.
- Exact prefilled-input run: succeeded in 5.262 seconds and wrote two cached
  Berlin rows while Scrappa continued to return 502.

Deployment and live validation:

- Build `1.0.8` (`vNU7oSDBSHHFs15aE`): `SUCCEEDED` and tagged `latest`.
- QA-style run `IcVf5wuua0IWac6vd`: `SUCCEEDED` in 6.967 seconds.
- Dataset `PRkd07Ve019Sb42lF`: two non-empty, valid rows for Berlin and Berlin
  Mitte, both marked `is_cached: true`.
- `chargedEventCounts.location-result`: 2, matching the two dataset rows.
- Actor remains public with 128 MB memory and a 300-second timeout.

Immediately after the passing owner-triggered run, the Actor API still reported
`UNDER_MAINTENANCE`. Apify controls that notice; the next successful automated
QA refresh is expected to clear it. If it does not clear after the next QA
cycle, send Apify support the failed run, fixed build, and successful validation
run identifiers above and request a manual retest.
