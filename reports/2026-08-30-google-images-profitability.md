# Google Images Actor profitability recovery

Checked and deployed on 2026-08-30 for public Actor `thescrappa/google-images-scraper` (`MrbqFgdpNTQcRW0Vt`). The triggering Apify alert reported 2026-08-14 revenue of `$0.22`, platform cost of `$1.54`, profit of `-$1.32`, and margin of `-585.39%`.

## Cost drivers

- The production build active on August 14 (`1.0.1`) required one `q` per run. It could not amortize Actor startup, input storage, and failed-request compute across queries.
- The Actor has unusually high run amplification. On August 30, its rolling 30-day public stats contained `127,717` runs: `106,808` succeeded, `13,737` failed, `7,084` timed out, and `88` were aborted. Only 13 users had ever used the Actor. The failures and timeouts consume platform resources without necessarily producing billable dataset items.
- Active pricing charges only for default-dataset items: `$0.00030` on FREE, `$0.00025` on BRONZE, `$0.00022` on SILVER, and `$0.00020` on GOLD and above. Failed, timed-out, and valid zero-result runs therefore had no revenue on August 14.
- A synthetic `$0.00005` `apify-actor-start` event is already scheduled for `2026-09-05T07:50:29.852Z`. Apify does not allow a pending significant pricing change to be replaced, so the immediate fix must contain resource use. The scheduled event will also make Apify cover the first five seconds of compute for each run.
- A successful post-fix 100-result smoke run showed platform cost composition of `$0.000500` dataset writes, `$0.000100` key-value writes, `$0.000065` compute, `$0.000005` key-value reads, and about `$0.000002` transfer. Dataset output and the documented `OUTPUT` record are expected functionality, so neither was removed.

## Fix

Batch input added after the alert remains supported for up to 50 deduplicated queries with five concurrent Scrappa requests. Legacy single-query `q` input remains compatible.

The deployed request policy now limits Scrappa work to two 30-second attempts and retries only transient connection, timeout, HTTP 408/429, and HTTP 5xx failures. Invalid 4xx requests fail immediately. Compared with the briefly deployed three-attempt, 60-second policy, the maximum work for one failing request falls from 180 seconds plus backoff to 60 seconds plus one backoff, a 66.7% reduction. The 50-query worst-case schedule remains bounded by the Actor's 720-second timeout and 128 MB memory limit.

The live Actor's stored default options were also corrected from 3,600 seconds to 720 seconds, with memory retained at 128 MB.

## Verification

- `npm test`: 14/14 tests passed, including transient retry, non-retryable 400, batch input, validation, and response-shape coverage.
- `apify validate-schema .actor/input_schema.json`: passed.
- Deployment: final build `1.0.5` (`LJwRpNUEYupnGjSJj`) succeeded and received the `latest` tag.
- Production smoke: run `4Qg1XCXgGELlcNe06` succeeded on build `1.0.5` in `9.193s`, used 128 MB with `52,031,488` peak bytes, stored 100 clean dataset rows, and recorded 100 `apify-default-dataset-item` charges.
- Finalized smoke platform cost: `$0.0006758049`.
- Lowest-tier event revenue check: at the `$0.00020` GOLD-and-above result price, 100 rows produce `$0.020`; the developer share before cost is `$0.016`, leaving approximately `$0.015324` profit and a `76.62%` margin on gross revenue for the representative run. At the FREE-tier `$0.00030` price, the same run leaves approximately `$0.023324` profit and a `77.75%` margin.

The smoke proves profitable successful execution. The bounded retry policy and the September 5 synthetic start event address the unbilled failure/startup path that drove the August 14 loss. Profitability should be rechecked after the start event activates because Apify does not expose renter-owned individual run records to the Actor owner through the normal run-list API.
