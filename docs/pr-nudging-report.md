# PR Nudging Report: Vinted User Profile Runtime Safety

Date: 2026-07-13

## Pull request

- Branch: `fix/vinted-user-profile-runtime-safety`
- Base: `origin/main`, which contains the merged Vinted User Profile Actor from PR #281
- PR: created after validation; merge is not performed in this stage

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

The branch is ready for GitHub CI and review monitoring after push. No implementation changes are required at PR-nudging time, and no merge will be performed in this stage.

## Outcome

PR created and awaiting green CI/review completion.
