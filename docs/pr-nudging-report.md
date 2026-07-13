# PR Nudging Report: Vinted User Profile Scraper

Date: 2026-07-13

## Pull request

- Branch: `feat/vinted-user-profile-scraper`
- PR: pending creation from the pushed branch
- Merge: not performed in this stage

## Validation before push

From `actors/vinted-user-profile-scraper`:

- `npm test` — 12 passed, 0 failed.
- `npm run typecheck` — passed.
- `npx --yes apify-cli validate-schema` — input and embedded dataset schemas passed.
- `git diff --check origin/main...HEAD` — passed.
- `git diff --check` — passed.

The focused tests cover singular, array, CSV, trimming, deduplication, country normalization, validation limits, response mapping, unresolved profiles, partial failures, transient retries, exact endpoint selection, PPE event naming, charge-capacity handling, and successful-save semantics.

## Release evidence

Actor `0z7FbFWBw77KVoabS` (`thescrappa/vinted-user-profile-scraper`) has:

- Successful build `EjLoh2HagHQAvZcjv` (`1.0.2`) with 128 MB runtime metadata.
- Public listing title `Vinted User Profile Scraper`.
- `SCRAPPA_API_KEY` configured as an Apify secret.
- Active `PAY_PER_EVENT` pricing at `$0.0005` for `user-profile-result`.

Two-profile batch `ROx2hd3RNGGtwBmkH` produced two dataset rows and two charges. Mixed-success batch `h52knnaDY5qxHvTS6` produced one valid dataset row and one charge after an invalid first ID. Both runs wrote only `INPUT` to the default key-value store.

## CI and review monitoring

CI and review status will be recorded here after the PR is created. Actionable feedback will be addressed only when it is within this scoped Actor change; broader implementation changes will be returned to implementation. No merge is permitted in this stage.

## Outcome

The tested implementation is ready for PR review and downstream merge/deploy handling.
