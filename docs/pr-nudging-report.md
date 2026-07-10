# PR Nudging Report: Kleinanzeigen Listing Details Scraper

## Release gates

- Branch: `feat/kleinanzeigen-listing-details-scraper`
- Deployed Apify actor: `kleinanzeigen-listing-details-scraper` (`1hSNdgPwGINp7xeHB`), public.
- Cloud build: `4jb5DP8IvTrkymaHy` (`1.0.1`), succeeded.
- Scrappa API key: configured as the encrypted `SCRAPPA_API_KEY` secret on version `1.0`.
- Pricing: active `PAY_PER_EVENT` from `2026-07-10T11:19:59.585Z`; `listing-detail-result` is the primary event at `$0.00025` per successful listing.

## Live verification

Private pre-publication smoke run `pcPHpHu5YeCqyppa2` supplied two distinct ad IDs in one run (`3348371956`, `3401748259`). It succeeded, saved two dataset items, and its `OUTPUT` record reported two requested, completed, and saved listings with zero failures. Finalized Apify run accounting reported exactly two `listing-detail-result` events.

## Local verification

- `npm --prefix actors/kleinanzeigen-listing-details-scraper test` — passed (13 tests).
- `npm --prefix actors/kleinanzeigen-listing-details-scraper run typecheck` — passed.
- `git diff --check main...HEAD` — passed.

## Repository inventory

`README.md` now contains the new live actor, actor ID, deployment/build evidence, smoke-run evidence, and active pricing status, as required by `CONTRIBUTING.md`.

## PR status

Ready to push and open for review. No implementation defects or review feedback remain.
