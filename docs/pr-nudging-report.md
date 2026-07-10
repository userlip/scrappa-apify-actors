# PR Nudging Report: Kleinanzeigen Listing Details Scraper

## Release gates

- Branch: `feat/kleinanzeigen-listing-details-scraper`
- Deployed Apify actor: `kleinanzeigen-listing-details-scraper` (`1hSNdgPwGINp7xeHB`), public.
- Cloud build: `yuhYcYVKEgrqygJhk` (`1.0.2`), succeeded.
- Scrappa API key: configured as the encrypted `SCRAPPA_API_KEY` secret on version `1.0`.
- Pricing: active `PAY_PER_EVENT` from `2026-07-10T11:19:59.585Z`; `listing-detail-result` is the primary event at `$0.00025` per successful listing.

## Live verification

Public smoke run `wcCLm7I6kdC973Azh` supplied two distinct ad IDs in one run (`3348371956`, `3401748259`). It succeeded, saved two dataset items, and finalized Apify accounting reported exactly two `listing-detail-result` events.

## Local verification

- `npm --prefix actors/kleinanzeigen-listing-details-scraper test` — passed (15 tests).
- `npm --prefix actors/kleinanzeigen-listing-details-scraper run test:dev` — passed (15 tests).
- `npm --prefix actors/kleinanzeigen-listing-details-scraper run typecheck` — passed.
- `git diff --check main...HEAD` — passed.

## Repository inventory

`README.md` now contains the new live actor, actor ID, deployment/build evidence, smoke-run evidence, and active pricing status, as required by `CONTRIBUTING.md`.

## Review follow-up

Automated review feedback was addressed with a shared error-sanitization utility. It redacts credential-like values from upstream error bodies before they can reach logs or the `OUTPUT` failure summary. The follow-up additionally rejects explicitly supplied `null` batch entries and accepts both root-level and documented wrapped detail payloads, including an upstream detail missing its own ID (which is safely replaced by the requested ID). These paths are covered by the 15-test suite.

An all-failed batch now persists its aggregate `OUTPUT` summary and then fails the run. This preserves per-ID failure details while ensuring monitoring does not interpret zero saved results as success. The default timeout is eight hours, which covers the advertised 100-ID sequential flow even if every request consumes all three 90-second attempts and retry backoff.

## PR status

PR [#265](https://github.com/userlip/scrappa-apify-actors/pull/265) is open, mergeable, and its completed GitHub checks are successful. The branch is pushed with the review follow-up; CI/review must complete again for the final merge handoff. Do not merge in this stage.
