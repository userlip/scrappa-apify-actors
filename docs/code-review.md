# Code Review: TikTok Challenge Details Scraper

## Result

No blocking findings. The new Actor is a thin, batch-first Scrappa wrapper and conforms to the repository cost-control rules.

## Evidence reviewed

- Diff reviewed: `913e3fc..HEAD`, including the new `actors/tiktok-challenge-details-scraper` implementation and focused tests.
- The Actor normalizes and deduplicates mixed name/ID inputs, enforces the combined 100-entity limit before network work, sends exactly one lookup parameter per entity, and processes all entities in a single run.
- Each successful detail lookup writes one dataset item. PAY_PER_EVENT uses `challenge-detail-result`, checks capacity before fetching, and retains later successes after per-entity failures. Failed and empty lookups are retained as safe per-entity `OUTPUT` outcomes without a charged dataset row.
- Reviewed the Scrappa API contract in the reference API checkout: `GET /api/tiktok/challenges/details` accepts `challenge_id` or `challenge_name`, proxies `/challenge/info`, and returns the implemented metadata shape.
- Metadata uses the required title, secret reference, 128 MB memory, 120-second timeout, batch-first schema, dataset view, and listing examples. No API key is present in the diff.
- Local verification passed in `actors/tiktok-challenge-details-scraper`:
  - `npm test`: 13 passing
  - `npm run typecheck`: passed
  - `jq empty .actor/actor.json .actor/input_schema.json`: passed
  - `git diff --check 913e3fc..HEAD`: passed

## Release gates outside this code review

Before public release, the deployment stages must configure `SCRAPPA_API_KEY` as a secret; create and API-verify active or earliest-scheduled paid PAY_PER_EVENT pricing for `challenge-detail-result` at USD `$0.00025`; and complete the mixed name/ID smoke run with event-to-dataset accounting and a partial-failure check.

CODE_REVIEW_PASSED
