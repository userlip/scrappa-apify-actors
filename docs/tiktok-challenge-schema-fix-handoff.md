# TikTok Challenge Details Schema Fix

The `challenge_id` schema field now uses the Apify-compatible `string` type with the `textfield` editor. Runtime `normalizeChallengeId` continues to accept safe integer values for backwards-compatible API input.

The regression test validates the current textfield/type pairing and proves that the failed legacy `['string', 'integer']` pairing is rejected under Apify editor compatibility rules.

Verification passed from `actors/tiktok-challenge-details-scraper`:

- `npm test` (17 passing)
- `npm run typecheck`
- `jq empty .actor/actor.json .actor/input_schema.json`
- `npx apify-cli validate-schema` (input and dataset schemas valid)
- `git diff --check`

Follow-up PR: https://github.com/userlip/scrappa-apify-actors/pull/272

Latest commit: `d9ba4ac` (`test: validate TikTok schema editor compatibility`)
