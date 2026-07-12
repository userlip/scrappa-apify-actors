# PR Nudging Report: ImmobilienScout24 Price Insights Scraper

Date: 2026-07-13

## Pull request

- Existing implementation PR: [#274 — Add ImmobilienScout24 Price Insights Actor](https://github.com/userlip/scrappa-apify-actors/pull/274)
- Existing PR state: merged on 2026-07-12 after its CI and automated review checks passed.
- Follow-up branch: `chore/immobilienscout24-price-insights-release-verification`.
- This PR stage creates a follow-up branch for the tested metadata, regression coverage, and release evidence that were added after the original merge.
- No merge is performed in this stage.

## Validation before push

From `actors/immobilienscout24-price-insights-scraper`:

- `npm test` — 26 passed, 0 failed.
- `npm run test:dev` — 26 passed, 0 failed.
- `npm run typecheck` — passed.
- `jq empty .actor/actor.json .actor/input_schema.json` — passed.
- `npx apify-cli validate-schema` — input and dataset schemas passed.
- `npm run test:audit-health` — 17 passed, 0 failed.
- `npm run test:audit-secrets` — 19 passed, 0 failed.
- `npm run test:audit-pricing` — 14 passed, 0 failed.
- `git diff --check` — passed.

The focused suites cover array, CSV, and singular input normalization; trimming and case-insensitive deduplication; the 100-location limit; bounded concurrency; endpoint and retry parameter shape; complete response mapping; partial failures; and charge-confirmed dataset writes.

## Live release evidence

Actor `gw1ZWMNQMBu0dGUnz` (`immobilienscout24-price-insights-scraper`) has:

- Successful build `1.0.9` (`BC4VS1JHWLSqLyDls`), 128 MB memory, and a 300-second timeout.
- `SCRAPPA_API_KEY` configured as an Apify secret.
- Active `PAY_PER_EVENT` pricing at `$0.0005` for `price-insight-result`.

Berlin/Munich batch run `PKs7fmAHPsPpPVhsh` produced two complete EUR dataset rows and exactly two charged events. Mixed valid/invalid run `h4gumcKUWI1YRBABi` produced one Berlin row, one charged event, and skipped the invalid location without charging it. The default key-value store contained only `INPUT`; no per-item `OUTPUT` records were written.

Portfolio audits reported 92/92 actors with secrets and active paid pricing. The health audit's only unrelated failure was `tiktok-challenge-posts-scraper` (`CVaJEgPjl3jWKbm71`); the new Actor had five recent successful runs and a successful latest build.

## CI and review monitoring

The original implementation PR #274 completed all repository Actor Tests, Claude review, Cubic review, and Socket project checks successfully before merge. The follow-up PR will be monitored until its checks and review are clean. Any feedback requiring unrelated implementation work will be returned to implementation rather than expanded here.

## Outcome

The tested follow-up changes are ready for review. No merge was performed.
