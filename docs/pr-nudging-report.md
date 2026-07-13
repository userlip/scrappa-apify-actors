# PR Nudging Report: Google Finance Indices Scraper Follow-up

Date: 2026-07-13

## Pull request

- Branch: `fix/google-finance-indices-batch-runner`
- Base: `main` (`origin/main` at `09d6661`)
- Changes: `d47c279` and `08a6475`
- PR: pending creation
- Merge: not performed in this stage

## Scope

This follow-up addresses the batching issue found after PR #284 merged. A
single Actor run now processes each normalized requested index symbol through
Scrappa's `/google-finance/indices` endpoint and aggregates only rows that
match the requested symbol. It remains a 128 MB / 120-second thin wrapper,
uses dataset-only output, and charges one `index-result` event only after a
successful dataset write.

The marketplace README also adds the requested Google Finance Indices Scraper
title, batch example, S&P 500 / Dow / NASDAQ / custom-symbol use cases,
pricing, matching caveat, and direct Scrappa API upgrade path.

## Verification before opening the PR

Testing at `08a6475` passed:

- `npm test` — 18 passed, 0 failed.
- `npm run typecheck` — passed.
- `jq empty .actor/actor.json .actor/input_schema.json` — passed.
- `npx apify-cli validate-schema` — input and dataset schemas valid.
- `npm audit --omit=dev --audit-level=high` — 0 vulnerabilities.
- `git diff --check main...HEAD` — passed.

The code-review handoff found no blocking correctness, security, performance,
test-coverage, or repository-convention findings.

## CI and review status

To be updated after PR creation and GitHub checks complete. No merge was
performed in this stage.

## Downstream release gates

After merge, the release stage must deploy the Actor, activate (or
earliest-schedule) `$0.00025` PAY_PER_EVENT pricing for `index-result`, run a
multi-index smoke test, verify dataset/event parity, and rerun the Apify
pricing, health, secret, and source-parity audits.

## Outcome

PR monitoring in progress; no merge performed.
