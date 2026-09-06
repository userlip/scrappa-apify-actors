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

## Release status and recommendation

**Local fix validated; deployment and hosted QA verification are blocked on Apify API access.** No live source, build tag, notice, pricing, or settings were changed. The accessible local Apify profile stores its token in an unavailable keyring, and no `APIFY_TOKEN` is configured. Public APIs supplied run metadata, input, log, and Actor metadata, but not authenticated version source. Source parity with the deployed version has therefore not been verified.

At inspection, live `latest` remains `1.0.6` and the Actor still reports `UNDER_MAINTENANCE`. It is not accurate to claim the notice has cleared or the deployed Actor has been repaired.

Once organization API access is available:

1. Read and compare live version `1.0` source; apply the reviewed changes while preserving its secret configuration and any unrelated live edits.
2. Build the updated version and run that build with its exact prefilled schema input, a 300-second timeout, and 128 MB. Require SUCCEEDED and a non-empty default dataset before promoting it to `latest`.
3. Confirm `latest` references the passing build, then verify the automated health result. Do not manually clear the notice as a substitute for testing.

The Actor can meet the QA criteria: full local execution produced real results in seconds, and the transient-error recovery budget fits under five minutes. After deployment and hosted validation, this specific failure is no longer expected on an isolated 503. Persistent upstream unavailability can still cause a legitimate QA failure. An automated-testing exemption is not warranted by the evidence.

[Apify's testing documentation](https://docs.apify.com/actors/publishing/test) requires SUCCEEDED plus a non-empty default dataset within five minutes. It says rebuilding a fixed Actor should be picked up within 24 hours; recovery from target-site issues may instead depend on a majority of successful tests over seven days.
