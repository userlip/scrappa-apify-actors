# Stepstone Jobs QA recovery

## Root cause

Apify automated test run `eL6cwHt0Tpi8RMxtx` failed on build `IyLXUPGxjxRxpfiPW` (`1.0.14`) in 10.934 seconds. Its valid prefilled input was:

```json
{"location":"Berlin","query":"software engineer","country":"de","sort":"relevance","page":1,"limit":25}
```

All three Scrappa requests returned HTTP 503; retry delays were 2.886 and 4.198 seconds. No dataset rows were written. This was upstream failure, not a five-minute timeout or invalid schema.

The same request and alternative queries reproduced 503 during investigation. A production service diagnostic identified `No approved proxy available ... attempt #2`. Scrappa's search limit allowed only the two VPN attempts and never reached the approved IPv4 fallback at attempt three. A diagnostic using the existing fallback returned 25 real jobs for the exact QA search. Backend fix: https://github.com/userlip/scrappa/pull/5926 (merge `85af2d32402954a899992f03a5f4844364eb7b38`). It permits three attempts within a 20-second deadline and retains the prohibition on direct scraping when proxies are unavailable.

## Actor changes

- Four requests maximum, each capped at 30 seconds; retry delays honor `Retry-After` up to 20 seconds.
- Maximum request budget: 180 seconds. With a 30-second completion reserve, 210 seconds fits within the configured 240-second Actor timeout and Apify's five-minute window.
- Preserve nested Scrappa error messages instead of reducing them to generic `Service Unavailable`.
- Retain the exact prefilled input and 25-result default; align local memory settings with the existing 128 MB deployment.

## Verification

- Actor: 27 tests passed, including repeated 503 recovery, exhausted attempts, nested errors, capped retry delays, serialization, input normalization, response mapping, and runtime budget.
- Apify CLI input and dataset schema validation passed.
- Backend: 66 Stepstone service/controller tests passed (210 assertions); Pint passed. GitHub CI was queued at release time, including earlier unrelated runs.
- Candidate Apify build `UhjsSGAFwTkSnxaPc` (`1.0.15`) succeeded. All 18 live source files match the local actor; the configured secret remains present.

- Candidate run `3djoYiAIAogITq9KP` succeeded with 25 jobs in 4.490 seconds.
- Published `latest` run `MCMoGdsgaWwXUfvTq` used build `1.0.15` and the exact original QA input with normal actor defaults (128 MB, 240 seconds). It succeeded from `2026-09-04T20:48:37.624Z` to `2026-09-04T20:48:41.182Z` (3.409 seconds reported runtime).
- Dataset `gITn6NbOR6WbhIuXD` has 25 rows, matching 25 charged `apify-default-dataset-item` events. `OUTPUT.success` is true and all 25 jobs have titles and URLs.
- The actor's maintenance notice is now `NONE`; automated testing remains enabled.

The standard backend rollout is still proceeding through the remaining origins. A final uncached verification will follow completion.
