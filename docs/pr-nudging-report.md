# PR Nudging Report: TikTok Challenge Details Scraper

Date: 2026-07-11

## Pull request

- PR: [#272 — fix: make TikTok challenge ID schema Apify-compatible](https://github.com/userlip/scrappa-apify-actors/pull/272)
- Branch: `fix/tiktok-challenge-details-input-schema`
- Head: `85d74d3183bb72201a8c3ce4dae2dec2b5c30288`
- Base: `main`
- State: open, ready for merge (`mergeStateStatus: CLEAN`); this stage did not merge it.

This is the follow-up to the initial Actor PR. It makes the Apify input schema editor-compatible and prevents name or ID lookups from saving or charging a mismatched upstream challenge response. It also deduplicates canonical results and retries a recoverable short save without double charging.

## Validation before push

Run from `actors/tiktok-challenge-details-scraper`:

- `npm test` — 18 passed, 0 failed.
- `npm run typecheck` — passed.
- `jq empty .actor/actor.json .actor/input_schema.json` — passed.
- `npm audit --omit=dev --package-lock-only --json` — 0 vulnerabilities.
- `git diff --check origin/main...HEAD` — passed.

The focused suite covers normalization and the combined 100-entity limit, partial failures, response mapping, canonical deduplication, mismatch rejection, short-save retries, and one charge per saved result.

## Release gate evidence

The deployed Actor `bEajaru9WVbLA0YBh` (`tiktok-challenge-details-scraper`) has a secret `SCRAPPA_API_KEY` and active `PAY_PER_EVENT` pricing of USD `$0.00025` for `challenge-detail-result`.

Mixed live smoke run `EpVbStwLx5gJzk8Xk` succeeded in 4.35 seconds: the `booktok` name plus its canonical ID produced one BookTok dataset item and exactly one charged event; the canonical duplicate was explicitly uncharged and the malformed ID was omitted during normalization. Build `1.0.6` (`NfNCbykxUqynIQ4nT`) succeeded. Full evidence is in `docs/testing-report.md`.

## CI and review monitoring

After pushing the final tested head, all PR checks passed:

- Actor Tests workflow `29163099479`: all 22 matrix jobs passed.
- Claude Code Review workflow `29163099505`: passed.
- Cubic AI code reviewer: passed.
- Socket Security project report and PR alerts: passed.

The repository CI matrix does not yet enumerate this newly added Actor; its dedicated test command was run locally as the focused gate above. GitHub's REST endpoint temporarily rate-limited direct review-comment enumeration, but both configured automated reviewers completed successfully and no failing or actionable review check remains.

## Outcome

The branch is pushed, PR #272 is open and clean, all rerun CI/review checks pass, and paid live behavior is verified. No merge was performed.

PR_NUDGING_PASSED
