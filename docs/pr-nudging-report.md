# PR Nudging Report: Vinted User Profile Scraper

Date: 2026-07-13

## Pull request

- Branch: `feat/vinted-user-profile-scraper`
- PR: [#281 — feat: add Vinted user profile scraper](https://github.com/userlip/scrappa-apify-actors/pull/281)
- Merge: not performed in this stage

## Validation before push

From `actors/vinted-user-profile-scraper`:

- `npm test` — 16 passed, 0 failed.
- `npm run typecheck` — passed.
- `npx --yes apify-cli validate-schema` — input and embedded dataset schemas passed.
- `git diff --check origin/main...HEAD` — passed.
- `git diff --check` — passed.

The focused tests cover singular, array, CSV, trimming, deduplication, country normalization, positive-ID validation, response mapping, unresolved profiles, partial failures, transient retries, auth classification, abort-to-timeout wrapping, exact endpoint selection, PPE event naming, charge-capacity handling, saved/uncharged edge cases, and successful-save semantics.

## Release evidence

Actor `0z7FbFWBw77KVoabS` (`thescrappa/vinted-user-profile-scraper`) has:

- Successful build `EjLoh2HagHQAvZcjv` (`1.0.2`) with 128 MB runtime metadata.
- Public listing title `Vinted User Profile Scraper`.
- `SCRAPPA_API_KEY` configured as an Apify secret.
- Active `PAY_PER_EVENT` pricing at `$0.0005` for `user-profile-result`.

Two-profile batch `ROx2hd3RNGGtwBmkH` produced two dataset rows and two charges. Mixed-success batch `h52knnaDY5qxHvTS6` produced one valid dataset row and one charge after an invalid first ID. Both runs wrote only `INPUT` to the default key-value store.

## CI and review monitoring

- All repository Actor Tests passed.
- Claude review passed after three rounds; all substantive findings were addressed in `e6049db`, `51111e4`, and `0390bc6`.
- Cubic review passed after the scoped follow-up fixes.
- Socket Project Report passed; Socket Pull Request Alerts was skipped/neutral due to the dependency-scan configuration.
- Remaining reviewer observations are non-blocking follow-ups: spot-check Apify Console rendering for the union-typed `user_ids` field, consider shared Vinted input helpers, and add low-level network-error retry tests in a later maintenance change.
- No merge was performed in this stage.

## Outcome

The tested implementation is in PR #281 with green CI and approving reviews, ready for downstream merge/deploy handling.
