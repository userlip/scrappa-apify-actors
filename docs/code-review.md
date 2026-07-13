# Code Review: ImmobilienScout24 Price Insights Scraper

## Result

No blocking findings. The implementation satisfies the repository's thin-wrapper and batch-first requirements.

## Review scope

- Reviewed `main...HEAD` (`0408483..459f67c`), including the metadata, schema, package metadata, focused regression tests, and implementation summary changes.
- Inspected the complete Actor source under `actors/immobilienscout24-price-insights-scraper/src` and the adjacent `immobilienscout24-search-scraper` conventions.
- Checked the Scrappa controller/OpenAPI contract for `GET /api/immobilienscout24/price-insights` in the available reference checkout.

## Findings

None.

The Actor normalizes array, CSV, and singular compatibility input; trims and case-insensitively deduplicates locations; rejects invalid, overlong, and over-limit input before network work; and processes the batch in bounded groups of ten. It calls only `/immobilienscout24/price-insights`, retries through the shared Scrappa client, preserves input-order writes, isolates per-location failures, and requires all four positive benchmark values plus resolved location, geocode, and currency before producing a dataset item.

PAY_PER_EVENT uses only `price-insight-result`. A PPE result increments the success count only after `Actor.pushData` confirms a charge, stops cleanly at the charge limit, and records an unconfirmed write as a failure. Failed or incomplete locations are not written or charged. The Actor does not write per-item key-value-store `OUTPUT` records and uses the required 128 MB resource profile.

The metadata description and README target property-market benchmarking, rent-vs-buy research, recurring city comparisons, batch usage, and the Scrappa direct API upgrade path. The input schema accepts the requested array-or-comma-separated contract and remains compatible with singular `location` input.

## Verification

- `npm test`: 26 passing tests.
- `npm run test:dev`: 26 passing tests against TypeScript source modules.
- `npm run typecheck`: passed.
- `jq empty .actor/actor.json .actor/input_schema.json`: passed.
- `npx apify-cli validate-schema`: input and dataset schemas passed.
- `npm run test:audit-health`: 17 passing tests.
- `npm run test:audit-secrets`: 19 passing tests.
- `npm run test:audit-pricing`: 14 passing tests.
- `git diff --check main...HEAD`: passed.
- Security/scope scan found no direct ImmobilienScout24 request, committed credential, per-item `OUTPUT` write, or unrelated application-source change.

## Release gates outside code review

Before public release, configure `SCRAPPA_API_KEY` as an Apify secret, build/deploy the Actor, run Berlin/Munich and mixed-failure smoke tests, compare dataset rows with confirmed charge events, and verify active or earliest-scheduled paid pricing for this exact Actor through the Apify API at the agreed `$0.0005` `price-insight-result` price. Those live gates were not available in this container.

CODE_REVIEW_PASSED
