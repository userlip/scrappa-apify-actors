# PR Nudging Report: Google Maps Directions Scraper

Date: 2026-07-13

## Pull request

- Branch: `release/google-maps-directions-scraper`
- Pull request: [#282 — Add Google Maps Directions Scraper Actor](https://github.com/userlip/scrappa-apify-actors/pull/282)
- Base: `main`
- State: open; no merge performed in this stage

The PR contains the tested thin Scrappa wrapper, batch-first route handling,
per-alternative dataset output and `route-result` charge accounting, actor
metadata/docs, release evidence, and the required root README inventory row.

## Validation before push

From `actors/google-maps-directions-scraper` on the PR branch:

- `npm test` — 16 passed, 0 failed.
- `npm run typecheck` — passed.
- `npx apify-cli validate-schema` — input and dataset schemas passed.
- `npm audit --omit=dev --audit-level=high` — 0 vulnerabilities.
- `git diff --check` — passed.

Live release evidence recorded by testing:

- Actor `ZF8jFdzF15k49AZQh` is public, configured for 128 MB and 300 seconds,
  retains `SCRAPPA_API_KEY` as a secret, and has active `$0.0005`
  `route-result` PAY_PER_EVENT pricing.
- Build `1.0.5` (`WyXaXBr5ncpnhxIpl`) succeeded.
- Two-route smoke `X3G3VmrqE91coTP8B` stored 6 rows and charged 6 events.
- Mixed-failure smoke `NwhoCso1d0soax99d` stored and charged 3 valid rows,
  continued after one `NO_ROUTES_FOUND` failure, and charged nothing for the
  failed request.

## CI and review monitoring

- Socket Security Project Report: passed.
- Socket Security Pull Request Alerts: skipped/neutral, with no alert.
- Claude review: skipped by repository configuration.
- Cubic AI code reviewer: still pending at report time; no comments or formal
  review requests were posted.
- No actionable review feedback was received, so no follow-up code changes
  were needed.

Unrelated pre-existing worktree edits in `handoff.md`, `.codegraph/`, and
`docs/source-document.md` were preserved and excluded from the PR.

## Outcome

The tested Actor is in PR #282 with the release gates and available checks
passing. Downstream merge/deploy should re-check the external Cubic status
before merging. This stage did not merge the PR.
