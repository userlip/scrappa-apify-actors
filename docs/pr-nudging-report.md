# PR Nudging Report: Vinted User Profile Runtime Safety

Date: 2026-07-13

## Pull request

- Branch: `fix/vinted-user-profile-runtime-safety`
- Base: `origin/main`, which contains the merged Vinted User Profile Actor from PR #281
- PR: [#283 — fix: harden Vinted profile batch runtime and billing](https://github.com/userlip/scrappa-apify-actors/pull/283)
- Merge: not performed in this stage

## Scope

This follow-up carries the two post-merge safety fixes for the public `thescrappa/vinted-user-profile-scraper` Actor:

- Bound batches to eight concurrent Scrappa calls, include retry-delay time in the runtime budget, cap waves by available PPE capacity, and retain a 60-second margin under the 600-second Actor timeout.
- Preserve unlimited PPE capacity and drain sibling workers after an actor-level Scrappa authentication failure before rethrowing, without writing or charging delayed sibling results.

## Validation before push

From `actors/vinted-user-profile-scraper`:

- `npm test` — 22 passed, 0 failed.
- `npm run test:dev` — 22 passed, 0 failed.
- `npm run typecheck` — passed.
- `npx --yes apify-cli validate-schema` — input and embedded dataset schemas passed.
- `git diff --check origin/main..HEAD` and `git diff --check` — passed.

The focused tests cover unlimited PPE capacity, bounded concurrency, maximum-batch runtime alignment, auth-failure draining, exact endpoint behavior, and successful-save/charge semantics. Existing testing and release evidence records successful deployed multi-profile and mixed-success smoke runs, active `$0.0005` `user-profile-result` pricing, the `SCRAPPA_API_KEY` secret, and dataset-to-charge parity.

## CI and review monitoring

- Actor Tests workflow run `29261462378`: all matrix jobs passed.
- Claude review: passed with no blocking findings.
- Cubic review: passed with no actionable findings.
- Socket Security Project Report and Pull Request Alerts: passed.
- No implementation changes or review fixes were required during PR nudging.

## Outcome

PR #283 is open with green CI and review checks; no merge was performed.
