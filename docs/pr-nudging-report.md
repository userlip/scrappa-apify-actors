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

- `npm --prefix actors/kleinanzeigen-listing-details-scraper test` — passed (14 tests).
- `npm --prefix actors/kleinanzeigen-listing-details-scraper run test:dev` — passed (14 tests).
- `npm --prefix actors/kleinanzeigen-listing-details-scraper run typecheck` — passed.
- `git diff --check main...HEAD` — passed.

## Repository inventory

`README.md` now contains the new live actor, actor ID, deployment/build evidence, smoke-run evidence, and active pricing status, as required by `CONTRIBUTING.md`.

## PR status

Automated review feedback was addressed with a shared error-sanitization utility. It redacts credential-like values from upstream error bodies before they can reach logs or the `OUTPUT` failure summary; its coverage is included in the 14 tests above. Ready for final CI and review.
