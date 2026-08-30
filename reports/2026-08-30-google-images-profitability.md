# Google Images Actor profitability recovery

Checked and deployed on 2026-08-30 for public Actor `thescrappa/google-images-scraper` (`MrbqFgdpNTQcRW0Vt`). Two adjacent Apify alerts cover the same production build and cost pattern:

- 2026-08-13: `$1.39` revenue, `$3.33` platform cost, `-$1.94` profit, and `-140.13%` margin.
- 2026-08-14: `$0.22` revenue, `$1.54` platform cost, `-$1.32` profit, and `-585.39%` margin.

The August 13 figures imply that, without implementation changes, revenue or result prices would have needed to increase by about `2.40x` merely to equal that day's platform cost. The recovery therefore prioritizes reducing unbilled work and amortizing fixed run overhead before considering a marketplace price increase.

## Cost drivers

- The production build active on August 13 and 14 (`1.0.1`, build `psT5nsFjo6WkUpw3q`) required one `q` per run. It could not amortize Actor startup, input storage, and failed-request compute across queries. The archived build definition confirms 128 MB memory, a 120-second packaged default timeout, and no batch input; the live Actor's separately stored default timeout was later found set to 3,600 seconds.
- The Actor has unusually high run amplification. On August 30, its rolling 30-day public stats contained `127,717` runs: `106,808` succeeded, `13,737` failed, `7,084` timed out, and `88` were aborted. Only 13 users had ever used the Actor. The failures and timeouts consume platform resources without necessarily producing billable dataset items.
- Active pricing charges only for default-dataset items: `$0.00030` on FREE, `$0.00025` on BRONZE, `$0.00022` on SILVER, and `$0.00020` on GOLD and above. Failed, timed-out, and valid zero-result runs therefore had no revenue on August 14.
- A synthetic `$0.00005` `apify-actor-start` event is already scheduled for `2026-09-05T07:50:29.852Z`. Apify does not allow a pending significant pricing change to be replaced, so the immediate fix must contain resource use. The scheduled event will also make Apify cover the first five seconds of compute for each run.
- Apify does not expose renter-owned run records to the Actor owner through the normal run-list API. Consequently the alert's daily totals cannot be joined to individual August 13 runs. The attribution is based on the production build, live 30-day status counts, current pricing, and owner-run cost measurements rather than an unavailable per-run cost export.
- A successful post-fix 100-result smoke run showed platform cost composition of `$0.000500` dataset writes (74.4%), `$0.000100` key-value writes (14.9%), `$0.000064` compute (9.5%), `$0.000005` key-value reads (0.7%), and about `$0.000003` transfer (0.4%). Dataset writes are also the billable result event and remain strongly profitable. The documented `OUTPUT` record accounts for one of the two key-value writes and is retained for compatibility.

## Fix

Batch input added after the alerts remains supported for up to 50 deduplicated queries with five concurrent Scrappa requests. Legacy single-query `q` input remains compatible. The Store README and input schema now lead with batching so users can discover the lower-cost path without breaking existing integrations.

The deployed request policy now limits Scrappa work to two 30-second attempts and retries only transient connection, timeout, HTTP 408/429, and HTTP 5xx failures. Invalid 4xx requests fail immediately. Compared with the briefly deployed three-attempt, 60-second policy, the maximum work for one failing request falls from 180 seconds plus backoff to 60 seconds plus one backoff, a 66.7% reduction. Compared with a run reaching the previously stored 3,600-second timeout, the request bound reduces the single-request failure path by about 98.3%. The 50-query worst-case schedule remains bounded by the Actor's 720-second timeout and 128 MB memory limit.

The live Actor's stored default options were also corrected from 3,600 seconds to 720 seconds, with memory retained at 128 MB.

## Verification

- `npm test`: 14/14 tests passed again on 2026-08-30, including transient retry, non-retryable 400, batch input, validation, and response-shape coverage.
- `apify validate-schema .actor/input_schema.json`: passed again on 2026-08-30.
- Runtime deployment: build `1.0.5` (`LJwRpNUEYupnGjSJj`) succeeded and passed the production smoke. Documentation-only build `1.0.7` (`c9efK7gITR1b5dFjd`) was deployed from pushed commit `e9a1138` and received the `latest` tag after the Store README was updated to lead with batch input; its cached container image is unchanged from the already verified recovery build.
- Production smoke: run `4Qg1XCXgGELlcNe06` succeeded on build `1.0.5` in `9.193s`, used 128 MB with `52,031,488` peak bytes, stored 100 clean dataset rows, and recorded 100 `apify-default-dataset-item` charges.
- Finalized smoke platform cost re-read from the authenticated run API: `$0.0006716578` (`$0.000500` dataset, `$0.000100` key-value writes, `$0.0000638403` compute, `$0.000005` key-value read, and `$0.0000028175` transfer).
- Lowest-tier event revenue check: at the `$0.00020` GOLD-and-above result price, 100 rows produce `$0.020`; the developer share before cost is `$0.016`, leaving approximately `$0.015328` profit and a `76.64%` margin on gross revenue for the representative run. At the FREE-tier `$0.00030` price, the same run leaves approximately `$0.023328` profit and a `77.76%` margin.

The smoke proves profitable successful execution. The bounded retry policy and the September 5 synthetic start event address the unbilled failure/startup path that drove both alerts.

## Human decisions and follow-up

- No immediate price increase was applied. Apify already has a significant pricing change pending, and does not allow it to be replaced before activation. Raising result prices before measuring the cost controls would also charge successful users for failure-heavy usage they did not cause.
- Recheck daily revenue, cost, failed/timed-out run counts, and average results per run after the `apify-actor-start` event activates on 2026-09-05. Use at least seven complete days to avoid reacting to this Actor's low user count and bursty automation.
- If costs still exceed revenue after that window, a human should choose between increasing the start event (targets fixed/failure cost) or increasing tiered result prices (targets all successful usage). As a historical upper bound only, reproducing the August 13 mix would require about `2.40x` revenue to break even: approximately FREE `$0.00072`, BRONZE `$0.00060`, SILVER `$0.000528`, and GOLD+ `$0.00048` per result. Demand elasticity and competitor pricing were not available, so these are break-even calculations, not recommended prices.
