# Arbeitsagentur Jobs QA recovery

Apify automated QA run `61vr8IobHW8Kg7CBH` failed on build `I8eBG6yX12DQCPLtC` after the exact prefilled request received three Scrappa HTTP 503 responses. The Actor retried after roughly 2.9 and 4.6 seconds, then failed in 14.476 seconds with no dataset rows. The input was valid and the run did not exceed Apify's five-minute limit; the failure was an exhausted transient-service retry window.

The deployed client now preserves Scrappa response status and `Retry-After` metadata. The jobs request makes four attempts, caps each retry delay at 20 seconds, and uses a 30-second request timeout. Its 180-second maximum request budget plus a 30-second completion reserve fits inside the Actor's 240-second timeout and Apify's five-minute automated-QA limit.

## Verification

- The exact prefilled Scrappa request (`Software Entwickler`, `Berlin`, `vz;ho`, radius `25`, page `1`, size `25`) returned 25 jobs during investigation.
- Local Actor suite: 20 tests passed, including the combined runtime budget and the actual capped `Retry-After` scheduling path.
- `npx apify-cli validate-schema` passed for the input schema and embedded dataset schema.
- The exact-prefill local Actor run completed successfully with 25 job rows.
- Apify build `RAxJMTaA54PEEmv2Q` (`1.0.6`) succeeded on 2026-08-30.
- Final cloud run `SjEyjWBSCiaqEkZeR` used build `1.0.6` and the exact published prefilled input.
- The final run succeeded from `2026-08-30T14:26:47.132Z` to `2026-08-30T14:26:57.045Z` (9.747 seconds).
- Default dataset `OUIczhx1gHzjLpWj3` contains 25 clean job rows, matching 25 charged `apify-default-dataset-item` events.
- Exact-prefill `OUTPUT` reports `success: true`, `maxErgebnisse: 240`, page `1`, size `25`, and 25 `stellenangebote`.
- Actor defaults are `latest`, 128 MB, and 240 seconds.
- Actor maintenance notice is `NONE` after deployment and the successful smoke run.

Automatic-testing opt-out was not configured because the Actor meets Apify's test criteria.
