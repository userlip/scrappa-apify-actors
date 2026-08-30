# Google Trends Related Queries QA recovery

Apify automated QA run `do3gFalO7vWaRyNrT` failed on build `VGqaWARXwacoY3RdM` after the prefilled request received three Scrappa HTTP 503 responses. The Actor retried after roughly 2.9 and 4.5 seconds, ignoring the upstream recovery window, and exited after 31 seconds without dataset rows.

The deployed client now preserves Scrappa response status and `Retry-After` metadata. The primary request makes four attempts, caps each retry delay at 20 seconds, and uses a 30-second request timeout. Optional autocomplete gets one non-fatal 15-second attempt. With a 30-second completion reserve, the combined worst-case budget is 225 seconds inside the Actor's 240-second timeout and Apify's five-minute QA limit.

## Verification

- Local Actor suite: 24 tests passed.
- Prefilled Scrappa request (`coffee`, `US`, `1y`, `en`, `web`) returned 25 related queries in each of four checks during the investigation.
- Apify build `BRezbJsyoWX9pOFdN` (`1.0.4`) succeeded on 2026-08-30.
- Cloud run `mDqDKQUuKj7lF4jS4` used the exact published prefilled input and build `1.0.4`.
- The run succeeded from `2026-08-30T13:37:26.360Z` to `2026-08-30T13:37:32.722Z` (6.362 seconds).
- Default dataset `EPgY411FBYTJ1PeK2` contains 25 related-query rows.
- `OUTPUT` reports `related_query_count: 25`, `related_topic_count: 0`, and Scrappa `response_time_ms: 2590`.
- Actor default run options are `latest`, 128 MB, and 240 seconds.
- Actor maintenance notice is `NONE` after deployment and the successful smoke run.
