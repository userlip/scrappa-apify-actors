# Code Review: Google Maps Directions Scraper

## Result

CODE_REVIEW_PASSED

No blocking correctness, security, simplicity, performance, test-coverage, or repository-convention findings were identified in the committed implementation diff (`origin/main...HEAD`).

The Actor is a thin batch-first wrapper around Scrappa's `/api/maps/directions` endpoint. It validates, normalizes, deduplicates, and bounds route input; processes up to 10 unique requests in one Apify run; isolates per-request failures; preserves returned alternatives and source-request metadata; and writes one dataset item per successfully stored alternative. The 240-second shared deadline is bounded below the configured 300-second wrapper timeout, and there are no per-item key-value-store writes.

The charge path uses the `route-result` PAY_PER_EVENT event, checks capacity before upstream work, and treats a row as saved only after Apify reports a successful charge. It caps Apify's aggregate explicit-plus-synthetic charge result to one route charge per stored row, matching the monetizable row semantics. No API key or other secret is present in the committed source, schema, Dockerfile, or documentation.

## Verification

Run from `actors/google-maps-directions-scraper`:

- `npm test` — 16 passing
- `npm run typecheck` — passed
- `jq empty .actor/actor.json .actor/input_schema.json` — passed
- `npx apify-cli validate-schema` — input and dataset schemas passed
- `git diff --check origin/main...HEAD` — passed
- `npm audit --omit=dev --audit-level=high` — 0 vulnerabilities

The focused tests cover input normalization/deduplication/limits, endpoint construction and retries, response alternative extraction and source enrichment, partial failures, deadline exhaustion, charge refusal, and aggregate Apify charge accounting.

## Downstream release gates

This code review does not substitute for release verification. The deployment stages must verify the deployed `SCRAPPA_API_KEY` secret, successful build, public status, 128 MB/300-second profile, active or earliest-allowed paid `route-result` pricing at `$0.0005`, and fresh two-route plus mixed-failure live dataset/charge parity.
