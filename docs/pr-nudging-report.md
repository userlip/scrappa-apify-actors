# PR Nudging Report: Stale Apify Notice Smoke Gate

## Release gates

- Branch: `chore/smoke-gate-three-actors`.
- Scope: documentation-only evidence for three controlled Apify smoke runs. No Actor source, deployment, secret, or pricing changes are included.
- Local validation: `node --test scripts/audit-apify-health.test.mjs scripts/audit-apify-pricing.test.mjs` passed (31 tests); `git diff --check origin/main...HEAD` passed.
- Unrelated untracked paths `.codegraph/` and `docs/source-document.md` remain excluded.

## Verified smoke evidence

| Actor | ID | Run | Terminal result | Dataset / paid event | Maintenance notice |
| --- | --- | --- | --- | --- | --- |
| Booking Search Scraper | `BehWN3LEvBxhEiJDF` | `vgKnULJIsxREWa7Z7` | `SUCCEEDED` | 26 schema-valid rows; `booking-result: 26` | Cleared |
| Google Maps Advanced Search Scraper | `DT8bUdm2Vn4HjlyDo` | `JVEjCoJ01QMfgzL0v` | `SUCCEEDED` | 1 schema-valid row; `search: 1` | Cleared |
| YouTube Transcript Scraper | `ztc698cHC09lkCDYE` | `2A6MAGndQAxjjBMWf` | `SUCCEEDED` | 1 schema-valid row; `apify-default-dataset-item: 1` | Cleared |

Each Actor remains publicly available with active `PAY_PER_EVENT` pricing. The implementation and testing records confirm the default-dataset shapes and that the public run API exposes the authoritative `actId` field.

## PR status

PR [#268](https://github.com/userlip/scrappa-apify-actors/pull/268) is open against `main`. The merge-safe evidence update in `b64fb1b` resolved the two review findings about intermediate commit references. Required checks now pass: Claude Code Review, Cubic AI code reviewer, Socket Security Project Report, and Socket Security Pull Request Alerts. The PR is clean and ready for the merge stage; this stage does not merge.

## Outcome

PR_NUDGING_PASSED
