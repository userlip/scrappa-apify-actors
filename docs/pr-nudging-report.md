# PR Nudging Report: TikTok Challenge Details Scraper

Date: 2026-07-11

## Pre-PR validation

- Branch: `feat/tiktok-challenge-details-scraper`.
- Reviewed commits: `58e7bda`, `2f0e4c5`, and `5b621e7` after base `913e3fc`.
- Focused checks, rerun in `actors/tiktok-challenge-details-scraper`, passed:
  - `npm test`: 13 passed, 0 failed.
  - `npm run typecheck`: passed.
  - `jq empty .actor/actor.json .actor/input_schema.json`: passed.
  - `git diff --check 913e3fc..HEAD`: passed.
- Code review found no blocking findings. The Actor is a thin, batch-first Scrappa wrapper and produces one charged dataset item per successful challenge lookup.

## PR scope

The change adds `actors/tiktok-challenge-details-scraper`, a batch-first Actor for resolving known TikTok challenge names and IDs using Scrappa's `/tiktok/challenges/details` endpoint. It accepts up to 100 normalized, deduplicated entities in one run; retains partial successes; writes clear per-entity outcomes; and uses the `challenge-detail-result` PAY_PER_EVENT event only for successful saved items.

The marketplace title is **TikTok Hashtag & Challenge Details Scraper**. The listing and schema include campaign-research, user/view-count, and challenge-resolution examples.

## Release gates for deployment

The PR contains no production deployment, secret, or Apify pricing mutation. Before public release, the deployment and live-verification stages must:

1. Configure `SCRAPPA_API_KEY` as a secret for the deployed version.
2. Create and API-verify active, or earliest-scheduled, paid `PAY_PER_EVENT` pricing for `challenge-detail-result` at USD `$0.00025` per successful item.
3. Run a mixed `booktok`, `1622962893630470`, and known-invalid batch; verify terminal success, per-entity failure reporting, one dataset item and one event per resolved challenge, and no charge for the invalid lookup.

## PR status

The branch is ready to push and open as a pull request against `main`. CI and review monitoring begin after the PR is created. This stage does not merge.

## Outcome

PR_NUDGING_PASSED
