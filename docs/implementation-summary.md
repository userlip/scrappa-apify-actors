# Implementation Summary: TikTok Challenge Details Scraper

Date: 2026-07-11

## Rework completed locally

- Fixed the Apify deployment blocker for `challenge_id`: the input schema now declares a single `string` type for its `textfield` editor, which Apify accepts.
- Kept backwards-compatible numeric single-ID input in the runtime normalizer (`normalizeChallengeId`), so API clients sending safe integer values continue to work even though the UI schema field is a string.
- Added a focused regression test that locks the Apify-compatible `textfield`/single-string pairing.

## Original implementation

- Created `actors/tiktok-challenge-details-scraper` on branch `feat/tiktok-challenge-details-scraper`.
- Added batch-first name and ID normalization with mixed-input support, separate deduplication, malformed-entry warnings, and a strict combined 100-entity cap.
- Added a thin Scrappa `GET /tiktok/challenges/details` wrapper. It processes all normalized entities within one Apify run, retains successful items after per-entity API/empty-response failures, and writes one compact `OUTPUT` summary.
- Added `challenge-detail-result` PAY_PER_EVENT handling: capacity is checked before every request and only successful saved records are charged.
- Added normalized detail fields (stable IDs/names, nullable metrics, cover, provenance, UTC retrieval timestamp) while preserving source fields, plus listing/schema/readme metadata and dataset view.
- Added focused tests for schema, normalization, endpoint request shape, response mapping, partial failure, development/PAY_PER_EVENT save paths, and charging limits.

## Local verification

From `actors/tiktok-challenge-details-scraper`:

```text
npm test          # 15 passing tests
npm run typecheck # passes
jq empty .actor/actor.json .actor/input_schema.json # passes
npx apify-cli validate-schema # input and dataset schemas pass Apify CLI validation
git diff --check  # passes
```

## Release handoff

## Deployment and release verification

- Deployed build `NfNCbykxUqynIQ4nT` (`1.0.6`) succeeded after publishing the committed canonical-deduplication fix.
- Actor `bEajaru9WVbLA0YBh` remains private. Its deployed `SCRAPPA_API_KEY` is present as a secret, its resources are 128 MB / 120 seconds, and active `PAY_PER_EVENT` pricing started at `2026-07-11T18:02:19.820Z`.
- The active primary event is `challenge-detail-result` at USD `$0.00025` per saved result.
- Mixed smoke run `Y6BdN3NgAhM5k0W54` succeeded using `booktok`, the matching stable ID `1622962893630470`, and an unavailable challenge name. It saved one canonical BookTok dataset item and charged exactly one event; the unavailable name produced a safe per-entity error, and the duplicate ID was reported as uncharged.

The prior smoke run exposed that its older live build had not yet included canonical-result deduplication and charged twice. The corrected build above resolves that production defect. The Actor remains private; publication and any PR remain downstream actions.

The requested new PR is intentionally not opened here: this stage is constrained to a local commit, and the PR stage follows testing.

The feature implementation, deployment-blocker correction, and charging-path test coverage are committed locally on `feat/tiktok-challenge-details-scraper` (`5cd31ea`). The branch is intentionally not pushed and no PR is opened by this implementation stage; the follow-up PR stage owns those external actions.

IMPLEMENTATION_COMPLETE
