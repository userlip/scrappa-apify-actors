# YouTube Search Data Scraper QA recovery — 2026-09-26

Actor: `thescrappa/youtube-api-search-data` (`ziD2fUoLsdzKlc6zR`)

## Root cause

Apify automated QA run [`t4rIdRCGHa9V6y0tH`](https://console.apify.com/view/runs/t4rIdRCGHa9V6y0tH) failed on 2026-09-26 at 15:14 UTC on build `0.0.14` (`aWqrPw0Y43NuShdyU`). It was **not** a five-minute timeout: origin `TEST`, 128 MB, 300-second timeout, runtime **13.1 seconds**, exit code 1, empty dataset, 0 charged items.

Stored INPUT was the schema prefill:

```json
{"q":"Javascript tutorial","sort":["relevance"],"duration":["short"],"upload_date":["hour"],"type":["all"],"limit":10}
```

Log:

```
Fetching from: https://scrappa.co/api/youtube/search?query=Javascript+tutorial&order=relevance&videoDuration=short&publishedAfter=2026-09-26T14%3A14%3A58.054Z&type=all&limit=10
Failed to fetch YouTube search data: Scrappa API request failed with 503 Service Unavailable
```

The 503 arrived about 12.3 seconds after the request. Scrappa's `YouTubeExternalController` returns `503 youtube_upstream_unavailable` when its call to the internal YouTube service throws a network error or times out. That upstream timeout is `services.youtube_external.timeout`, which defaults to **12s**. So the upstream search was slow once and Scrappa correctly reported a transient failure. The actor treated every non-2xx response as fatal and had no retry, so one upstream blip failed the whole QA run.

The prefilled input itself is valid. The same query, filters and one-hour `publishedAfter` succeeded against Scrappa during this investigation and returned 10 results. Scrappa maps a one-hour window to YouTube's `today` bucket, so the "hour" prefill does not produce an empty result set. No schema change was needed.

Only the run linked in the notification could be inspected. QA runs belong to Apify's test user and do not appear in the organization's run list, so the other two daily failures were not examined individually. No Sentry issue matched this endpoint.

## Repair

- `src/main.rs`: the Scrappa search call now retries transient failures (network errors, request or body timeouts, HTTP 408/429/500/502/503/504) up to **3 attempts** with 2s then 4s backoff. Other 4xx responses and invalid JSON still fail immediately. Worst case is 3 × 60s + 6s = 186s, which stays inside the 300s QA window. Retrying is safe because the call is an idempotent GET and failed Scrappa requests do not consume credits.
- `Cargo.toml`: enables tokio's `time` feature explicitly for `tokio::time::sleep` instead of relying on reqwest to turn it on.
- Automated testing was not disabled. Apify support was not contacted.

## Validation

- `cargo test --locked` (rust:1.90 container): **20 passed, 0 failed**, no warnings. Three new tests cover: a 503 followed by success is retried and saves rows; persistent 503s fail after 3 attempts; a 400 is not retried.
- The production `.actor/Dockerfile` image built locally.
- Before deploying, the live version `0.0` `SOURCE_FILES` matched git `HEAD` byte for byte. Only `Cargo.toml` and `src/main.rs` changed, and the `SCRAPPA_API_KEY` secret env var was preserved.

## Deployed validation — 2026-09-26

- Candidate build `0.0.15` (`UrnMZhKiAJhwuMPvT`) succeeded.
- QA-style runs on `0.0.15` used the exact failed-run INPUT, 128 MB and a 300s timeout:
  - [`6KZ5j4zEN1S9R5DLE`](https://console.apify.com/view/runs/6KZ5j4zEN1S9R5DLE): `SUCCEEDED` in 1.57s, 10 dataset items, 10 `apify-default-dataset-item` charges.
  - [`XgvGNVc6KMZ0zjcBm`](https://console.apify.com/view/runs/XgvGNVc6KMZ0zjcBm) (`maxTotalChargeUsd` 999999999, as in QA): `SUCCEEDED` in 1.58s, 10 items, 10 charges.
- `latest` was promoted to `0.0.15` and the temporary `candidate` tag removed. Previous `latest` `0.0.14` (`aWqrPw0Y43NuShdyU`) is kept for rollback.
- Post-promotion run on the default build, [`T7c7XmuGsoLQyCEHe`](https://console.apify.com/view/runs/T7c7XmuGsoLQyCEHe): `SUCCEEDED` on `0.0.15` in 1.37s, 10 items.
- Actor notice set to `NONE`. The actor is still public and not deprecated.

Apify's daily automated QA runs independently of these org-owned runs and should pick up build `0.0.15` on its next cycle: https://docs.apify.com/platform/actors/publishing/test
