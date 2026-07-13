# Implementation Summary: Vinted User Profile Scraper

Date: 2026-07-13

Branch: `feat/vinted-user-profile-scraper`

## Change completed

Added `actors/vinted-user-profile-scraper`, a thin, batch-first Actor backed only by Scrappa's `/vinted/user-profile` endpoint.

- Accepts singular `user_id`, array or comma-separated `user_ids`, and optional case-normalized `country`.
- Trims, validates, deduplicates, and bounds batches at 100 IDs before network work.
- Processes requests in one run, continues after profile-level failures, and writes exactly one dataset item for each resolved public profile.
- Charges successful PPE writes only with `user-profile-result`; failed or unresolved profiles are logged without dataset/error rows or charges.
- Checks PPE capacity before each fetch and avoids all per-profile key-value-store writes.
- Maps the confirmed `success.data.user` envelope, preserving raw public fields and exposing normalized reputation, feedback, bundle, inventory, activity, verification, business, location, URL, and request metadata.
- Includes 128 MB runtime metadata, secret reference, input/dataset schemas, marketplace README examples, package/build files, and focused tests.

## Contract evidence

Scrappa's configured endpoint search and live call for user `255914028` in `DE` confirmed the endpoint name, `user_id`/`country` parameters, and the `success.data.user` response fields used by the mapper, including login, reputation, feedback sentiment, bundle discounts, item counts, activity, verification, business/holiday status, location, and profile URL.

## Local verification

From `actors/vinted-user-profile-scraper`:

```text
npm test                              # 13 passing tests
npm run typecheck                     # passes
npx --yes apify-cli validate-schema   # input and embedded dataset schemas pass
git diff --check                      # passes
```

Apify deployment, secret audit, public visibility, paid pricing activation/API verification, and live multi-ID charge-parity smoke runs were verified downstream: Actor `0z7FbFWBw77KVoabS`, build `EjLoh2HagHQAvZcjv` (`1.0.2`), active `$0.0005` `user-profile-result` pricing, and successful two-profile and mixed-success runs. This branch is now tracked by PR #281; no merge is performed in the PR stage.
