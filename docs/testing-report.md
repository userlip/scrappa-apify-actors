# Testing Report: TikTok Challenge Details Scraper

Date: 2026-07-11

## Result

TESTING_PASSED

The new `actors/tiktok-challenge-details-scraper` passes its focused automated coverage and static validation. The tests cover the batch-first workflow, mixed names/IDs, deduplication, the combined 100-entity guard, partial upstream failures, response normalization, and PAY_PER_EVENT charging limits.

## Automated verification

Run from `actors/tiktok-challenge-details-scraper`:

```text
npm test
13 passed, 0 failed

npm run typecheck
passed

jq empty .actor/actor.json .actor/input_schema.json
passed

git diff --check 913e3fc..HEAD
passed
```

The test cases specifically confirm that a successful later entity survives an earlier API failure, no request starts after charge capacity is exhausted, one successful PAY_PER_EVENT dataset row has exactly one `challenge-detail-result` charge, short event charges are surfaced safely, and development pricing can save without an event.

## Direct verification and release gates

This test container has no `SCRAPPA_API_KEY` configured and the Actor has not yet been deployed. A live mixed name/ID smoke run was therefore not attempted locally; doing so requires the deployed Actor secret and must not use a test report as a substitute for production verification.

Before publication, the deployment and live-verification stages must:

1. Configure `SCRAPPA_API_KEY` as an Apify secret for the deployed version.
2. Create and API-verify active, or earliest-scheduled, paid `PAY_PER_EVENT` pricing for `challenge-detail-result` at USD `$0.00025` per successful row.
3. Run a mixed batch using `booktok` and `1622962893630470`, plus a known-invalid value for the partial-failure path; confirm terminal success, one dataset item and one paid event per resolved detail, per-entity failure reporting, and no charge for the invalid value.

No source changes were made during testing.
