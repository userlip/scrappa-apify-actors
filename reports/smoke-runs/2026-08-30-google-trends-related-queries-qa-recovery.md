# Google Trends Related Queries QA recovery

Apify automated QA run `do3gFalO7vWaRyNrT` failed on build `VGqaWARXwacoY3RdM` after the prefilled request received three Scrappa HTTP 503 responses. The Actor retried after roughly 2.9 and 4.5 seconds, ignoring the upstream recovery window, and exited after 31 seconds without dataset rows.

The deployed client now preserves Scrappa response status and `Retry-After` metadata. The primary request makes four attempts, caps each retry delay at 20 seconds, and uses a 30-second request timeout. Optional autocomplete gets one non-fatal 15-second attempt. With a 30-second completion reserve, the combined worst-case budget is 225 seconds inside the Actor's 240-second timeout and Apify's five-minute QA limit.

## Verification

- Prefilled Scrappa request (`coffee`, `US`, `1y`, `en`, `web`) returned 25 related queries in each of four checks during the investigation.
- Local Actor suite: 26 tests passed after review follow-ups, including the combined runtime budget and actual `Retry-After` scheduling path.
- Final Apify build `pkeoRQeZ6bZLuYVPB` (`1.0.6`) succeeded on 2026-08-30. Build `1.0.5` had failed before compilation on a transient `only-allow: Text file busy` install error and was replaced by the successful rebuild.
- Cloud run `xkuztmDi18JroBxwO` used the exact published prefilled input and build `1.0.6`.
- The exact-prefill run succeeded from `2026-08-30T13:45:16.898Z` to `2026-08-30T13:45:21.077Z` (4.179 seconds).
- Default dataset `fUo1g6IoNLHk409fh` contains 25 related-query rows.
- Exact-prefill `OUTPUT` reports `related_query_count: 25`, `related_topic_count: 0`, and Scrappa `response_time_ms: 1618`.
- Autocomplete-enabled smoke run `GAEO5De2WIcEBckmb` also succeeded in 6.658 seconds with 25 dataset rows and a non-null autocomplete response, proving the optional path fits the same Actor timeout.
- Actor default run options are `latest`, 128 MB, and 240 seconds.
- Actor maintenance notice is `NONE` after deployment and the successful smoke run.
