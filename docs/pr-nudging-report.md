# PR Nudging Report: Google Finance Indices Scraper Follow-up

Date: 2026-07-13

## Pull request

- Branch: `fix/google-finance-indices-batch-runner`
- Base: `main` (`origin/main` at `09d6661`)
- Changes: `d47c279` and `08a6475`
- PR: [#285 — fix: batch Google Finance index lookups](https://github.com/userlip/scrappa-apify-actors/pull/285)
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

The initial GitHub workflows passed: the 22-job Actor Tests matrix and the
automated Claude review both completed successfully. Cubic then reported one
valid correctness issue: a capacity-truncated batch could stop processing
responses that had already been fetched. This was fixed with separate
fetch-truncation and mid-processing charge-limit state, with a regression test
that confirms two already-fetched symbols are both saved at capacity two.

The review also requested `maxItems` for the union `string | array` input.
Apify's input-schema validator rejects that keyword for a union property, so
the supported CSV-or-array contract remains validated by `normalizeIndices`;
the test suite covers its strict three-symbol limit. A schema containing the
proposed keyword was rejected by `npx apify-cli validate-schema` and was not
shipped.

Corrective commit `f43a34c` was pushed and verified with `npm test` (20
passing), typecheck, schema validation, production dependency audit, and diff
checks. The fresh 22-job GitHub Actor Tests matrix and refreshed automated
review both completed successfully. The review confirmed that `maxItems` is
technically infeasible for the union contract and that runtime enforcement is
the appropriate supported control. No blocking review feedback remains; no
merge was performed in this stage.

## Downstream release gates

After merge, the release stage must deploy the Actor, activate (or
earliest-schedule) `$0.00025` PAY_PER_EVENT pricing for `index-result`, run a
multi-index smoke test, verify dataset/event parity, and rerun the Apify
pricing, health, secret, and source-parity audits.

## Outcome

PR is ready for downstream merge/deploy; no merge performed in this stage.
