# Implementation Brief: ImmobilienScout24 Price Insights Scraper

## Status and context

Create and release the public paid Actor `immobilienscout24-price-insights-scraper`, titled **ImmobilienScout24 Price Insights Scraper**. The Actor must be a thin Apify wrapper around Scrappa's `GET /api/immobilienscout24/price-insights` endpoint; scraping remains on Scrappa infrastructure.

The target directory is already present in this checkout and contains the proposed implementation and focused tests. Intake therefore treats the remaining work as implementation verification, correction where a gate exposes a defect, deployment, monetization activation, and live verification. No failed downstream addendum was available in the canvas inbox. Existing repository reports concern other Actors and are not evidence for this Actor.

Reference implementation to preserve:

- `actors/immobilienscout24-search-scraper/src/main.ts` and `src/shared/scrappa-client.ts` for Actor lifecycle, API key handling, retries, timeout, and failure conventions.
- `actors/immobilienscout24-price-insights-scraper/src/main.ts` for the Actor entrypoint and safe final `Actor.fail` path.
- `src/request-params.ts` for batch-first input normalization.
- `src/response-utils.ts` for the resolved-location and four-benchmark output contract.
- `src/batch-runner.ts` for bounded concurrent fetches, partial failure, dataset writes, and `price-insight-result` charging.

## Scope

Work only in `actors/immobilienscout24-price-insights-scraper` and its release metadata/docs as needed:

- Keep `locations` as the primary input. Accept an array of strings or comma-separated text, retain singular `location` compatibility, trim values, ignore blank entries, deduplicate case-insensitively, and enforce a bounded batch size.
- Resolve each normalized location through Scrappa's price-insights endpoint in one Apify run. Do not call ImmobilienScout24 directly or add scraping logic to the Actor.
- Emit one dataset item only for a complete successful snapshot. The item must include resolved `location`, `geocode`, `currency`, and positive numeric `apartment_rent_per_m2`, `apartment_buy_per_m2`, `house_rent_per_m2`, and `house_buy_per_m2`; retaining the requested location/index is acceptable for traceability.
- Continue after an individual API error or incomplete response. Failed locations must be logged, produce no dataset item, and produce no paid event.
- Use PAY_PER_EVENT event `price-insight-result` at the proposed USD `$0.0005` per successfully saved location snapshot. Count a result as successful only after the Apify write/charge response confirms it.
- Do not write per-item `OUTPUT` key-value records. Dataset output is the primary result channel.
- Keep the Actor resource profile at 128 MB and use a minimal timeout appropriate for the bounded batch; resolve the current `.actor/actor.json` timeout setting during implementation/release review rather than increasing resources.
- Keep the README, `.actor/README.md`, input schema, actor title, dataset view, package metadata, and tests aligned with the actual contract and Scrappa direct-API upgrade path.

Out of scope: direct ImmobilienScout24 scraping, new Scrappa backend behavior, unrelated Actor refactors, portfolio-wide pricing changes, secret rotation, Actor deletion, or merging/publishing a PR from the implementation stage.

## Acceptance criteria

1. `locations: [" Berlin ", "Munich", "berlin"]` results in exactly two attempted locations, preserving first-seen display values; `locations: "Berlin, Munich"` works; singular `location: "Hamburg"` remains compatible; missing/invalid/over-limit input fails before network work.
2. A single Apify run handles a multi-location batch and emits at most one dataset item per normalized location. Successful output contains the resolved geocode, currency, and all four apartment/house rent/buy price-per-square-meter benchmarks.
3. Invalid, unavailable, incomplete, or failed locations do not abort remaining locations, do not write dataset rows, and do not charge `price-insight-result`. If every location fails, the run fails clearly; if some succeed, the run exits with an explicit partial-success summary.
4. The only upstream data request is Scrappa's `/immobilienscout24/price-insights` endpoint, using the shared API-key header, timeout, retry, and error conventions. No raw API key is committed or logged.
5. PAY_PER_EVENT writes use exactly `price-insight-result` and the proposed `$0.0005` price. The implementation does not claim a saved/charged result unless Apify confirms the charge. Charge-limit and dataset-write errors are handled without silently charging failed locations.
6. Actor metadata has title `ImmobilienScout24 Price Insights Scraper`, memory `128` MB, a bounded batch-oriented input schema, the dataset view, and a release-ready description targeting property-market benchmarking, rent-vs-buy research, and recurring city comparison. README pricing copy matches the configured event.
7. The Actor version has `SCRAPPA_API_KEY` configured as an Apify secret, a successful build, and a successful two-location live smoke run. Live evidence shows dataset-item count equals the number of confirmed successful charges; a mixed valid/unavailable batch demonstrates partial-failure behavior.
8. Before public release, Apify API metadata verifies active paid pricing or the earliest allowed scheduled paid pricing for this exact Actor. If Apify blocks immediate pricing, record the exact actor slug/ID, blocker, earliest activation timestamp, and follow-up action; do not treat listing copy as monetization evidence.

## Applicable repository rules

The binding `AGENTS.md` and `CLAUDE.md` both state:

> Scraping must stay on Scrappa infrastructure. Apify actors in this repo should be thin marketplace wrappers around Scrappa API endpoints, not the place where heavy scraping work runs.

> Accept multiple URLs/entities in a single Apify run whenever the upstream Scrappa API supports it, or loop through the provided list inside one run when it does not.

> Push one dataset item per processed URL/entity so Apify monetization still charges per result.

> Keep Apify memory and timeout settings minimal for wrapper actors. The actor should mostly validate input, call `https://scrappa.co/api`, and write dataset items.

