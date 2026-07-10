# Testing Report: Stale Apify Notice Smoke Gate

Date: 2026-07-10

## Result

TESTING_PASSED

The targeted regression checks and public Apify verification passed. The documented smoke runs remain terminally successful, produce the expected dataset shapes, account for paid events, and have no maintenance notice. No Actor source or pricing change was included in the reviewed implementation.

## Automated checks

```text
node --test scripts/audit-apify-health.test.mjs scripts/audit-apify-pricing.test.mjs
31 passed, 0 failed

git diff --check origin/main...HEAD
passed
```

## Live verification

The public Apify API was refetched for each recorded run, Actor, and default dataset. Each Actor is public, has a cleared notice (`NONE` or absent), has an active `PAY_PER_EVENT` pricing record, and its run is linked through `actId`, terminally `SUCCEEDED`, and reports the expected positive charge event.

| Actor ID | Smoke run ID | Dataset verification | Paid event verification |
| --- | --- | --- | --- |
| `BehWN3LEvBxhEiJDF` | `vgKnULJIsxREWa7Z7` | 26 rows; first row includes `name`, `url`, `currency`, and documented request context fields | `booking-result: 26` |
| `DT8bUdm2Vn4HjlyDo` | `JVEjCoJ01QMfgzL0v` | 1 row; includes `name`, `rating`, `review_count`, `full_address`, `phone_numbers`, `website`, `latitude`, and `longitude` | `search: 1` (the implementation record also confirms `result: 1`) |
| `ztc698cHC09lkCDYE` | `2A6MAGndQAxjjBMWf` | 1 row; includes `videoId`, `transcript`, `text`, and `segmentCount`; `segmentCount` equals transcript-array length | `apify-default-dataset-item: 1` |

The unauthenticated public run endpoint returns `actId` (not `actorId`); verification uses this authoritative field. Authenticated portfolio-wide audit commands were not rerun in this test container because no Apify token is configured here. The implementation record provides the previously completed authenticated audit evidence; the in-scope public smoke evidence was independently revalidated above.

## Scope and handoff

- Reviewed change: the documentation-only branch diff (`origin/main...HEAD`), covering the implementation, code-review, testing, and PR-nudging records; no source, build, deployment, secret, or pricing change.
- Existing unrelated untracked paths (`.codegraph/` and `docs/source-document.md`) were not modified.
- No regression or follow-up implementation work is required.
