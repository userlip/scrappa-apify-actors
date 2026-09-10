# Kleinanzeigen details maintenance recovery

Actor: `thescrappa/kleinanzeigen-listing-details-scraper` (`1hSNdgPwGINp7xeHB`).
Task: `d3180835-21d7-488a-afcc-6d2ef3b12ecc`; repository ID 18. The configured checkout and Mimir repository record use `userlip/scrappa-apify-actors` (the request also calls it `Scrappa-co/scrappa-apify-actors`).

## Confirmed cause

[Run jS4zl8zaBwqtWeace](https://console.apify.com/view/runs/jS4zl8zaBwqtWeace) started September 10, 2026 at 17:30:29.985 UTC and finished at 17:30:34.919 UTC, FAILED, on build `1.0.5` (`UlmHQl6bFF967T9Kf`). Origin: TEST; memory: 128 MB; timeout: 300 seconds.

Its stored INPUT was `{"ad_id":"3451021120","ad_ids":["3451021120"]}`. Deduplication produced one request. The log records HTTP 404: “The requested ad does not exist or has been removed.” OUTPUT confirms one failure, zero saved listings; default dataset `fIRBrIRaGoMslBaSy` is empty and zero listing events were charged. This is an expired prefill, not a five-minute timeout, missing credential, or charging failure.

The notification reports three days of failed QA. Only the supplied run was independently verified: listing the actor's previous runs requires an Apify token, unavailable in this session. Do not represent all three daily logs as inspected.

The deployed build already fails all-error batches, whereas the starting local source silently exited successfully. The patch preserves that observed live behavior after writing OUTPUT; compare the remaining live source with the checkout before replacing the deployed version.

## Change

Remove both hardcoded ID prefills. Prefill only `{"query":"fahrrad"}`. When no explicit IDs are supplied, search the first page through Scrappa, deduplicate numeric IDs, and try at most three current listings until one detail is saved. Never save or charge search results as details. Explicit ID inputs retain batching and do not trigger discovery or fall back to unrelated listings.

Discovery uses one search plus at most three detail requests, each with a 60-second timeout and no retries: at most 240 seconds of HTTP work, leaving approximately 60 seconds for Apify startup/storage within QA's 300-second limit. Upstream outages or three unavailable candidates still fail honestly. Existing explicit-ID timeout/retry behavior is unchanged.

## Validation

- `npm ci --ignore-scripts --no-audit --no-fund` in the actor directory.
- `npm test`: 17 passed, including build and a mocked run of the actual main entrypoint using schema-derived prefill, an expired first candidate, one charged successful detail, and all-candidates-removed failure.
- `npm run test:dev`: 17 passed against TypeScript source.
- `npm run typecheck`: passed.
- `git diff --check`: passed.
- A live Scrappa search using the actor's HTTP client returned current IDs in 2.436 seconds. A preliminary Python urllib probe returned 403; the actual client succeeded using the same local credential, so that probe was not evidence of invalid credentials.

The full compiled actor was then run locally with real Scrappa requests and the exact schema-derived prefill. It exited 0. Started at `2026-09-10T20:27:04.725Z`; its dataset row was written at `20:27:08.509Z` (3.784 seconds). OUTPUT: three candidates, one completed, one saved, zero failures. Saved listing ID: `3498053111`, with a non-empty title and matching `request_ad_id`. This validates actual search-to-details parsing and local SDK storage; paid charging is covered by the mocked entrypoint test, not this local run. The local smoke harness uses `CRAWLEE_STORAGE_DIR` for SDK 3 storage.

## Deployment and console follow-up

1. Review the current Apify source against this checkout (live build drift noted above), then upload the patched actor directory, including `.actor/input_schema.json`, and rebuild version 1.0 tagged `latest`. Deploying only the new schema to the old implementation will fail because the old code requires IDs. No Apify build or version was changed in this session.
2. Preserve the secret `SCRAPPA_API_KEY` setting; do not replace it with a literal secret in source. The live failed run successfully reached the API, and the local actor client authenticated successfully. Keep the `listing-detail-result` paid event and 128 MB memory.
3. Run the new build with exactly `{"query":"fahrrad"}`, 300-second timeout, and enough event budget for one result. Clear old saved `ad_id`/`ad_ids` fields in the console; explicit IDs intentionally take priority over query. Verify SUCCEEDED, at least one genuine detail dataset row, and total run duration below 300 seconds. A local live run does not establish cloud build success.
4. Apify's [automated testing documentation](https://docs.apify.com/actors/publishing/test) requires SUCCEEDED plus a non-empty default dataset within five minutes. After rebuilding, automatic tests should pick up the fix and restore healthy status within 24 hours. Verify the actual badge/test outcome; do not manually claim recovery based on local tests. There is no need to opt out of QA or contact support for this expired-input failure.