> Avoid unnecessary key-value store writes per item. Use dataset output as the primary result channel unless the actor has a clear compatibility reason to write `OUTPUT`.

The same files also require the final Actor failure path to use `Actor.fail(message)` rather than leaving an uncaught exception. The implementation must preserve that convention and must not expose credentials in source, docs, logs, handoffs, or screenshots. Monetization guidance in the assigned proposal is additionally binding: never publish a public Scrappa Actor as free, treat missing/disabled/future-only paid pricing as a P0 release issue, and verify pricing through Apify API/Console.

## Testing plan

Run from `actors/immobilienscout24-price-insights-scraper`:

- `npm test` to build and run all focused tests.
- `npm run typecheck` for strict TypeScript validation.
- Exercise normalization tests for array, CSV, singular compatibility, trimming, case-insensitive deduplication, blank values, invalid types, length, and maximum batch size.
- Exercise response mapping tests for the complete live-shaped response, missing geocode/currency/prices, each missing benchmark, non-numeric/non-positive values, and preservation of request metadata.
- Exercise batch-runner tests for endpoint/parameter shape, bounded multi-location processing, input-order dataset writes, per-location API/incomplete-response failures, retryable Scrappa errors, partial success, all-failure behavior, PAY_PER_EVENT event name, charge confirmation, charge limits, and propagation of dataset/charging failures.
- Validate JSON metadata with `jq empty .actor/actor.json .actor/input_schema.json` and, where available in the release environment, `npx apify-cli validate-schema`.
- Run relevant repository audits before release: `npm run audit:health`, `npm run audit:secrets`, and `npm run audit:pricing` (or their JSON forms) without recording secret values. Run the focused repository test/audit suites required by CI.
- Run `git diff --check` and inspect the final diff for accidental credentials, direct third-party scraping, per-item KV writes, or unrelated changes.

## Deploy and live-verification plan

1. From a new implementation branch/PR, build and push the Actor source with Apify CLI. Do not merge or publish from intake; downstream implementation/release stages own those actions.
2. Confirm the deployed Actor's version/build is successful, `SCRAPPA_API_KEY` is present as a secret, memory is 128 MB, timeout is bounded, and the source is the intended build.
3. Configure PAY_PER_EVENT pricing for `price-insight-result` at USD `$0.0005` per event. Query `GET /v2/acts/{actorId}` and the relevant pricing metadata after configuration. Verify effective `pricingInfos`/`currentPricingInfo` or the earliest scheduled activation, not README text.
4. Smoke-run at least two known locations, e.g. Berlin and Munich. Verify the run succeeds, the dataset has one complete item per resolved location, the four numeric benchmarks and geocodes are present, and charged-event count equals dataset-item count.
5. Run a mixed batch containing a known valid location and a deliberately unavailable/narrow location. Verify the valid row remains, the failed location is logged/skipped, and only the valid row produces a `price-insight-result` charge. If a known unavailable fixture is not available, use a controlled mocked/local runner for the partial-failure assertion and document that limitation.
6. Check the run log for absence of raw `SCRAPPA_API_KEY`, direct ImmobilienScout24 requests, uncaught exceptions, and unexpected `OUTPUT` writes. Record actor ID, build ID, run IDs, dataset IDs, pricing evidence, and timestamps in the downstream release report without recording secrets.

## Rollback plan

- If build, schema, endpoint, output, or billing behavior is wrong, keep the last known-good Actor version/build available and stop public release. Repoint the Actor to the previous version or deploy a corrected branch/PR; do not delete the Actor or rewrite history.
- If pricing is missing, disabled, incorrectly priced, or attached to the wrong event, keep the Actor private or disable publication until corrected/scheduled at the earliest allowed Apify date. Record the API blocker and do not represent the Actor as monetized.
- If live smoke reveals overcharging or dataset/charge mismatch, stop further smoke/customer runs, preserve run evidence, and revert to the last known-good build before changing pricing or code. Investigate whether duplicate writes, charge confirmation, or charge-limit handling caused the mismatch.
- If only a location fails, retain the partial-failure behavior and investigate the Scrappa response contract; do not broaden the wrapper to scrape around the endpoint.

## Risks and mitigations

- Narrow or unsupported locations may not resolve. Mitigate with per-location failure isolation, clear logs, zero charge/zero dataset row for failures, and a valid-location smoke fixture.
- The upstream benchmark names, nesting, currency, or definitions may evolve. Keep the mapper strict enough to avoid charging incomplete snapshots, test the observed live response shape, and revalidate against Scrappa before release.
- Batch concurrency can increase upstream pressure or make Apify event limits visible mid-run. Keep a bounded concurrency/batch size, preserve deterministic write order, and stop cleanly on charge limits.
- A timeout that is too generous increases Apify overhead, while one that is too short can fail large valid batches. Keep memory at 128 MB and choose/verify a bounded timeout against the maximum normalized batch and retry policy.
- Apify may delay pricing activation or expose pricing in `pricingInfos` without a populated `currentPricingInfo`. Treat the API's effective/scheduled timestamp as the source of truth and block public release when no active or earliest-scheduled paid state exists.
- The proposed price is billing-sensitive and must be verified against actual charged event counts. Keep the event name stable and compare every successful smoke dataset row with exactly one charge.

## Handoff

Implementation may proceed after confirming the existing target files satisfy this contract. Any code correction must be made on a new branch and delivered through a new PR as requested. The implementation handoff must include the local test output, exact files changed, build/deploy evidence, pricing API evidence, and a live charge-parity result.
