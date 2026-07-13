# PR Nudging Report: Google Finance Indices Scraper

Date: 2026-07-13

## Pull request

- Branch: `fix/google-finance-indices-charge-limit-tests`
- Base: `main` (`origin/main` at `75df757`)
- Commits: `cf9f260` and `4e0fd68`
- PR: [#284 — fix: finalize Google Finance indices billing boundary](https://github.com/userlip/scrappa-apify-actors/pull/284)
- Merge: not performed in this stage

## Scope reviewed

The branch finalizes the new `actors/google-finance-indices-scraper` Actor's
Apify billing boundary:

- Removes the unnecessary `OUTPUT` key-value-store write; returned indices are
  available only as dataset items.
- Stops safely at the PAY_PER_EVENT capacity limit, retaining and charging an
  accepted final dataset item exactly once.
- Marks unattempted requested symbols correctly when the charge limit stops the
  batch, without treating them as upstream failures.
- Adds regression coverage for exhausted capacity, a refused dataset write, an
  accepted final write that raises the limit flag, and charge-event behavior.

The Actor remains a small batch-first wrapper around Scrappa's Google Finance
indices endpoint, with one dataset item and `index-result` event per accepted
result.

## Verification before PR

Testing reported the following successful checks from
`actors/google-finance-indices-scraper`:

- `npm test` — 16 passed, 0 failed.
- `npm run typecheck` — passed.
- `jq empty .actor/actor.json .actor/input_schema.json` — passed.
- `npx apify-cli validate-schema` — input and embedded dataset schemas passed.
- `npm audit --omit=dev --audit-level=high` — 0 vulnerabilities.
- `git diff --check 34b516e^..4e0fd68` and `git diff --check origin/main...HEAD` — passed.

No remote CI run existed before branch push. PR-stage review found no
additional actionable issue in the changed source files; the open PR is now
being monitored for checks and review feedback.

## Release gates retained for downstream stage

Apify deployment, active or earliest-scheduled `$0.00025` PAY_PER_EVENT
pricing for `index-result`, a multi-index smoke run, dataset-to-event parity,
and portfolio audits need the authorized release environment. They are not
performed by this PR stage; `docs/release-verification.md` records the access
boundary.

## Outcome

The branch is ready to push and open for CI/review. This stage does not merge.
