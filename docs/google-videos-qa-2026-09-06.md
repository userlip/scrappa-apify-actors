# Google Videos Actor QA repair — 2026-09-06

Actor: `thescrappa/google-videos-scraper` / `kAdTwn5fkBCGKOQUq`.

## Failure and root cause

[Failed QA run](https://console.apify.com/view/runs/kE2PONgdKGSuTnAJ8): `FAILED`, build `1.0.6` / `D2UMFllFQSWYtW5cX`, origin `TEST`, 128 MB, 300-second timeout. It started at `2026-09-05T20:38:32.006Z` and finished at `20:38:56.626Z` (24.620 seconds wall time). This was an upstream error, not a five-minute timeout.

The input record was:

```json
{"q":"coffee brewing tutorial","queries":["coffee brewing tutorial","espresso machine review"],"hl":"en","gl":"us","google_domain":"google.com","safe":"off"}
```

The [run log](https://api.apify.com/v2/logs/kE2PONgdKGSuTnAJ8) shows:

- `20:38:42.794Z`: the first query saved 10 results.
- `20:38:42.799Z`: the second query started.
- `20:38:55.222Z`: `Actor failed: Scrappa API error (503): HTTP 503`.

The client made a single attempt and the Actor failed the whole run on the second query's transient HTTP 503. Existing results do not satisfy QA when the final status is FAILED. The upstream reason for this specific 503 is not exposed by the log; this investigation does not establish a backend outage cause. Both queries returned results during current validation.

An additional timeout defect was found: returning `response.json()` without awaiting it cleared the abort timer before the response body finished. A stalled body could therefore exceed the intended 60-second attempt budget.

## Changes

- Retry GET requests on HTTP 408/429/500/502/503/504, request timeouts, or fetch connection failures. Three total attempts with 1- and 2-second delays; permanent errors and exhausted retries still fail. POST behavior is unchanged.
- Await JSON decoding before clearing the request timeout.
- Prefill only `queries: ["coffee brewing tutorial"]`, removing the redundant legacy `q` prefill. Keep legacy single-query and ten-query batch support.
- Preserve Scrappa-side scraping, 128 MB Actor resources, dataset output, and charging behavior. No synthetic rows or success masking.

The one-query QA input spends at most approximately `3 × 60 + 1 + 2 = 183` seconds on API attempts, leaving about 117 seconds of the QA allowance for startup and storage. Customer batches may take longer than QA; the existing local manifest's 720-second timeout remains unchanged. Live metadata currently reports a 120-second default timeout, so release work should reconcile that separately from QA's explicit 300-second override.

## Validation

- `cd actors/google-videos-scraper && npm ci --no-audit --no-fund && npm test`: TypeScript build and all 17 tests pass.
- Re-ran the 503 recovery test against the original client from repository HEAD: it fails immediately with the exact `Scrappa API error (503): HTTP 503` error. The patched client passes the same scenario with one returned result and two HTTP calls.
- Regression tests also confirm three-attempt exhaustion, no retries for HTTP 401, timeout coverage for a stalled HTTP 200 response body, and one effective prefilled query with batch support retained.
- Full patched Actor executed locally against the real Scrappa API with the new exact schema prefill: exit 0, 10 default dataset items, 1.483 seconds.
- Full patched Actor executed locally against the real Scrappa API with the original failed-run input: exit 0, 20 default dataset items, 2.461 seconds.
- Local full-Actor checks used Node 24.18.0 and Apify SDK 3.7.2; the deployed container uses Node 20.20.2. These measurements exclude Apify container startup and may benefit from upstream caching.
- A Python urllib probe received HTTP 403; Node fetch, which is the Actor's actual HTTP client, and both complete Actor executions succeeded. The Python result was not treated as an Actor reproduction.

## Release completed

The fix is pushed to `main` as `af933af`. Authenticated comparison confirmed the live implementation matched the pre-repair local code. Updated only the reviewed source/schema/test/documentation files, preserving the existing secret `SCRAPPA_API_KEY`, pricing, memory, and default run settings.

The earlier credential blocker was an investigation error: the token is available via Apify CLI's installed `@napi-rs/keyring` module, using service `com.apify.cli` and entry `token`. No new token was needed and no secret values were committed or printed.

- Built version `1.0` with validation tag `qa-repair`: build `1.0.7` / `i1wSbfAmjmFIdUAIS`, SUCCEEDED.
- [Exact-prefill hosted run](https://console.apify.com/view/runs/q7bFH3fYvwSKXiRZ2): SUCCEEDED, 10 default dataset items, 3.563 seconds wall time, 128 MB, 300-second timeout.
- Promoted the passing build to `latest`.
- [Original two-query input on latest](https://console.apify.com/view/runs/7vlQHXb24hTGoNaui): SUCCEEDED, 20 default dataset items, 3.681 seconds wall time, confirmed build `i1wSbfAmjmFIdUAIS`.
- After the passing prefill test, cleared the stale maintenance notice through the Actor API. A separate authenticated read confirmed `notice: NONE` and `latest: 1.0.7`; secret metadata still confirms `SCRAPPA_API_KEY` is secret.

The Actor is now out of maintenance and the published build meets the tested QA criteria. These are developer-triggered hosted validation runs, not a claim that Apify's next scheduled automated test has already executed. Persistent upstream unavailability can still cause a legitimate future failure; no testing exemption or synthetic success was used.

[Apify's testing documentation](https://docs.apify.com/actors/publishing/test) requires SUCCEEDED plus a non-empty default dataset within five minutes. Both hosted checks passed those criteria.
