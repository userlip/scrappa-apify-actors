# Testing Report: TikTok Challenge Details Scraper

Date: 2026-07-11

## Result

TESTING_PASSED

The focused local suite and the required deployed mixed-batch smoke test passed. The deployed Actor is configured as a thin, batch-first wrapper with an active paid event price and a secret API key.

## Local verification

Run from `actors/tiktok-challenge-details-scraper`:

| Check | Result |
| --- | --- |
| `npm test` | 18 passed, 0 failed |
| `npm run typecheck` | Passed |
| `jq empty .actor/actor.json .actor/input_schema.json` | Passed |
| `npm audit --omit=dev --package-lock-only --json` | 0 vulnerabilities |
| `git diff --check main...HEAD` | Passed |

The focused tests cover name/ID normalization and deduplication, the combined 100-entity limit, partial upstream failures, response mapping, canonical-result deduplication, recoverable short saves, and one event charge per saved dataset item.

## Deployed verification

Actor: `bEajaru9WVbLA0YBh` (`tiktok-challenge-details-scraper`)

- Actor metadata confirms the title **TikTok Hashtag & Challenge Details Scraper**, `128` MB memory, and a `120`-second timeout.
- Version `1.0` has `SCRAPPA_API_KEY` configured as a secret (value not inspected or recorded).
- Active `PAY_PER_EVENT` pricing began at `2026-07-11T18:02:19.820Z`. The primary `challenge-detail-result` event is priced at USD `$0.00025`.
- Build `1.0.6` (`NfNCbykxUqynIQ4nT`) succeeded.

Mixed smoke run: `EpVbStwLx5gJzk8Xk`

```json
{
  "challenge_names": ["booktok"],
  "challenge_ids": ["1622962893630470", "not-a-valid-id"]
}
```

| Assertion | Evidence |
| --- | --- |
| Run terminal status | `SUCCEEDED` in 4.35 seconds |
| Invalid ID behavior | Log reports it was omitted during normalization; it was not attempted or charged |
| Dataset output | Exactly one BookTok row, canonical ID `1622962893630470` |
| Summary behavior | `requested: 2`, `attempted: 2`, `saved: 1`, `failed: 0`; the ID lookup is `duplicate` and explicitly uncharged |
| Billing behavior | `chargedEventCounts.challenge-detail-result: 1` and `DATASET_WRITES: 1` |

The live result therefore satisfies the required one saved item and one paid event for the duplicate canonical BookTok lookup, while retaining the malformed ID as an uncharged per-entity normalization warning.

## Evidence sources

- Authenticated Apify actor, version, build, run, dataset, key-value-store, and log API responses.
- Local actor source at `actors/tiktok-challenge-details-scraper`.

No source changes were made during testing.
